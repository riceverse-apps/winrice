//! Weak-PC optimizer: apply, restore, rollback and suggestions.

use crate::hardware::{Battery, Cpu, DiskInfo, Net, Ram, Sensors, Snapshot, SystemStat};
use crate::optimize::{self, parse_reg_query, TWEAKS};
use crate::os::fake::{FakeOs, World};
use crate::os::Os;

fn ids(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

fn all_recommended() -> Vec<String> {
    TWEAKS.iter().filter(|t| t.recommended).map(|t| t.id.to_string()).collect()
}

#[test]
fn apply_then_restore_leaves_the_machine_as_it_was() {
    let mut w = World::default();
    w.registry.insert(r"Control Panel\Desktop|MenuShowDelay".into(), ("REG_SZ".into(), "400".into()));
    let os = FakeOs::new(w);
    let before = os.machine();
    assert_eq!(optimize::apply(&os, &all_recommended()).unwrap(), all_recommended().len());
    assert!(optimize::status(&os).iter().filter(|s| s.recommended).all(|s| s.optimized && s.restorable));
    assert_eq!(os.reg_get(r"Control Panel\Desktop", "MenuShowDelay"), Some(("REG_SZ".into(), "0".into())));
    optimize::restore(&os, &[]).unwrap();
    assert_eq!(os.machine(), before, "restore must bring back values and delete the ones that did not exist");
    assert!(optimize::status(&os).iter().all(|s| !s.optimized && !s.restorable));
}

#[test]
fn applying_twice_keeps_the_first_original() {
    let mut w = World::default();
    w.registry.insert(r"Control Panel\Desktop|MenuShowDelay".into(), ("REG_SZ".into(), "400".into()));
    let os = FakeOs::new(w);
    optimize::apply(&os, &ids(&["menu-delay"])).unwrap();
    optimize::apply(&os, &ids(&["menu-delay"])).unwrap();
    optimize::restore(&os, &ids(&["menu-delay"])).unwrap();
    assert_eq!(os.reg_get(r"Control Panel\Desktop", "MenuShowDelay"), Some(("REG_SZ".into(), "400".into())));
}

#[test]
fn restoring_one_tweak_leaves_the_others() {
    let os = FakeOs::new(World::default());
    optimize::apply(&os, &ids(&["transparency", "menu-delay"])).unwrap();
    optimize::restore(&os, &ids(&["transparency"])).unwrap();
    let st = optimize::status(&os);
    assert!(!st.iter().find(|s| s.id == "transparency").unwrap().optimized);
    assert!(st.iter().find(|s| s.id == "menu-delay").unwrap().optimized);
}

#[test]
fn a_failure_in_the_middle_restores_what_was_changed() {
    let os = FakeOs::new(World::default());
    let before = os.machine();
    *os.fail_at.lock().unwrap() = Some(3);
    assert!(optimize::apply(&os, &all_recommended()).is_err());
    assert_eq!(os.machine(), before);
    assert!(optimize::status(&os).iter().all(|s| !s.restorable));
}

#[test]
fn unknown_tweak_is_refused_before_anything_changes() {
    let os = FakeOs::new(World::default());
    let before = os.machine();
    assert_eq!(optimize::apply(&os, &ids(&["transparency", "nope"])).unwrap_err(), "unknown_tweak:nope");
    assert_eq!(os.machine(), before);
}

#[test]
fn tweaks_are_hkcu_only_and_unique() {
    let mut seen = std::collections::BTreeSet::new();
    for t in TWEAKS {
        assert!(seen.insert(t.id), "duplicate tweak id {}", t.id);
        for v in t.values {
            assert!(!v.key.to_uppercase().starts_with("HK"), "{} must not name a hive", v.key);
            assert!(v.kind == "REG_SZ" || v.kind == "REG_DWORD");
        }
    }
}

#[test]
fn reg_query_output_is_parsed() {
    let sz = "\r\nHKEY_CURRENT_USER\\Control Panel\\Desktop\r\n    MenuShowDelay    REG_SZ    400\r\n\r\n";
    assert_eq!(parse_reg_query(sz, "MenuShowDelay"), Some(("REG_SZ".into(), "400".into())));
    let dw = "HKEY_CURRENT_USER\\X\n   EnableTransparency    REG_DWORD    0x1\n";
    assert_eq!(parse_reg_query(dw, "EnableTransparency"), Some(("REG_DWORD".into(), "1".into())));
    assert_eq!(parse_reg_query(dw, "Other"), None);
}

fn snap(ram_gb: u64, ram_pct: f32, free_pct: u64, uptime_days: u64) -> Snapshot {
    const GB: u64 = 1024 * 1024 * 1024;
    Snapshot {
        cpu: Cpu { model: String::new(), usage: 10.0, per_core: vec![], clock_mhz: 0 },
        ram: Ram { used: 0, total: ram_gb * GB, percent: ram_pct },
        disks: vec![DiskInfo { name: "C".into(), mount: "C:\\".into(), used: (100 - free_pct) * GB, total: 100 * GB, read_bps: 0, write_bps: 0 }],
        net: Net { adapter: String::new(), ip: None, down_bps: 0, up_bps: 0 },
        system: SystemStat { uptime_s: uptime_days * 24 * 3600, os: String::new(), battery: None::<Battery> },
        gpus: vec![],
        sensors: Sensors { available: false, reason: "", cpu_temp_c: None, gpu_temp_c: None },
        interval_s: 1.0,
    }
}

fn has(v: &[optimize::Suggestion], id: &str) -> bool {
    v.iter().any(|s| s.id == id)
}

#[test]
fn suggestions_follow_the_hardware() {
    let weak = optimize::suggest(&snap(4, 90.0, 10, 10));
    for id in ["ram_low", "ram_busy", "disk_low", "restart", "heavy_tools", "startup_apps"] {
        assert!(has(&weak, id), "{id}");
    }
    let strong = optimize::suggest(&snap(32, 30.0, 60, 1));
    for id in ["ram_low", "ram_ok", "ram_busy", "disk_low", "restart", "heavy_tools"] {
        assert!(!has(&strong, id), "{id}");
    }
    assert!(has(&strong, "power_plan"));
    assert!(has(&optimize::suggest(&snap(8, 50.0, 50, 1)), "ram_ok"));
}
