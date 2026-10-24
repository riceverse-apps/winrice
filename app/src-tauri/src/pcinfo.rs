//! "My PC": read-only hardware and driver facts for the widget. It never installs or changes a
//! driver; it only reads what Windows already knows (CIM/WMI through PowerShell) and flags old
//! graphics drivers so the user can go to Windows Update or the maker's site.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Gpu {
    pub name: String,
    pub driver_version: String,
    /// ISO date (YYYY-MM-DD) of the installed driver, when Windows reports it.
    pub driver_date: Option<String>,
    /// Driver older than two years.
    pub driver_old: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PcInfo {
    pub maker: String,
    pub model: String,
    pub cpu: String,
    pub board: String,
    pub bios: String,
    pub windows: String,
    pub gpus: Vec<Gpu>,
}

const SCRIPT: &str = r#"$ErrorActionPreference='SilentlyContinue'
$cs=Get-CimInstance Win32_ComputerSystem; $bios=Get-CimInstance Win32_BIOS; $bb=Get-CimInstance Win32_BaseBoard
$cpu=Get-CimInstance Win32_Processor | Select-Object -First 1; $os=Get-CimInstance Win32_OperatingSystem
$gpus=@(Get-CimInstance Win32_VideoController | ForEach-Object { @{ name=$_.Name; version=$_.DriverVersion; date=$(if($_.DriverDate){$_.DriverDate.ToString('yyyy-MM-dd')}else{''}) } })
@{ maker=$cs.Manufacturer; model=$cs.Model; cpu=$cpu.Name; board=("$($bb.Manufacturer) $($bb.Product)").Trim(); bios=("$($bios.SMBIOSBIOSVersion) ($($bios.ReleaseDate.ToString('yyyy-MM-dd')))").Trim(); windows="$($os.Caption) $($os.Version)"; gpus=$gpus } | ConvertTo-Json -Compress -Depth 4"#;

#[derive(Deserialize)]
struct RawGpu {
    name: Option<String>,
    version: Option<String>,
    date: Option<String>,
}

#[derive(Deserialize)]
struct Raw {
    maker: Option<String>,
    model: Option<String>,
    cpu: Option<String>,
    board: Option<String>,
    bios: Option<String>,
    windows: Option<String>,
    #[serde(default, deserialize_with = "one_or_many")]
    gpus: Vec<RawGpu>,
}

/// PowerShell turns a one-element array into a bare object.
fn one_or_many<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<RawGpu>, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    let items = match v {
        serde_json::Value::Array(a) => a,
        serde_json::Value::Null => vec![],
        other => vec![other],
    };
    Ok(items.into_iter().filter_map(|i| serde_json::from_value(i).ok()).collect())
}

/// `today` as days since 1970-01-01 is passed in so the parsing stays testable.
pub fn parse(json: &str, today_days: i64) -> Option<PcInfo> {
    let raw: Raw = serde_json::from_str(json.trim().trim_start_matches('\u{feff}')).ok()?;
    let s = |o: Option<String>| o.unwrap_or_default().trim().to_string();
    Some(PcInfo {
        maker: s(raw.maker),
        model: s(raw.model),
        cpu: s(raw.cpu),
        board: s(raw.board),
        bios: s(raw.bios),
        windows: s(raw.windows),
        gpus: raw
            .gpus
            .into_iter()
            .map(|g| {
                let date = g.date.map(|d| d.trim().to_string()).filter(|d| !d.is_empty());
                let old = date.as_deref().and_then(days_from_iso).is_some_and(|d| today_days - d > 730);
                Gpu { name: s(g.name), driver_version: s(g.version), driver_date: date, driver_old: old }
            })
            .collect(),
    })
}

/// Days since 1970-01-01 for "YYYY-MM-DD" (civil-from-days algorithm).
pub fn days_from_iso(d: &str) -> Option<i64> {
    let mut it = d.split('-');
    let (y, m, day): (i64, i64, i64) = (it.next()?.parse().ok()?, it.next()?.parse().ok()?, it.next()?.parse().ok()?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&day) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

fn encode(script: &str) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes: Vec<u8> = script.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    let mut out = String::new();
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

pub fn read() -> Option<PcInfo> {
    // -EncodedCommand (UTF-16LE, base64) keeps the quotes of the script intact on the command line.
    let out = std::process::Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-EncodedCommand", &encode(SCRIPT)]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let today = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64 / 86_400;
    parse(&String::from_utf8_lossy(&out.stdout), today)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_of_utf16_matches_powershell() {
        assert_eq!(encode("hi"), "aABpAA==");
    }

    #[test]
    fn iso_dates_become_day_numbers() {
        assert_eq!(days_from_iso("1970-01-01"), Some(0));
        assert_eq!(days_from_iso("2000-03-01"), Some(11_017));
        assert_eq!(days_from_iso("nonsense"), None);
        assert_eq!(days_from_iso("2020-13-01"), None);
    }

    #[test]
    fn parses_the_powershell_output_and_flags_old_graphics_drivers() {
        let today = days_from_iso("2026-10-24").unwrap();
        let json = r#"{"maker":"ACME ","model":"Laptop 5","cpu":"Fast CPU","board":"ACME B1","bios":"1.2 (2024-01-01)","windows":"Windows 11 Pro 10.0.26100",
            "gpus":[{"name":"Old GPU","version":"1.0","date":"2020-05-01"},{"name":"New GPU","version":"2.0","date":"2026-08-01"},{"name":"No date","version":"","date":""}]}"#;
        let p = parse(json, today).unwrap();
        assert_eq!(p.maker, "ACME");
        assert_eq!(p.gpus.len(), 3);
        assert!(p.gpus[0].driver_old);
        assert!(!p.gpus[1].driver_old);
        assert!(!p.gpus[2].driver_old && p.gpus[2].driver_date.is_none());
    }

    #[test]
    fn a_single_gpu_comes_as_a_bare_object() {
        let p = parse(r#"{"model":"X","gpus":{"name":"Only","version":"1","date":"2026-01-01"}}"#, 20_000).unwrap();
        assert_eq!(p.gpus.len(), 1);
        assert_eq!(p.gpus[0].name, "Only");
        assert!(parse("not json", 0).is_none());
    }
}
