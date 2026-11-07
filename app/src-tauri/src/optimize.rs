//! "Faster on a weak PC": a short, curated list of safe tweaks plus hardware-based suggestions.
//!
//! Every tweak only changes values under the current user's registry (HKCU): no administrator
//! permission, no services, no system files. Before changing anything the previous value of every
//! key is saved in `optimize-backup.json`, so each tweak (or all of them) can be put back exactly
//! as it was. If a write fails halfway, what was already changed is restored right away.

use crate::hardware::Snapshot;
use crate::os::Os;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub struct Value {
    /// Key path under HKCU (no hive prefix, so a tweak can never touch HKLM).
    pub key: &'static str,
    pub name: &'static str,
    /// `REG_SZ` or `REG_DWORD`.
    pub kind: &'static str,
    /// Value that makes Windows lighter (DWORD data in decimal).
    pub data: &'static str,
}

pub struct Tweak {
    pub id: &'static str,
    /// `high`, `medium` or `low`: how much it is expected to help on a weak PC.
    pub impact: &'static str,
    /// Takes full effect only after signing out and in again.
    pub restart: bool,
    /// Part of the one-click "weak PC" selection.
    pub recommended: bool,
    pub values: &'static [Value],
}

const ADV: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";
const CDM: &str = r"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager";
const DESKTOP: &str = r"Control Panel\Desktop";

const fn v(key: &'static str, name: &'static str, kind: &'static str, data: &'static str) -> Value {
    Value { key, name, kind, data }
}

pub static TWEAKS: &[Tweak] = &[
    Tweak {
        id: "animations",
        impact: "medium",
        restart: true,
        recommended: true,
        values: &[
            v(r"Control Panel\Desktop\WindowMetrics", "MinAnimate", "REG_SZ", "0"),
            v(ADV, "TaskbarAnimations", "REG_DWORD", "0"),
            v(r"Software\Microsoft\Windows\DWM", "EnableAeroPeek", "REG_DWORD", "0"),
        ],
    },
    Tweak {
        id: "transparency",
        impact: "medium",
        restart: false,
        recommended: true,
        values: &[v(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize", "EnableTransparency", "REG_DWORD", "0")],
    },
    Tweak {
        id: "menu-delay",
        impact: "low",
        restart: true,
        recommended: true,
        values: &[v(DESKTOP, "MenuShowDelay", "REG_SZ", "0")],
    },
    Tweak {
        id: "background-apps",
        impact: "high",
        restart: false,
        recommended: true,
        values: &[
            v(r"Software\Microsoft\Windows\CurrentVersion\BackgroundAccessApplications", "GlobalUserDisabled", "REG_DWORD", "1"),
            v(r"Software\Microsoft\Windows\CurrentVersion\Search", "BackgroundAppGlobalToggle", "REG_DWORD", "0"),
        ],
    },
    Tweak {
        id: "suggestions",
        impact: "low",
        restart: false,
        recommended: true,
        values: &[
            v(CDM, "SubscribedContent-338388Enabled", "REG_DWORD", "0"),
            v(CDM, "SubscribedContent-338389Enabled", "REG_DWORD", "0"),
            v(CDM, "SubscribedContent-353698Enabled", "REG_DWORD", "0"),
            v(CDM, "SystemPaneSuggestionsEnabled", "REG_DWORD", "0"),
            v(CDM, "SilentInstalledAppsEnabled", "REG_DWORD", "0"),
            v(CDM, "SoftLandingEnabled", "REG_DWORD", "0"),
            v(ADV, "ShowSyncProviderNotifications", "REG_DWORD", "0"),
        ],
    },
    Tweak {
        id: "game-dvr",
        impact: "medium",
        restart: false,
        recommended: true,
        values: &[
            v(r"System\GameConfigStore", "GameDVR_Enabled", "REG_DWORD", "0"),
            v(r"Software\Microsoft\Windows\CurrentVersion\GameDVR", "AppCaptureEnabled", "REG_DWORD", "0"),
        ],
    },
    Tweak {
        id: "startup-delay",
        impact: "medium",
        restart: true,
        recommended: true,
        values: &[v(r"Software\Microsoft\Windows\CurrentVersion\Explorer\Serialize", "StartupDelayInMSec", "REG_DWORD", "0")],
    },
    // Not part of the one-click selection: they change how Windows looks or behaves in a way
    // some people notice, so they stay opt-in.
    Tweak {
        id: "thumbnails",
        impact: "medium",
        restart: false,
        recommended: false,
        values: &[v(ADV, "IconsOnly", "REG_DWORD", "1")],
    },
    Tweak {
        id: "drag-contents",
        impact: "low",
        restart: true,
        recommended: false,
        values: &[v(DESKTOP, "DragFullWindows", "REG_SZ", "0")],
    },
];

#[derive(Debug, Clone, Serialize)]
pub struct TweakStatus {
    pub id: &'static str,
    pub impact: &'static str,
    pub restart: bool,
    pub recommended: bool,
    /// Every value already has the lighter setting.
    pub optimized: bool,
    /// WinRice saved the original values, so "restore" brings them back.
    pub restorable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Saved {
    key: String,
    name: String,
    /// `None` when the value did not exist (restoring deletes it).
    prev: Option<(String, String)>,
}

type Backup = BTreeMap<String, Vec<Saved>>;

fn backup_path(os: &dyn Os) -> String {
    format!("{}\\optimize-backup.json", os.data_dir())
}

fn load_backup(os: &dyn Os) -> Backup {
    os.read(&backup_path(os)).ok().flatten().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn save_backup(os: &dyn Os, b: &Backup) -> Result<(), String> {
    let json = serde_json::to_string_pretty(b).map_err(|e| e.to_string())?;
    os.write(&backup_path(os), &json).map_err(|e| e.to_string())
}

fn is_optimized(os: &dyn Os, t: &Tweak) -> bool {
    t.values.iter().all(|v| os.reg_get(v.key, v.name).is_some_and(|(_, d)| d == v.data))
}

pub fn status(os: &dyn Os) -> Vec<TweakStatus> {
    let backup = load_backup(os);
    TWEAKS
        .iter()
        .map(|t| TweakStatus {
            id: t.id,
            impact: t.impact,
            restart: t.restart,
            recommended: t.recommended,
            optimized: is_optimized(os, t),
            restorable: backup.contains_key(t.id),
        })
        .collect()
}

fn put_back(os: &dyn Os, saved: &Saved) -> Result<(), String> {
    match &saved.prev {
        Some((kind, data)) => os.reg_set(&saved.key, &saved.name, kind, data),
        None => os.reg_del(&saved.key, &saved.name),
    }
}

/// Applies the chosen tweaks. All or nothing: on a failure everything changed so far is restored.
pub fn apply(os: &dyn Os, ids: &[String]) -> Result<usize, String> {
    let tweaks: Vec<&Tweak> = ids
        .iter()
        .map(|id| TWEAKS.iter().find(|t| t.id == id).ok_or_else(|| format!("unknown_tweak:{id}")))
        .collect::<Result<_, _>>()?;
    let mut backup = load_backup(os);
    let mut done: Vec<Saved> = vec![];
    let mut new_entries: Backup = BTreeMap::new();
    let mut changed = 0;
    for t in tweaks {
        // The first backup is the real original; applying again must never overwrite it.
        let fresh = !backup.contains_key(t.id);
        let mut saved = vec![];
        for val in t.values {
            let prev = os.reg_get(val.key, val.name);
            let s = Saved { key: val.key.into(), name: val.name.into(), prev };
            if s.prev.as_ref().map(|(_, d)| d.as_str()) == Some(val.data) {
                saved.push(s);
                continue;
            }
            if let Err(e) = os.reg_set(val.key, val.name, val.kind, val.data) {
                for d in done.iter().rev() {
                    let _ = put_back(os, d);
                }
                return Err(e);
            }
            done.push(s.clone());
            saved.push(s);
        }
        if fresh {
            new_entries.insert(t.id.into(), saved);
        }
        changed += 1;
    }
    backup.extend(new_entries);
    if let Err(e) = save_backup(os, &backup) {
        for d in done.iter().rev() {
            let _ = put_back(os, d);
        }
        return Err(e);
    }
    Ok(changed)
}

/// Puts the saved original values back for the given tweaks (all of them when `ids` is empty).
pub fn restore(os: &dyn Os, ids: &[String]) -> Result<usize, String> {
    let mut backup = load_backup(os);
    let wanted: Vec<String> = if ids.is_empty() { backup.keys().cloned().collect() } else { ids.to_vec() };
    let mut count = 0;
    let mut errors = vec![];
    for id in wanted {
        let Some(saved) = backup.get(&id) else { continue };
        let failed: Vec<String> = saved.iter().rev().filter_map(|s| put_back(os, s).err()).collect();
        if failed.is_empty() {
            backup.remove(&id);
            count += 1;
        } else {
            errors.extend(failed);
        }
    }
    save_backup(os, &backup)?;
    if let Some(e) = errors.into_iter().next() {
        return Err(e);
    }
    Ok(count)
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Suggestion {
    /// Text key under `optimize.suggestion.<id>` in the locale files.
    pub id: &'static str,
    /// `high`, `medium` or `info`.
    pub level: &'static str,
}

const GB: u64 = 1024 * 1024 * 1024;

/// Advice that WinRice cannot (or should not) apply by itself, chosen from what this PC has.
pub fn suggest(s: &Snapshot) -> Vec<Suggestion> {
    let mut out = vec![];
    let total_gb = s.ram.total / GB;
    if s.ram.total > 0 && total_gb <= 4 {
        out.push(Suggestion { id: "ram_low", level: "high" });
    } else if s.ram.total > 0 && total_gb <= 8 {
        out.push(Suggestion { id: "ram_ok", level: "medium" });
    }
    if s.ram.percent >= 85.0 {
        out.push(Suggestion { id: "ram_busy", level: "high" });
    }
    let system = s.disks.iter().find(|d| d.mount.to_uppercase().starts_with("C:")).or_else(|| s.disks.first());
    if let Some(d) = system {
        if d.total > 0 && (d.total.saturating_sub(d.used)) as f64 / (d.total as f64) < 0.15 {
            out.push(Suggestion { id: "disk_low", level: "high" });
        }
    }
    if s.system.uptime_s > 7 * 24 * 3600 {
        out.push(Suggestion { id: "restart", level: "medium" });
    }
    if s.ram.total > 0 && total_gb <= 8 {
        out.push(Suggestion { id: "heavy_tools", level: "medium" });
    }
    out.push(Suggestion { id: "startup_apps", level: "info" });
    out.push(Suggestion { id: "power_plan", level: "info" });
    out.push(Suggestion { id: "storage_sense", level: "info" });
    out
}

/// Reads the value out of `reg query` output: `    Name    REG_SZ    data`.
/// DWORD data comes back in decimal so it compares with the table above.
pub fn parse_reg_query(text: &str, name: &str) -> Option<(String, String)> {
    let line = text.lines().map(str::trim_start).find(|l| {
        l.len() > name.len() && l.get(..name.len()).is_some_and(|h| h.eq_ignore_ascii_case(name)) && l[name.len()..].starts_with(char::is_whitespace)
    })?;
    let rest = line[name.len()..].trim_start();
    let (kind, data) = rest.split_once(char::is_whitespace).map(|(k, d)| (k, d.trim())).unwrap_or((rest.trim(), ""));
    let data = if kind == "REG_DWORD" {
        u32::from_str_radix(data.trim_start_matches("0x"), 16).map(|n| n.to_string()).unwrap_or_else(|_| data.to_string())
    } else {
        data.to_string()
    };
    Some((kind.to_string(), data))
}
