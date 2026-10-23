//! HardwareMonitor: live system metrics for the widget (see 11-widget-e-hardware.md).
//!
//! Everything here works WITHOUT administrator rights (Windows performance data through
//! `sysinfo`, plus `GetSystemPowerStatus` for the battery). Temperatures, fan speeds, SMART
//! and GPU load need a privileged sensor helper (LibreHardwareMonitorLib, MPL-2.0); that
//! helper comes later, so `sensors.available` is false and the UI explains why.
//!
//! The sampler is pull-based: nothing is measured unless the UI asks, so a hidden widget
//! costs no CPU. No data ever leaves the computer.

use serde::Serialize;
use sysinfo::{Disks, Networks, System};

#[derive(Debug, Clone, Serialize)]
pub struct Cpu {
    pub model: String,
    pub usage: f32,
    pub per_core: Vec<f32>,
    pub clock_mhz: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Ram {
    pub used: u64,
    pub total: u64,
    pub percent: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskInfo {
    pub name: String,
    pub mount: String,
    pub used: u64,
    pub total: u64,
    pub read_bps: u64,
    pub write_bps: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Net {
    pub adapter: String,
    pub ip: Option<String>,
    pub down_bps: u64,
    pub up_bps: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Battery {
    pub percent: u8,
    pub charging: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemStat {
    pub uptime_s: u64,
    pub os: String,
    pub battery: Option<Battery>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Sensors {
    /// Temperatures, fan speeds, SMART and GPU load. Needs the admin helper.
    pub available: bool,
    pub reason: &'static str,
    pub cpu_temp_c: Option<f32>,
    pub gpu_temp_c: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub cpu: Cpu,
    pub ram: Ram,
    pub disks: Vec<DiskInfo>,
    pub net: Net,
    pub system: SystemStat,
    pub gpus: Vec<String>,
    pub sensors: Sensors,
    /// Seconds between the two last samples (rates are bytes per second over this span).
    pub interval_s: f32,
}

pub struct Sampler {
    sys: System,
    disks: Disks,
    nets: Networks,
    last: std::time::Instant,
}

impl Sampler {
    pub fn new() -> Self {
        let mut sys = System::new();
        // CPU usage needs two readings; take the first one now.
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        Self {
            sys,
            disks: Disks::new_with_refreshed_list(),
            nets: Networks::new_with_refreshed_list(),
            last: std::time::Instant::now(),
        }
    }

    pub fn sample(&mut self) -> Snapshot {
        let now = std::time::Instant::now();
        let dt = now.duration_since(self.last).as_secs_f32().max(0.05);
        self.last = now;

        self.sys.refresh_cpu_usage();
        self.sys.refresh_cpu_frequency();
        self.sys.refresh_memory();
        self.disks.refresh(true);
        self.nets.refresh(true);

        let cpus = self.sys.cpus();
        let cpu = Cpu {
            model: cpus.first().map(|c| c.brand().trim().to_string()).unwrap_or_default(),
            usage: self.sys.global_cpu_usage(),
            per_core: cpus.iter().map(|c| c.cpu_usage()).collect(),
            clock_mhz: cpus.first().map(|c| c.frequency()).unwrap_or(0),
        };

        let (used, total) = (self.sys.used_memory(), self.sys.total_memory());
        let ram = Ram { used, total, percent: if total == 0 { 0.0 } else { used as f32 * 100.0 / total as f32 } };

        let per_sec = |bytes: u64| (bytes as f32 / dt) as u64;
        let disks = self
            .disks
            .list()
            .iter()
            .map(|d| {
                let u = d.usage();
                DiskInfo {
                    name: d.name().to_string_lossy().into_owned(),
                    mount: d.mount_point().to_string_lossy().into_owned(),
                    used: d.total_space().saturating_sub(d.available_space()),
                    total: d.total_space(),
                    read_bps: per_sec(u.read_bytes),
                    write_bps: per_sec(u.written_bytes),
                }
            })
            .collect();

        Snapshot {
            cpu,
            ram,
            disks,
            net: self.net(dt),
            system: SystemStat { uptime_s: System::uptime(), os: System::long_os_version().unwrap_or_default(), battery: battery() },
            gpus: gpu_names(),
            sensors: Sensors { available: false, reason: "needs_admin_helper", cpu_temp_c: None, gpu_temp_c: None },
            interval_s: dt,
        }
    }

    /// The "active" adapter is the one that has moved the most data (loopback excluded).
    fn net(&self, dt: f32) -> Net {
        let best = self
            .nets
            .iter()
            .filter(|(name, _)| !name.to_lowercase().contains("loopback"))
            .max_by_key(|(_, d)| d.total_received() + d.total_transmitted());
        match best {
            Some((name, d)) => Net {
                adapter: name.clone(),
                ip: d.ip_networks().iter().map(|n| n.addr).find(|a| a.is_ipv4() && !a.is_loopback()).map(|a| a.to_string()),
                down_bps: (d.received() as f32 / dt) as u64,
                up_bps: (d.transmitted() as f32 / dt) as u64,
            },
            None => Net { adapter: String::new(), ip: None, down_bps: 0, up_bps: 0 },
        }
    }
}

/// Battery state, `None` on desktops.
fn battery() -> Option<Battery> {
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut s = SYSTEM_POWER_STATUS { ACLineStatus: 0, BatteryFlag: 0, BatteryLifePercent: 0, SystemStatusFlag: 0, BatteryLifeTime: 0, BatteryFullLifeTime: 0 };
    // SAFETY: `s` is a valid, writable SYSTEM_POWER_STATUS for the duration of the call.
    let ok = unsafe { GetSystemPowerStatus(&mut s) } != 0;
    if !ok || s.BatteryFlag == 128 || s.BatteryLifePercent > 100 {
        return None; // 128 = no battery, 255 = unknown
    }
    Some(Battery { percent: s.BatteryLifePercent, charging: s.ACLineStatus == 1 })
}

/// GPU names do not change while the app runs: ask Windows once, in the background.
fn gpu_names() -> Vec<String> {
    use std::sync::{Mutex, OnceLock};
    static NAMES: OnceLock<Mutex<Option<Vec<String>>>> = OnceLock::new();
    let cell = NAMES.get_or_init(|| {
        std::thread::spawn(|| {
            let found = query_gpus();
            *NAMES.get().unwrap().lock().unwrap() = Some(found);
        });
        Mutex::new(None)
    });
    cell.lock().unwrap().clone().unwrap_or_default()
}

fn query_gpus() -> Vec<String> {
    use std::os::windows::process::CommandExt;
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", "(Get-CimInstance Win32_VideoController).Name -join '|'"])
        .creation_flags(0x0800_0000)
        .output();
    match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().split('|').filter(|s| !s.is_empty()).map(str::to_string).collect(),
        Err(_) => vec![],
    }
}
