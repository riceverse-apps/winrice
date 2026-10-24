//! Self-test: runs the WinRice engine for real (winget, processes, registry, files, tasks) and
//! reports what works. It exists to exercise `RealOs`, which the unit tests (fake machine) cannot.
//!
//! ```text
//! winrice.exe --selftest <output.json>
//! ```
//!
//! SAFETY: it installs and removes software and edits the registry, so it REFUSES to run unless
//! the current user is the Windows Sandbox account (`WDAGUtilityAccount`).

use crate::actions::{self, Action};
use crate::engine::{self, Journal, Plan, Step};
use crate::manifest::{self, Manifest};
use crate::net;
use crate::os::{Os, RealOs};
use crate::profiles;
use crate::settings;
use crate::store::Store;
use crate::theme;
use crate::themeops;
use serde::Serialize;
use std::time::{Duration, Instant};

#[derive(Serialize)]
struct Check {
    name: String,
    ok: bool,
    detail: String,
    seconds: u64,
}

struct T {
    os: RealOs,
    all: Vec<Manifest>,
    out: Vec<Check>,
    path: String,
    installed: Vec<String>,
}

/// 1x1 transparent PNG.
const PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06,
    0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8, 0xFF, 0xFF, 0x3F, 0x00, 0x05, 0xFE, 0x02,
    0xFE, 0xA7, 0x35, 0x81, 0x84, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
];

impl T {
    fn m(&self, id: &str) -> Manifest {
        self.all.iter().find(|m| m.id == id).cloned().unwrap_or_else(|| panic!("no manifest {id}"))
    }
    fn st(&self, id: &str) -> actions::ToolStatus {
        actions::status(&self.os, &self.m(id))
    }
    fn plan(&self, id: &str, action: Action, resolve: bool) -> Result<Plan, String> {
        actions::plan(&self.os, &self.all, &self.m(id), action, resolve).map_err(|e| e.to_string())
    }
    /// Plans and runs an action through the engine; fails with the full explanation if it did not go through.
    fn run(&self, id: &str, action: Action, resolve: bool) -> Result<Journal, String> {
        let plan = self.plan(id, action, resolve)?;
        self.run_plan(&plan)
    }
    fn run_plan(&self, plan: &Plan) -> Result<Journal, String> {
        let (o, j) = engine::run_journal(&self.os, plan);
        if o.ok {
            Ok(j)
        } else {
            Err(format!("failed at step {}: {:?} (rolled back: {}, rollback errors: {:?})", o.applied, o.error, o.rolled_back, o.rollback_errors))
        }
    }
    fn undo(&self, j: &Journal) -> Result<(), String> {
        let e = engine::revert(&self.os, j);
        if e.is_empty() { Ok(()) } else { Err(format!("undo errors: {e:?}")) }
    }
    fn file(&self, p: &str) -> Option<String> {
        self.os.read(&self.os.expand(p)).ok().flatten()
    }
    fn save(&self) {
        let _ = std::fs::write(&self.path, serde_json::to_string_pretty(&self.out).unwrap_or_default());
    }
}

fn wait_for(secs: u64, cond: impl Fn() -> bool) -> bool {
    let end = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < end {
        if cond() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    cond()
}

fn ensure(c: bool, what: &str) -> Result<(), String> {
    if c { Ok(()) } else { Err(what.to_string()) }
}

pub fn run_if_requested() -> Option<i32> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("--selftest") {
        return None;
    }
    if std::env::var("USERNAME").ok().as_deref() != Some("WDAGUtilityAccount") {
        eprintln!("--selftest installs software and edits the registry: it only runs inside Windows Sandbox.");
        return Some(5);
    }
    let path = args.get(1).cloned().unwrap_or_else(|| "selftest.json".into());
    Some(run(path))
}

fn run(path: String) -> i32 {
    let os = RealOs;
    let root = std::path::PathBuf::from(os.data_dir());
    let all = manifest::all();
    let mut t = T { os, all, out: vec![], path, installed: vec![] };

    macro_rules! check {
        ($name:expr, $body:block) => {{
            let _ = std::fs::write(format!("{}.progress", t.path), $name);
            let start = Instant::now();
            let r: Result<String, String> = (|| -> Result<String, String> { $body })();
            let (ok, detail) = match r {
                Ok(d) => (true, d),
                Err(e) => (false, e),
            };
            t.out.push(Check { name: $name.into(), ok, detail, seconds: start.elapsed().as_secs() });
            t.save();
        }};
    }

    // ------------------------------------------------------------ network / catalog
    check!("remote catalog: download, verify SHA-256 and load", {
        let s = net::sync_catalog(&net::RealHttp, &root, net::INDEX_URL)?;
        ensure(s.errors.is_empty(), &format!("errors: {:?}", s.errors))?;
        let loaded = manifest::all_with(&root).len();
        ensure(loaded == 20, &format!("{loaded} tools after sync"))?;
        Ok(format!("{} tools and {} themes downloaded and verified", s.tools_updated.len(), s.themes_updated.len()))
    });
    t.all = manifest::all_with(&root);

    check!("hardware monitor reads real values", {
        let mut s = crate::hardware::Sampler::new();
        std::thread::sleep(Duration::from_millis(500));
        let a = s.sample();
        ensure(a.ram.total > 0 && !a.cpu.per_core.is_empty() && !a.system.os.is_empty(), "empty readings")?;
        Ok(format!("{} cores, RAM {} MB, {}", a.cpu.per_core.len(), a.ram.total / 1_048_576, a.system.os))
    });

    check!("history database opens, logs and computes undoable entries", {
        let p = root.join("selftest-state.db");
        let _ = std::fs::remove_file(&p);
        let s = Store::open(&p.to_string_lossy()).map_err(|e| e.to_string())?;
        s.log_j(1, "a", "activate", "ok", None, Some("[]"));
        s.log_j(2, "b", "activate", "ok", None, Some("[]"));
        let h = s.history(10);
        ensure(h.len() == 2 && h[0].undoable && !h[1].undoable, "undoable flags wrong")?;
        Ok("ok".into())
    });

    // ------------------------------------------------------------ install / on / off / undo (Flow Launcher)
    check!("install Flow Launcher with winget", {
        t.run("flow-launcher", Action::Install, false)?;
        t.installed.push("flow-launcher".into());
        ensure(t.st("flow-launcher").installed, "not detected as installed afterwards")?;
        Ok("installed and detected".into())
    });
    check!("installing it again does not fail (already installed)", {
        // the plan is refused up front; running a raw install step must also be accepted by winget handling
        let plan = Plan { tool: "flow-launcher".into(), action: "install".into(), steps: vec![Step::WingetInstall { tool: "flow-launcher".into(), id: "Flow-Launcher.Flow-Launcher".into() }] };
        t.run_plan(&plan)?;
        ensure(t.st("flow-launcher").installed, "lost after a repeated install")?;
        Ok("winget code for 'already installed' accepted".into())
    });
    check!("turn Flow Launcher ON: process runs, autostart entry written", {
        let _ = t.run("flow-launcher", Action::Activate, false)?;
        let m = t.m("flow-launcher");
        ensure(t.st("flow-launcher").active, "status is not active")?;
        ensure(t.os.process_running(m.detect.process.as_deref().unwrap_or("")), "process not running")?;
        ensure(t.os.run_key_get("WinRice-flow-launcher").is_some(), "no autostart entry in the Run key")?;
        Ok(format!("run key = {}", t.os.run_key_get("WinRice-flow-launcher").unwrap_or_default()))
    });
    check!("try mode / undo: reverting turns it OFF and removes the autostart entry", {
        // the previous check left it on; take a journal of a fresh off->on cycle and revert it
        t.run("flow-launcher", Action::Deactivate, false)?;
        let j = t.run("flow-launcher", Action::Activate, false)?;
        ensure(t.st("flow-launcher").active, "did not turn on")?;
        t.undo(&j)?;
        ensure(!t.st("flow-launcher").active, "still active after revert")?;
        ensure(t.os.run_key_get("WinRice-flow-launcher").is_none(), "autostart entry survived the revert")?;
        Ok("reverted completely".into())
    });
    check!("turn Flow Launcher OFF by the toggle", {
        t.run("flow-launcher", Action::Activate, false)?;
        t.run("flow-launcher", Action::Deactivate, false)?;
        ensure(!t.st("flow-launcher").active, "still active")?;
        ensure(t.os.run_key_get("WinRice-flow-launcher").is_none(), "autostart entry left")?;
        Ok("off".into())
    });
    check!("a real failure in the middle rolls everything back", {
        let plan = Plan {
            tool: "selftest".into(),
            action: "install".into(),
            steps: vec![
                Step::SetRunKey { tool: "selftest".into(), name: "WinRice-selftest".into(), command: "cmd.exe".into() },
                Step::WingetInstall { tool: "selftest".into(), id: "Nonexistent.Package.Zzzz".into() },
            ],
        };
        let (o, _) = engine::run_journal(&t.os, &plan);
        ensure(!o.ok && o.rolled_back, "the operation should have failed and rolled back")?;
        ensure(t.os.run_key_get("WinRice-selftest").is_none(), "the registry entry of the first step was left behind")?;
        Ok(format!("winget failed ({:?}) and the registry entry was removed", o.error))
    });

    // ------------------------------------------------------------ GlazeWM: settings, palette, conflicts
    check!("install GlazeWM and Komorebi", {
        for id in ["glazewm", "komorebi"] {
            t.run(id, Action::Install, false)?;
            t.installed.push(id.into());
        }
        ensure(t.st("glazewm").installed && t.st("komorebi").installed, "not detected")?;
        Ok("both installed".into())
    });
    check!("turn GlazeWM ON (process, config created)", {
        t.run("glazewm", Action::Activate, false)?;
        ensure(t.st("glazewm").active, "not active")?;
        ensure(wait_for(15, || t.file("%USERPROFILE%\\.glzr\\glazewm\\config.yaml").is_some()), "GlazeWM did not create its config")?;
        Ok("running, config exists".into())
    });
    check!("option change writes the config, keeps a backup, and undo restores it", {
        let cfg = "%USERPROFILE%\\.glzr\\glazewm\\config.yaml";
        let before = t.file(cfg).ok_or("no config")?;
        let plan = settings::plan_set(&t.os, &t.m("glazewm"), "gaps.inner_gap", serde_json::json!(8))?;
        let j = t.run_plan(&plan)?;
        let after = t.file(cfg).ok_or("config vanished")?;
        ensure(after.contains("inner_gap: 8px") || after.contains("inner_gap: '8px'"), &format!("value not written:\n{}", after.lines().filter(|l| l.contains("gap")).collect::<Vec<_>>().join("\n")))?;
        ensure(settings::read_all(&t.os, &t.m("glazewm"))["gaps.inner_gap"] == serde_json::json!(8.0), "read back a different value")?;
        let backups = t.os.list_dir(&format!("{}\\backups\\originals", t.os.data_dir()));
        ensure(!backups.is_empty(), "no original was backed up")?;
        t.undo(&j)?;
        ensure(t.file(cfg).as_deref() == Some(before.as_str()), "undo did not restore the exact original file")?;
        Ok("written, read back, backed up, restored byte for byte".into())
    });
    check!("palette sync recolors GlazeWM and undo restores it", {
        let cfg = "%USERPROFILE%\\.glzr\\glazewm\\config.yaml";
        let before = t.file(cfg).ok_or("no config")?;
        let palette = theme::load_embedded().into_iter().find(|e| e.theme.id == "catppuccin-dev").and_then(|e| e.theme.palette).ok_or("no palette")?;
        let (o, j, changes) = themeops::apply_palette(&t.os, &t.all, &palette);
        ensure(o.ok, &format!("{:?}", o.error))?;
        ensure(!changes.is_empty(), "nothing to recolor")?;
        ensure(t.file(cfg).ok_or("no config")?.to_lowercase().contains("cba6f7"), "color not in the file")?;
        t.undo(&j)?;
        ensure(t.file(cfg).as_deref() == Some(before.as_str()), "undo did not restore the file")?;
        Ok(format!("{} option(s) recolored and restored", changes.len()))
    });
    check!("conflict: Komorebi is blocked while GlazeWM is on; 'turn the other off' switches them in one step", {
        match t.plan("komorebi", Action::Activate, false) {
            Err(e) if e.starts_with("conflict:") => {}
            other => return Err(format!("expected a conflict, got {other:?}")),
        }
        t.run("komorebi", Action::Activate, true)?;
        ensure(t.st("komorebi").active, "komorebi not active")?;
        ensure(!t.st("glazewm").active, "glazewm was not turned off")?;
        t.run("komorebi", Action::Deactivate, false)?;
        ensure(!t.st("komorebi").active, "komorebi did not stop")?;
        Ok("blocked, then resolved".into())
    });

    // ------------------------------------------------------------ profiles / pause
    check!("pause customizations then resume gives back exactly the same set", {
        t.run("glazewm", Action::Activate, false)?;
        t.run("flow-launcher", Action::Activate, false)?;
        let mut before = profiles::active_tools(&t.os, &t.all);
        before.sort();
        let p = profiles::switch(&t.os, &t.all, &[]);
        ensure(p.outcome.ok, &format!("pause: {:?}", p.outcome.error))?;
        ensure(profiles::active_tools(&t.os, &t.all).is_empty(), "something is still on after the pause")?;
        let r = profiles::switch(&t.os, &t.all, &before);
        ensure(r.outcome.ok, &format!("resume: {:?}", r.outcome.error))?;
        let mut after = profiles::active_tools(&t.os, &t.all);
        after.sort();
        ensure(before == after, &format!("before {before:?} after {after:?}"))?;
        Ok(format!("{before:?} paused and resumed"))
    });

    // ------------------------------------------------------------ themes
    check!("apply the 'Minimal Clean' theme (Flow Launcher only) and undo it", {
        t.run("flow-launcher", Action::Deactivate, false).ok();
        let entry = theme::load_embedded().into_iter().find(|e| e.theme.id == "minimal-clean").ok_or("no theme")?;
        let (o, j) = themeops::apply(&t.os, &t.all, &entry, &["flow-launcher".to_string()]);
        ensure(o.ok, &format!("{:?}", o.error))?;
        ensure(t.st("flow-launcher").active, "the theme did not turn it on")?;
        t.undo(&j)?;
        ensure(!t.st("flow-launcher").active, "undo did not turn it off")?;
        Ok("applied and undone".into())
    });
    check!("apply 'Catppuccin Dev' (GlazeWM option) and undo it, even from a stored journal", {
        t.run("glazewm", Action::Deactivate, false).ok();
        let entry = theme::load_embedded().into_iter().find(|e| e.theme.id == "catppuccin-dev").ok_or("no theme")?;
        let (o, j) = themeops::apply(&t.os, &t.all, &entry, &["glazewm".to_string()]);
        ensure(o.ok, &format!("{:?}", o.error))?;
        ensure(t.file("%USERPROFILE%\\.glzr\\glazewm\\config.yaml").is_some_and(|c| c.contains("12px")), "theme option not written")?;
        let restored = Journal::from_json(&j.to_json()).ok_or("journal does not survive JSON")?;
        t.undo(&restored)?;
        ensure(!t.st("glazewm").active, "glazewm still on after undo")?;
        Ok("applied; undone from the serialized journal".into())
    });

    // ------------------------------------------------------------ Oh My Posh and the PowerShell profile
    check!("Oh My Posh: profile block added, the shell still starts, block removed, profile restored", {
        t.run("oh-my-posh", Action::Install, false)?;
        t.installed.push("oh-my-posh".into());
        let files = ["%USERPROFILE%\\Documents\\PowerShell\\Microsoft.PowerShell_profile.ps1", "%USERPROFILE%\\Documents\\WindowsPowerShell\\Microsoft.PowerShell_profile.ps1"];
        let before: Vec<String> = files.iter().map(|f| t.file(f).unwrap_or_default()).collect();
        t.run("oh-my-posh", Action::Activate, false)?;
        ensure(t.st("oh-my-posh").active, "status not active")?;
        ensure(files.iter().any(|f| t.file(f).is_some_and(|c| c.contains("oh-my-posh init pwsh"))), "snippet not in any profile")?;
        let out = std::process::Command::new("powershell").args(["-NoLogo", "-Command", "Write-Output SHELL_STARTED"]).output().map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&out.stdout).to_string();
        ensure(text.contains("SHELL_STARTED"), &format!("PowerShell failed to start with the profile: {text} {}", String::from_utf8_lossy(&out.stderr)))?;
        t.run("oh-my-posh", Action::Deactivate, false)?;
        ensure(!t.st("oh-my-posh").active, "still active")?;
        for (f, b) in files.iter().zip(&before) {
            ensure(t.file(f).unwrap_or_default().trim() == b.trim(), &format!("profile {f} differs from the original"))?;
        }
        Ok("profile edited and restored; PowerShell started with it".into())
    });

    // ------------------------------------------------------------ wallpaper, scheduled task
    check!("wallpaper is set and undo puts the previous one back", {
        let prev = t.os.wallpaper_get();
        let plan = Plan { tool: "selftest".into(), action: "theme_wallpaper".into(), steps: vec![Step::SetWallpaper { theme: "selftest".into(), name: "w.png".into(), bytes: PNG.to_vec() }] };
        let j = t.run_plan(&plan)?;
        let now = t.os.wallpaper_get().unwrap_or_default();
        ensure(now.ends_with("selftest-w.png"), &format!("registry says {now:?}"))?;
        t.undo(&j)?;
        ensure(t.os.wallpaper_get() == prev, &format!("not restored: {:?} vs {:?}", t.os.wallpaper_get(), prev))?;
        Ok("registry value changed and restored".into())
    });
    check!("scheduled task can be read, disabled and enabled", {
        let _ = std::process::Command::new("schtasks").args(["/Create", "/TN", "\\WinRiceSelfTest", "/TR", "cmd.exe", "/SC", "ONLOGON", "/F"]).output();
        ensure(t.os.task_enabled("\\WinRiceSelfTest") == Some(true), "task not found or not enabled")?;
        t.os.task_set_enabled("\\WinRiceSelfTest", false)?;
        ensure(t.os.task_enabled("\\WinRiceSelfTest") == Some(false), "did not disable")?;
        t.os.task_set_enabled("\\WinRiceSelfTest", true)?;
        ensure(t.os.task_enabled("\\WinRiceSelfTest") == Some(true), "did not enable")?;
        let _ = std::process::Command::new("schtasks").args(["/Delete", "/TN", "\\WinRiceSelfTest", "/F"]).output();
        ensure(t.os.task_enabled("\\NoSuchTaskHere").is_none(), "a missing task should read as None")?;
        Ok("ok (changed directly: the Sandbox user is an administrator)".into())
    });

    // ------------------------------------------------------------ restore original Windows
    check!("Restore original Windows: everything off, configs back, installed tools removed", {
        t.run("glazewm", Action::Activate, false).ok();
        t.run("flow-launcher", Action::Activate, false).ok();
        let plan = actions::plan_restore(&t.os, &t.all, &t.installed, true);
        let steps = plan.steps.len();
        let j = t.run_plan(&plan);
        let _ = j.as_ref().map_err(|e| e.clone())?;
        ensure(profiles::active_tools(&t.os, &t.all).is_empty(), "tools are still on")?;
        ensure(t.file("%USERPROFILE%\\.glzr\\glazewm\\config.yaml").is_none(), "GlazeWM config was not removed (it did not exist originally)")?;
        let left: Vec<&String> = t.installed.iter().filter(|id| t.st(id).installed).collect();
        ensure(left.is_empty(), &format!("still installed: {left:?}"))?;
        Ok(format!("{steps} steps; all tools off and removed"))
    });

    let failed = t.out.iter().filter(|c| !c.ok).count();
    t.save();
    if failed == 0 { 0 } else { 1 }
}
