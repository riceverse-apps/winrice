//! Administrator helper and scheduled-task autostart (Seelen UI).

use crate::actions::{self, Action};
use crate::elevated::{self, Command};
use crate::engine::{self, Step};
use crate::manifest;
use crate::os::fake::{FakeOs, World};
use crate::tests::*;
use std::cell::Cell;

const TASK: &str = r"\Seelen\Seelen UI Service";

fn world(task: Option<bool>) -> World {
    let mut w = World::default();
    w.appx.insert("Seelen.SeelenUI".into());
    if let Some(t) = task {
        w.tasks.insert(TASK.into(), t);
    }
    w
}

fn os(w: World) -> FakeOs {
    let os = FakeOs::new(w);
    os.appx_launch.lock().unwrap().insert("Seelen.SeelenUI_fakepub".into(), "seelen-ui.exe".into());
    os
}

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

#[test]
fn turning_seelen_on_enables_its_task_and_undoing_puts_it_back() {
    let all = manifest::all();
    let fake = os(world(Some(false))); // the user had disabled it
    let plan = actions::plan(&fake, &all, &tool("seelen-ui"), Action::Activate, false).unwrap();
    assert!(plan.steps.iter().any(|s| matches!(s, Step::SetTask { enabled: true, name, .. } if name == TASK)));

    let (out, journal) = engine::run_journal(&fake, &plan);
    assert!(out.ok, "{:?}", out.error);
    assert!(fake.machine().tasks[TASK]);

    assert!(engine::revert(&fake, &journal).is_empty());
    assert_eq!(fake.machine(), world(Some(false)), "task, process and everything else back as before");
}

#[test]
fn turning_seelen_off_disables_its_task_so_it_stays_off_after_a_restart() {
    let all = manifest::all();
    let mut w = world(Some(true));
    w.processes.insert("seelen-ui.exe".into());
    w.processes.insert("slu-service.exe".into());
    let fake = os(w);
    let plan = actions::plan(&fake, &all, &tool("seelen-ui"), Action::Deactivate, false).unwrap();
    assert!(plan.steps.iter().any(|s| matches!(s, Step::SetTask { enabled: false, .. })));
    assert!(engine::run(&fake, &plan).ok);
    assert!(!fake.machine().tasks[TASK]);
    assert!(!fake.machine().processes.contains("seelen-ui.exe"));
}

#[test]
fn task_changes_are_all_or_nothing_in_both_directions() {
    let all = manifest::all();
    // on
    let probe = os(world(Some(false)));
    let on = actions::plan(&probe, &all, &tool("seelen-ui"), Action::Activate, false).unwrap();
    assert!(engine::run(&probe, &on).ok);
    for n in 0..probe.ticks() {
        let f = os(world(Some(false)));
        *f.fail_at.lock().unwrap() = Some(n);
        let out = engine::run(&f, &on);
        assert!(!out.ok && out.rolled_back, "on, op {n}");
        assert_eq!(f.machine(), world(Some(false)), "on, op {n}");
    }
    // off
    let mut running = world(Some(true));
    running.processes.insert("seelen-ui.exe".into());
    let probe = os(running.clone());
    let off = actions::plan(&probe, &all, &tool("seelen-ui"), Action::Deactivate, false).unwrap();
    assert!(engine::run(&probe, &off).ok);
    for n in 0..probe.ticks() {
        let f = os(running.clone());
        *f.fail_at.lock().unwrap() = Some(n);
        let out = engine::run(&f, &off);
        if !out.ok {
            assert!(f.machine().tasks[TASK], "off, op {n}: the task is back");
        }
    }
}

#[test]
fn saying_no_to_the_administrator_prompt_cancels_everything_cleanly() {
    let all = manifest::all();
    let f = os(world(Some(false)));
    *f.uac_denied.lock().unwrap() = true;
    let plan = actions::plan(&f, &all, &tool("seelen-ui"), Action::Activate, false).unwrap();
    let out = engine::run(&f, &plan);
    assert!(!out.ok && out.rolled_back);
    assert_eq!(out.error.as_deref(), Some("elevation_denied"));
    assert_eq!(f.machine(), world(Some(false)), "nothing was left half done");
}

#[test]
fn no_prompt_when_the_task_is_already_in_the_wanted_state() {
    let all = manifest::all();
    let f = os(world(Some(true)));
    *f.uac_denied.lock().unwrap() = true; // would fail if the helper were needed
    let plan = actions::plan(&f, &all, &tool("seelen-ui"), Action::Activate, false).unwrap();
    assert!(engine::run(&f, &plan).ok, "enabled already: no elevation, no prompt");
}

#[test]
fn a_task_that_does_not_exist_is_just_skipped() {
    let all = manifest::all();
    let f = os(world(None));
    let plan = actions::plan(&f, &all, &tool("seelen-ui"), Action::Activate, false).unwrap();
    assert!(engine::run(&f, &plan).ok);
    assert!(f.machine().tasks.is_empty());
}

// ---- the elevated helper itself

#[test]
fn the_helper_understands_only_one_command_with_strict_arguments() {
    assert_eq!(elevated::parse(&s(&["--elevated", "task-set", TASK, "0"])), Some(Command::TaskSet { name: TASK.into(), enabled: false }));
    assert_eq!(elevated::parse(&s(&["--elevated", "task-set", TASK, "1"])), Some(Command::TaskSet { name: TASK.into(), enabled: true }));
    for bad in [
        vec!["--elevated"],
        vec!["--elevated", "task-set", TASK],
        vec!["--elevated", "task-set", TASK, "2"],
        vec!["--elevated", "task-set", TASK, "yes"],
        vec!["--elevated", "run", "cmd.exe", "0"],
        vec!["--elevated", "task-set", TASK, "0", "extra"],
        vec!["task-set", TASK, "0"],
    ] {
        assert_eq!(elevated::parse(&s(&bad)), None, "{bad:?}");
    }
}

#[test]
fn the_helper_only_touches_tasks_named_by_an_official_manifest() {
    assert!(elevated::allowed_task(TASK));
    for other in [r"\Microsoft\Windows\Defrag\ScheduledDefrag", r"\Seelen\Other", "", "*", r"\Seelen\Seelen UI Service /RU SYSTEM", r"\\"] {
        assert!(!elevated::allowed_task(other), "{other:?}");
    }

    let called = Cell::new(0);
    let change = |_: &str, _: bool| {
        called.set(called.get() + 1);
        true
    };
    // normal start of the app: not a helper call
    assert_eq!(elevated::dispatch(&s(&[]), change), None);
    assert_eq!(elevated::dispatch(&s(&["something-else"]), change), None);
    // allowed
    assert_eq!(elevated::dispatch(&s(&["--elevated", "task-set", TASK, "0"]), change), Some(0));
    assert_eq!(called.get(), 1);
    // refused without calling Windows at all
    assert_eq!(elevated::dispatch(&s(&["--elevated", "task-set", r"\Microsoft\Windows\Defrag\ScheduledDefrag", "0"]), change), Some(4));
    assert_eq!(elevated::dispatch(&s(&["--elevated", "bogus"]), change), Some(2));
    assert_eq!(called.get(), 1, "nothing outside the allow-list reached Windows");
    // Windows failing is reported
    assert_eq!(elevated::dispatch(&s(&["--elevated", "task-set", TASK, "1"]), |_: &str, _: bool| false), Some(3));
}
