//! Transaction Engine tests. They run against an in-memory fake OS: nothing is ever
//! installed, started or written on the real machine.

use crate::actions::{self, Action, PlanError};
use crate::engine::{self, with_block, without_block};
use crate::manifest::{self, Manifest};
use crate::os::fake::{FakeOs, World};
use crate::store::Store;

pub(crate) const GLAZE_EXE: &str = r"%ProgramFiles%\glzr.io\GlazeWM\glazewm.exe";
pub(crate) const GLAZE_CFG: &str = r"%USERPROFILE%\.glzr\glazewm\config.yaml";
pub(crate) const OMP_EXE: &str = r"%LOCALAPPDATA%\Microsoft\WindowsApps\oh-my-posh.exe";
pub(crate) const PS7_PROFILE: &str = r"%USERPROFILE%\Documents\PowerShell\Microsoft.PowerShell_profile.ps1";
pub(crate) const PS5_PROFILE: &str = r"%USERPROFILE%\Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1";

pub(crate) fn tool(id: &str) -> Manifest {
    manifest::all().into_iter().find(|m| m.id == id).unwrap_or_else(|| panic!("no manifest {id}"))
}

pub(crate) fn world(files: &[(&str, &str)]) -> World {
    let mut w = World::default();
    for (k, v) in files {
        w.files.insert((*k).into(), (*v).into());
    }
    w
}

pub(crate) fn glaze_installed() -> World {
    world(&[(GLAZE_EXE, ""), (GLAZE_CFG, "gaps: 10\n")])
}

pub(crate) fn glaze_active() -> World {
    let mut w = glaze_installed();
    w.processes.insert("glazewm.exe".into());
    w.run_keys.insert("WinRice-glazewm".into(), format!("\"{GLAZE_EXE}\" start"));
    w
}

pub(crate) fn omp_installed() -> World {
    world(&[(OMP_EXE, ""), (PS7_PROFILE, "Set-Alias ll ls\n")]) // PS5 profile does not exist
}

#[test]
fn catalog_has_the_five_phase1_tools() {
    let ids: Vec<String> = manifest::all().into_iter().map(|m| m.id).collect();
    for id in ["glazewm", "zebar", "windhawk", "windows-terminal", "oh-my-posh", "flow-launcher"] {
        assert!(ids.contains(&id.to_string()), "missing {id}");
        assert!(manifest::docs(id, "pt-BR").is_some() && manifest::docs(id, "en").is_some());
    }
}

/// Core acceptance test: force a failure at EVERY mutating step of a plan; the machine
/// must always end exactly as it started.
fn assert_rollback_everywhere(initial: World, m: &Manifest, action: Action) {
    let all = manifest::all();
    let probe = FakeOs::new(initial.clone());
    let plan = actions::plan(&probe, &all, m, action, false).expect("plan");
    let ok = engine::run(&probe, &plan);
    assert!(ok.ok, "{} {:?} should succeed: {:?}", m.id, action, ok.error);
    let total = probe.ticks();
    assert!(total > 0);

    for n in 0..total {
        let os = FakeOs::new(initial.clone());
        let plan = actions::plan(&os, &all, m, action, false).unwrap();
        *os.fail_at.lock().unwrap() = Some(n);
        let out = engine::run(&os, &plan);
        assert!(!out.ok && out.rolled_back, "{} {:?}: failure at op {n} must roll back", m.id, action);
        assert!(out.rollback_errors.is_empty(), "rollback errors at op {n}: {:?}", out.rollback_errors);
        assert_eq!(os.machine(), initial, "{} {:?}: machine changed after failure at op {n}", m.id, action);
    }
}

#[test]
fn activate_glazewm_rolls_back_from_any_step() {
    assert_rollback_everywhere(glaze_installed(), &tool("glazewm"), Action::Activate);
}

#[test]
fn deactivate_glazewm_rolls_back_from_any_step() {
    assert_rollback_everywhere(glaze_active(), &tool("glazewm"), Action::Deactivate);
}

#[test]
fn activate_oh_my_posh_rolls_back_from_any_step() {
    assert_rollback_everywhere(omp_installed(), &tool("oh-my-posh"), Action::Activate);
}

#[test]
fn install_and_uninstall_roll_back_from_any_step() {
    assert_rollback_everywhere(World::default(), &tool("flow-launcher"), Action::Install);
    let mut installed = world(&[(r"%LOCALAPPDATA%\FlowLauncher\Flow.Launcher.exe", "")]);
    installed.winget.insert("Flow-Launcher.Flow-Launcher".into());
    assert_rollback_everywhere(installed, &tool("flow-launcher"), Action::Uninstall);
}

#[test]
fn activate_really_changes_the_machine() {
    let os = FakeOs::new(glaze_installed());
    let all = manifest::all();
    let plan = actions::plan(&os, &all, &tool("glazewm"), Action::Activate, false).unwrap();
    assert!(engine::run(&os, &plan).ok);
    let w = os.machine();
    assert!(w.processes.contains("glazewm.exe"));
    assert_eq!(w.run_keys["WinRice-glazewm"], format!("\"{GLAZE_EXE}\" start"));
    // the config was backed up before anything else
    assert!(os.world.lock().unwrap().files.keys().any(|k| k.starts_with("DATA\\backups\\originals")));
}

#[test]
fn a_tool_that_never_starts_is_rolled_back() {
    let os = FakeOs::new(glaze_installed());
    *os.spawn_is_dud.lock().unwrap() = true;
    let initial = os.machine();
    let plan = actions::plan(&os, &manifest::all(), &tool("glazewm"), Action::Activate, false).unwrap();
    let out = engine::run(&os, &plan);
    assert!(!out.ok && out.rolled_back);
    assert_eq!(out.error.as_deref(), Some("glazewm.exe did not start"));
    assert_eq!(os.machine(), initial);
}

#[test]
fn conflicting_tools_cannot_both_be_active() {
    let mut glaze = tool("glazewm");
    let mut other = tool("zebar");
    other.id = "komorebi".into();
    other.detect.process = Some("komorebi.exe".into());
    other.detect.paths = vec![r"C:\k\komorebi.exe".into()];
    glaze.conflicts = vec!["komorebi".into()];

    let mut w = glaze_installed();
    w.files.insert(r"C:\k\komorebi.exe".into(), "".into());
    w.processes.insert("komorebi.exe".into());
    let os = FakeOs::new(w);
    let err = actions::plan(&os, &[glaze.clone(), other], &glaze, Action::Activate, false).unwrap_err();
    assert_eq!(err, PlanError::Conflict(vec!["komorebi".into()]));
    assert_eq!(err.to_string(), "conflict:komorebi");
}

#[test]
fn tools_without_a_toggle_are_not_toggleable() {
    let os = FakeOs::new(world(&[(r"%LOCALAPPDATA%\Microsoft\WindowsApps\wt.exe", "")]));
    let wt = tool("windows-terminal");
    assert!(!actions::status(&os, &wt).toggleable);
    assert_eq!(actions::plan(&os, &[wt.clone()], &wt, Action::Activate, false).unwrap_err(), PlanError::NotToggleable);
}

#[test]
fn profile_block_is_idempotent_and_removable() {
    let original = "Set-Alias ll ls\n";
    let once = with_block(original, "oh-my-posh", "oh-my-posh init pwsh | Invoke-Expression");
    assert_eq!(with_block(&once, "oh-my-posh", "oh-my-posh init pwsh | Invoke-Expression"), once);
    assert_eq!(without_block(&once, "oh-my-posh"), original);
    assert_eq!(without_block(original, "oh-my-posh"), original);
}

#[test]
fn restore_original_puts_files_back_and_removes_created_ones() {
    let os = FakeOs::new(omp_installed());
    let initial = os.machine();
    let all = manifest::all();
    let omp = tool("oh-my-posh");

    let plan = actions::plan(&os, &all, &omp, Action::Activate, false).unwrap();
    assert!(engine::run(&os, &plan).ok);
    let profile = os.read_for_test(PS7_PROFILE);
    assert!(profile.contains("oh-my-posh init pwsh"));
    assert!(os.read_for_test(PS5_PROFILE).contains("oh-my-posh init pwsh")); // was created

    let restore = actions::plan_restore(&os, &all, &[], false);
    assert!(engine::run(&os, &restore).ok);
    assert_eq!(os.machine(), initial);
}

#[test]
fn restore_with_uninstall_removes_only_what_winrice_installed() {
    let mut w = glaze_active();
    w.winget.insert("glzr-io.glazewm".into());
    let os = FakeOs::new(w);
    let all = manifest::all();
    let plan = actions::plan_restore(&os, &all, &["glazewm".to_string()], true);
    assert!(engine::run(&os, &plan).ok);
    let after = os.machine();
    assert!(!after.winget.contains("glzr-io.glazewm"));
    assert!(after.run_keys.is_empty() && after.processes.is_empty());
}

#[test]
fn store_tracks_installs_and_history() {
    let s = Store::in_memory().unwrap();
    s.mark_installed("glazewm").unwrap();
    s.mark_installed("glazewm").unwrap();
    assert_eq!(s.installed_by_app(), vec!["glazewm".to_string()]);
    s.unmark_installed("glazewm").unwrap();
    assert!(s.installed_by_app().is_empty());
    s.log(1, "glazewm", "activate", "ok", None);
    s.log(2, "glazewm", "deactivate", "rolled_back", Some("boom"));
    let h = s.history(10);
    assert_eq!(h.len(), 2);
    assert_eq!(h[0].action, "deactivate");
}
