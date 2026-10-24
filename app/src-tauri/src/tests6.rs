//! Regression tests for the double-install bug: winget reports "already installed" with a
//! non-zero exit code, which must not be treated as a failure nor undone later.

use crate::actions::{self, Action, PlanError};
use crate::engine::{self, Plan, Step};
use crate::manifest;
use crate::os::fake::{FakeOs, World};
use crate::os::Os;
use crate::tests::*;

fn install_step() -> Step {
    Step::WingetInstall { tool: "glazewm".into(), id: "glzr-io.glazewm".into() }
}

fn installed_world() -> World {
    let mut w = glaze_installed();
    w.winget.insert("glzr-io.glazewm".into());
    w
}

#[test]
fn installing_something_already_installed_succeeds() {
    let os = FakeOs::new(installed_world());
    let plan = Plan { tool: "glazewm".into(), action: "install".into(), steps: vec![install_step()] };
    let out = engine::run(&os, &plan);
    assert!(out.ok, "{:?}", out.error);
    assert!(os.machine().winget.contains("glzr-io.glazewm"));
}

#[test]
fn undoing_never_removes_a_package_winrice_did_not_install() {
    let mut w = installed_world();
    w.run_keys.clear();
    // step 1: "install" (already there), step 2 fails -> rollback must leave the package alone
    let plan = Plan {
        tool: "glazewm".into(),
        action: "install".into(),
        steps: vec![install_step(), Step::SetRunKey { tool: "glazewm".into(), name: "WinRice-glazewm".into(), command: "x".into() }],
    };
    let os = FakeOs::new(w.clone());
    *os.fail_at.lock().unwrap() = Some(1); // the run-key write
    let out = engine::run(&os, &plan);
    assert!(!out.ok && out.rolled_back);
    assert_eq!(os.machine(), w, "the pre-existing package is still installed");

    // and a journal kept from a successful "already installed" run does not uninstall it either
    let os = FakeOs::new(w.clone());
    let (ok, journal) = engine::run_journal(&os, &plan);
    assert!(ok.ok);
    assert!(engine::revert(&os, &journal).is_empty());
    assert!(os.machine().winget.contains("glzr-io.glazewm"), "undo must not uninstall what was already there");
}

#[test]
fn a_real_install_is_still_undone_on_rollback() {
    let os = FakeOs::new(World::default());
    os.packages.lock().unwrap().insert("glzr-io.glazewm".into(), vec![GLAZE_EXE.into()]);
    let plan = Plan {
        tool: "glazewm".into(),
        action: "install".into(),
        steps: vec![install_step(), Step::SetRunKey { tool: "glazewm".into(), name: "k".into(), command: "x".into() }],
    };
    *os.fail_at.lock().unwrap() = Some(1);
    let out = engine::run(&os, &plan);
    assert!(!out.ok && out.rolled_back);
    assert_eq!(os.machine(), World::default(), "what WinRice itself installed is removed again");
}

#[test]
fn a_second_install_request_is_refused_after_the_first_finished() {
    // Two requests planned one after the other under the same lock: the second must see the result of the first.
    let all = manifest::all();
    let glaze = tool("glazewm");
    let os = FakeOs::new(World::default());
    os.packages.lock().unwrap().insert("glzr-io.glazewm".into(), vec![GLAZE_EXE.into()]);

    let first = actions::plan(&os, &all, &glaze, Action::Install, false).unwrap();
    assert!(engine::run(&os, &first).ok);
    assert_eq!(actions::plan(&os, &all, &glaze, Action::Install, false).unwrap_err(), PlanError::AlreadyInstalled);
}

#[test]
fn msix_tools_are_detected_through_their_package() {
    let all = manifest::all();
    let seelen = tool("seelen-ui");
    assert_eq!(seelen.detect.appx, vec!["Seelen.SeelenUI".to_string()]);

    let os = FakeOs::new(World::default());
    assert!(!actions::status(&os, &seelen).installed);
    os.world.lock().unwrap().appx.insert("Seelen.SeelenUI".into());
    assert!(actions::status(&os, &seelen).installed, "installed MSIX package counts as installed");
    assert!(actions::plan(&os, &all, &seelen, Action::Install, false).is_err());
}

#[cfg(windows)]
#[test]
fn real_appx_lookup_does_not_invent_packages() {
    let os = crate::os::RealOs;
    assert!(!os.appx_installed("Definitely.Not.A.Package.123"));
}

// ---- MSIX apps are started through their package

fn seelen_world() -> World {
    let mut w = World::default();
    w.appx.insert("Seelen.SeelenUI".into());
    w
}

fn seelen_os(world: World) -> FakeOs {
    let os = FakeOs::new(world);
    os.appx_launch.lock().unwrap().insert("Seelen.SeelenUI_fakepub".into(), "seelen-ui.exe".into());
    os
}

#[test]
fn an_msix_tool_is_started_through_explorer_and_its_package_id() {
    let all = manifest::all();
    let seelen = tool("seelen-ui");
    let os = seelen_os(seelen_world());
    let plan = actions::plan(&os, &all, &seelen, Action::Activate, false).unwrap();
    let start = plan.steps.iter().find_map(|s| match s {
        Step::Start { program, args, .. } => Some((program.clone(), args.clone())),
        _ => None,
    });
    assert_eq!(start, Some(("explorer.exe".to_string(), vec![r"shell:AppsFolder\Seelen.SeelenUI_fakepub!App".to_string()])));
    // autostart uses the same command, so Windows starts it at sign-in the same way
    assert!(plan.steps.iter().any(|s| matches!(s, Step::SetRunKey { command, .. } if command.contains("shell:AppsFolder"))));

    assert!(engine::run(&os, &plan).ok);
    assert!(os.machine().processes.contains("seelen-ui.exe"), "the app really started");
}

#[test]
fn msix_activation_and_deactivation_roll_back_from_any_step() {
    let all = manifest::all();
    let seelen = tool("seelen-ui");
    // activate
    let probe = seelen_os(seelen_world());
    let plan = actions::plan(&probe, &all, &seelen, Action::Activate, false).unwrap();
    assert!(engine::run(&probe, &plan).ok);
    let on = probe.machine();
    for n in 0..probe.ticks() {
        let os = seelen_os(seelen_world());
        *os.fail_at.lock().unwrap() = Some(n);
        let out = engine::run(&os, &plan);
        assert!(!out.ok && out.rolled_back, "activate op {n}");
        assert_eq!(os.machine(), seelen_world(), "activate op {n}");
    }
    // deactivate (and the restart used to undo it goes through the package too)
    let off_plan = actions::plan(&probe, &all, &seelen, Action::Deactivate, false).unwrap();
    for n in 0..4 {
        let os = seelen_os(seelen_world());
        os.world.lock().unwrap().processes.insert("seelen-ui.exe".into());
        os.world.lock().unwrap().run_keys = on.run_keys.clone();
        let before = os.machine();
        *os.fail_at.lock().unwrap() = Some(n);
        let out = engine::run(&os, &off_plan);
        if !out.ok {
            assert_eq!(os.machine(), before, "deactivate op {n}");
        }
    }
}

#[test]
fn an_uninstalled_msix_tool_cannot_be_activated() {
    let os = seelen_os(World::default());
    // not installed: planning must say so instead of inventing a program that is not found
    assert_eq!(actions::plan(&os, &manifest::all(), &tool("seelen-ui"), Action::Activate, false).unwrap_err(), PlanError::NotInstalled);
}
