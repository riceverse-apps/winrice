//! Phase 3.5 tests: hardware sampler, widget settings, profiles, pause/resume, quick actions.

use crate::engine;
use crate::hardware::Sampler;
use crate::manifest;
use crate::os::fake::{FakeOs, World};
use crate::profiles;
use crate::store::Store;
use crate::tests::*;
use crate::widget::WidgetSettings;

const ZEBAR_EXE: &str = r"%ProgramFiles%\glzr.io\Zebar\zebar.exe";

/// GlazeWM and Zebar installed and running, like a real rice.
fn rice() -> World {
    let mut w = glaze_active();
    w.files.insert(ZEBAR_EXE.into(), String::new());
    w.processes.insert("zebar.exe".into());
    w.run_keys.insert("WinRice-zebar".into(), format!("\"{ZEBAR_EXE}\""));
    w
}

fn ids(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn hardware_sampler_reports_real_values_without_admin() {
    let mut s = Sampler::new();
    std::thread::sleep(std::time::Duration::from_millis(300));
    let a = s.sample();
    std::thread::sleep(std::time::Duration::from_millis(300));
    let b = s.sample();

    assert!(!b.cpu.per_core.is_empty() && b.cpu.per_core.len() >= 1);
    assert!((0.0..=100.0).contains(&b.cpu.usage), "cpu {}", b.cpu.usage);
    assert!(b.cpu.per_core.iter().all(|c| (0.0..=100.0).contains(c)));
    assert!(b.ram.total > 0 && b.ram.used <= b.ram.total);
    assert!((0.0..=100.0).contains(&b.ram.percent));
    assert!(b.system.uptime_s > 0 && !b.system.os.is_empty());
    assert!(b.interval_s > 0.1 && b.interval_s < 5.0, "interval {}", b.interval_s);
    assert!(a.disks.iter().all(|d| d.used <= d.total));
    // Sensors that need the privileged helper are reported as unavailable, not faked.
    assert!(!b.sensors.available && b.sensors.cpu_temp_c.is_none() && b.sensors.gpu_temp_c.is_none());
    assert_eq!(b.sensors.reason, "needs_admin_helper");
}

#[test]
fn widget_settings_are_clamped_and_forward_compatible() {
    let d = WidgetSettings::default();
    assert_eq!(d.refresh_ms, 1000);
    assert!(d.flyout && !d.floating && d.shortcut == "Super+Alt+W");

    let wild = WidgetSettings {
        refresh_ms: 100,
        opacity: f32::NAN,
        corner: "middle".into(),
        layer: "sideways".into(),
        metrics: ids(&["cpu", "bogus", "ram"]),
        alert_ram_pct: Some(500),
        alert_cpu_temp: Some(5),
        shortcut: "  ".into(),
        ..d.clone()
    }
    .sanitized();
    assert_eq!(wild.refresh_ms, 500, "never faster than 500 ms");
    assert_eq!(wild.opacity, d.opacity);
    assert_eq!((wild.corner.as_str(), wild.layer.as_str()), ("bottom-right", "normal"));
    assert_eq!(wild.metrics, ids(&["cpu", "ram"]));
    assert_eq!((wild.alert_ram_pct, wild.alert_cpu_temp), (Some(100), Some(30)));
    assert_eq!(wild.shortcut, "Super+Alt+W");

    assert_eq!(WidgetSettings { refresh_ms: 10_000_000, ..d.clone() }.sanitized().refresh_ms, 60_000);
    assert_eq!(WidgetSettings { opacity: 0.0, ..d.clone() }.sanitized().opacity, 0.3);

    // older/partial saved JSON keeps working and picks up new defaults
    let partial: WidgetSettings = serde_json::from_str(r#"{"floating":true,"refresh_ms":2000}"#).unwrap();
    assert!(partial.floating && partial.refresh_ms == 2000 && partial.flyout);
    let round: WidgetSettings = serde_json::from_str(&serde_json::to_string(&wild).unwrap()).unwrap();
    assert_eq!(round, wild);
}

#[test]
fn pause_then_resume_restores_exactly_the_same_state() {
    let all = manifest::all();
    let initial = rice();
    let os = FakeOs::new(initial.clone());
    let before = profiles::active_tools(&os, &all);
    assert_eq!(before, ids(&["glazewm", "zebar"]));

    let paused = profiles::switch(&os, &all, &[]);
    assert!(paused.outcome.ok, "{:?}", paused.outcome.error);
    assert!(profiles::active_tools(&os, &all).is_empty());
    let m = os.machine();
    assert!(m.processes.is_empty() && m.run_keys.is_empty(), "everything is really off");

    let resumed = profiles::switch(&os, &all, &before);
    assert!(resumed.outcome.ok, "{:?}", resumed.outcome.error);
    assert_eq!(profiles::active_tools(&os, &all), before);
    assert_eq!(os.machine(), initial, "resume must give back exactly the same machine");
}

#[test]
fn pause_and_resume_are_all_or_nothing() {
    let all = manifest::all();
    // pause
    let probe = FakeOs::new(rice());
    assert!(profiles::switch(&probe, &all, &[]).outcome.ok);
    for n in 0..probe.ticks() {
        let os = FakeOs::new(rice());
        *os.fail_at.lock().unwrap() = Some(n);
        let r = profiles::switch(&os, &all, &[]);
        assert!(!r.outcome.ok && r.outcome.rolled_back, "pause op {n}");
        assert_eq!(os.machine(), rice(), "pause op {n} left a mess");
    }
    // resume
    let mut off = rice();
    off.processes.clear();
    off.run_keys.clear();
    let probe = FakeOs::new(off.clone());
    assert!(profiles::switch(&probe, &all, &ids(&["glazewm", "zebar"])).outcome.ok);
    for n in 0..probe.ticks() {
        let os = FakeOs::new(off.clone());
        *os.fail_at.lock().unwrap() = Some(n);
        let r = profiles::switch(&os, &all, &ids(&["glazewm", "zebar"]));
        assert!(!r.outcome.ok && r.outcome.rolled_back, "resume op {n}");
        assert_eq!(os.machine(), off, "resume op {n} left a mess");
    }
}

#[test]
fn switching_profile_changes_only_what_differs_and_reports_skipped_tools() {
    let all = manifest::all();
    let os = FakeOs::new(rice());
    // keep glazewm, drop zebar, ask for flow-launcher (not installed) and an unknown tool
    let r = profiles::switch(&os, &all, &ids(&["glazewm", "flow-launcher", "ghost"]));
    assert!(r.outcome.ok);
    assert_eq!(r.skipped, ids(&["flow-launcher", "ghost"]));
    assert_eq!(profiles::active_tools(&os, &all), ids(&["glazewm"]));
    assert!(os.machine().processes.contains("glazewm.exe"), "glazewm was left alone");
    assert!(engine::revert(&os, &r.journal).is_empty());
    assert_eq!(os.machine(), rice(), "the switch can be undone completely");
}

#[test]
fn switching_to_a_clashing_profile_turns_the_old_tools_off_first() {
    let all = manifest::all();
    let mut w = rice();
    w.files.insert(r"%ProgramFiles%\komorebi\bin\komorebic.exe".into(), String::new());
    w.files.insert(r"%ProgramFiles%\komorebi\bin\komorebi.exe".into(), String::new());
    let os = FakeOs::new(w);
    let r = profiles::switch(&os, &all, &ids(&["komorebi"]));
    assert!(r.outcome.ok, "{:?}", r.outcome.error);
    assert_eq!(profiles::active_tools(&os, &all), ids(&["komorebi"]));
}

#[test]
fn quick_actions_resolve_to_the_installed_program() {
    let glaze = tool("glazewm");
    let w = glaze.widget.as_ref().expect("glazewm declares widget options");
    assert!(w.show_by_default && w.quick_actions.iter().any(|q| q.id == "reload-config"));

    let os = FakeOs::new(glaze_installed());
    let (prog, args) = profiles::quick_action_command(&os, &glaze, "reload-config").unwrap();
    assert_eq!(prog, GLAZE_EXE);
    assert_eq!(args, ids(&["command", "wm-reload-config"]));
    assert_eq!(profiles::quick_action_command(&os, &glaze, "nope").unwrap_err(), "unknown_action");
    assert_eq!(profiles::quick_action_command(&os, &tool("zebar"), "reload-config").unwrap_err(), "unknown_action");
}

#[test]
fn profiles_and_pause_state_are_stored() {
    let s = Store::in_memory().unwrap();
    s.save_profile("Work", &ids(&["glazewm", "zebar"]), 1);
    s.save_profile("Games", &[], 2);
    s.save_profile("Work", &ids(&["glazewm"]), 3); // saving again replaces
    let p = s.profiles();
    assert_eq!(p.len(), 2);
    assert_eq!(p.iter().find(|(n, _)| n == "Work").unwrap().1, ids(&["glazewm"]));
    s.delete_profile("Games");
    assert_eq!(s.profiles().len(), 1);

    s.meta_set("paused", "[\"glazewm\"]");
    assert!(s.meta_get("paused").is_some());
    s.meta_del("paused");
    assert!(s.meta_get("paused").is_none());
}
