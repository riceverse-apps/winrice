//! Phase 2 tests: try mode, conflicts, compatibility, watchdog, settings, full catalog.

use crate::actions::{self, Action, PlanError};
use crate::engine;
use crate::manifest::{self, Manifest, Setting};
use crate::os::fake::{FakeOs, World};
use crate::settings;
use crate::tests::*;
use crate::trial::{LastOp, Watchdog, Who};
use serde_json::json;

#[test]
fn try_mode_journal_reverts_a_successful_change_exactly() {
    for (initial, id) in [(glaze_installed(), "glazewm"), (omp_installed(), "oh-my-posh")] {
        let os = FakeOs::new(initial.clone());
        let plan = actions::plan(&os, &manifest::all(), &tool(id), Action::Activate, false).unwrap();
        let (out, journal) = engine::run_journal(&os, &plan);
        assert!(out.ok);
        assert_ne!(os.machine(), initial);
        assert!(engine::revert(&os, &journal).is_empty());
        assert_eq!(os.machine(), initial, "{id} not fully reverted");
    }
}

#[test]
fn only_an_unconfirmed_try_can_be_reverted_by_the_timer() {
    let os = FakeOs::new(glaze_installed());
    let plan = actions::plan(&os, &manifest::all(), &tool("glazewm"), Action::Activate, false).unwrap();
    let last = LastOp::default();

    // confirmed -> the timer finds nothing to revert
    let (_, j) = engine::run_journal(&os, &plan);
    let t = last.store("glazewm", "activate", j, true, None);
    assert!(last.keep(t));
    assert!(last.take(Who::Token(t), true).is_none());

    // not confirmed -> the timer takes it and the machine goes back
    let os2 = FakeOs::new(glaze_installed());
    let (_, j) = engine::run_journal(&os2, &plan);
    let t2 = last.store("glazewm", "activate", j, true, None);
    let slot = last.take(Who::Token(t2), true).expect("timer takes an unconfirmed try");
    assert!(engine::revert(&os2, &slot.journal).is_empty());
    assert_eq!(os2.machine(), glaze_installed());
    assert!(last.take(Who::Anyone, false).is_none());
}

#[test]
fn a_newer_operation_replaces_the_old_slot() {
    let last = LastOp::default();
    let os = FakeOs::new(glaze_installed());
    let plan = actions::plan(&os, &manifest::all(), &tool("glazewm"), Action::Activate, false).unwrap();
    let (_, j1) = engine::run_journal(&os, &plan);
    let old = last.store("a", "activate", j1, true, None);
    let (_, j2) = engine::run_journal(&FakeOs::new(glaze_installed()), &plan);
    let new = last.store("b", "activate", j2, false, None);
    assert!(last.take(Who::Token(old), true).is_none(), "old timer must not touch the new operation");
    assert!(last.is_current(new));
    assert!(last.take(Who::Anyone, false).is_some(), "emergency shortcut still works");
}

#[test]
fn watchdog_triggers_only_on_repeated_explorer_restarts() {
    let mut dog = Watchdog::default();
    assert!(!dog.observe(vec![100]));
    assert!(!dog.observe(vec![100])); // stable
    assert!(!dog.observe(vec![])); // being restarted
    assert!(!dog.observe(vec![101])); // restart 1
    assert!(!dog.observe(vec![102])); // restart 2
    assert!(dog.observe(vec![103])); // restart 3 -> crash loop

    let mut calm = Watchdog::default();
    for _ in 0..30 {
        assert!(!calm.observe(vec![7]));
    }
    let mut once = Watchdog::default();
    once.observe(vec![1]);
    assert!(!once.observe(vec![2]), "a single restart is normal");
}

#[test]
fn incompatible_windows_build_blocks_install_and_activate() {
    let os = FakeOs::new(glaze_installed());
    *os.build.lock().unwrap() = 19045; // Windows 10
    let glaze = tool("glazewm");
    assert!(!actions::status(&os, &glaze).compatible);
    let all = manifest::all();
    assert_eq!(actions::plan(&os, &all, &glaze, Action::Activate, false).unwrap_err(), PlanError::Incompatible);
    assert_eq!(actions::plan(&os, &all, &glaze, Action::Install, false).unwrap_err(), PlanError::Incompatible);

    // too new for a tool with a max build
    let mut capped = tool("glazewm");
    capped.requires.windows_max_build = Some(22631);
    *os.build.lock().unwrap() = 26100;
    assert!(!actions::status(&os, &capped).compatible);
}

fn clashing_pair() -> (Manifest, Manifest, World) {
    let mut glaze = tool("glazewm");
    let mut komo = tool("zebar");
    komo.id = "komorebi".into();
    komo.detect.process = Some("komorebi.exe".into());
    komo.detect.paths = vec![r"C:\k\komorebi.exe".into()];
    komo.activate.as_mut().unwrap().start = Some("komorebi.exe".into());
    komo.activate.as_mut().unwrap().autostart = false;
    komo.deactivate.as_mut().unwrap().remove_autostart = false;
    glaze.conflicts = vec!["komorebi".into()];
    let mut w = glaze_installed();
    w.files.insert(r"C:\k\komorebi.exe".into(), "".into());
    w.processes.insert("komorebi.exe".into());
    (glaze, komo, w)
}

#[test]
fn resolving_a_conflict_turns_the_other_tool_off_first_and_is_atomic() {
    let (glaze, komo, world) = clashing_pair();
    let all = [glaze.clone(), komo];

    let os = FakeOs::new(world.clone());
    assert!(matches!(actions::plan(&os, &all, &glaze, Action::Activate, false), Err(PlanError::Conflict(_))));
    let plan = actions::plan(&os, &all, &glaze, Action::Activate, true).unwrap();
    assert!(engine::run(&os, &plan).ok);
    let after = os.machine();
    assert!(!after.processes.contains("komorebi.exe"), "the other tool was turned off");
    assert!(after.processes.contains("glazewm.exe"), "the new tool is on");

    // a failure anywhere (even after the other tool was stopped) restores both
    for n in 0..os.ticks() {
        let os = FakeOs::new(world.clone());
        let plan = actions::plan(&os, &all, &glaze, Action::Activate, true).unwrap();
        *os.fail_at.lock().unwrap() = Some(n);
        let out = engine::run(&os, &plan);
        assert!(!out.ok && out.rolled_back);
        assert_eq!(os.machine(), world, "failure at op {n}");
    }
}

fn gap_setting() -> Setting {
    Setting {
        key: "gaps.inner_gap".into(),
        kind: "number".into(),
        label: Default::default(),
        default: json!(20),
        suffix: Some("px".into()),
        min: Some(0.0),
        max: Some(200.0),
        palette: None,
    }
}

#[test]
fn settings_are_read_written_validated_and_rolled_back() {
    let mut glaze = tool("glazewm");
    glaze.settings = vec![gap_setting()];
    let os = FakeOs::new(world(&[(GLAZE_EXE, ""), (GLAZE_CFG, "gaps:\n  inner_gap: 20px\nother: keep\n")]));
    assert_eq!(settings::read_all(&os, &glaze)["gaps.inner_gap"], json!(20.0));

    let plan = settings::plan_set(&os, &glaze, "gaps.inner_gap", json!(8)).unwrap();
    assert!(engine::run(&os, &plan).ok);
    let text = os.read_for_test(GLAZE_CFG);
    assert!(text.contains("inner_gap: 8px") && text.contains("other: keep"), "{text}");
    assert_eq!(settings::read_all(&os, &glaze)["gaps.inner_gap"], json!(8.0));

    assert!(settings::plan_set(&os, &glaze, "gaps.inner_gap", json!(999)).is_err(), "out of range");
    assert!(settings::plan_set(&os, &glaze, "gaps.inner_gap", json!("x")).is_err(), "wrong type");
    assert!(settings::plan_set(&os, &glaze, "nope", json!(1)).is_err(), "unknown key");

    let before = os.machine();
    let plan = settings::plan_set(&os, &glaze, "gaps.inner_gap", json!(50)).unwrap();
    let probe = FakeOs::new(before.clone());
    assert!(engine::run(&probe, &plan).ok);
    assert!(probe.ticks() >= 3);
    for n in 0..probe.ticks() {
        let os2 = FakeOs::new(before.clone());
        *os2.fail_at.lock().unwrap() = Some(n);
        let _ = engine::run(&os2, &plan);
        assert_eq!(os2.machine(), before, "failure at op {n}");
    }
}

#[test]
fn json_config_values_can_be_created_from_nothing() {
    let out = settings::config_set("", "json", "a.b", &json!(true)).unwrap();
    assert_eq!(settings::config_get(&out, "json", "a.b"), Some(json!(true)));
    assert!(settings::config_set("{}", "toml", "a", &json!(1)).is_err());
}

#[test]
fn full_catalog_is_valid_and_consistent() {
    let all = manifest::all();
    let ids: Vec<&str> = all.iter().map(|m| m.id.as_str()).collect();
    for m in &all {
        for o in &m.conflicts {
            assert!(ids.contains(&o.as_str()), "{} conflicts with unknown {o}", m.id);
        }
        // conflicts are bidirectional (10-guia-para-o-agente)
        for o in all.iter().filter(|o| m.conflicts.contains(&o.id)) {
            assert!(o.conflicts.contains(&m.id), "{} <-> {} conflict not declared on both sides", m.id, o.id);
        }
        assert!(manifest::docs(&m.id, "pt-BR").is_some());
    }
    assert_eq!(all.len(), 20, "catalog size");
}
