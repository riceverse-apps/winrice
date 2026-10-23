//! Profiles (a named set of tools that are on), "Pause customizations" and quick actions.
//!
//! Switching profiles, pausing and resuming all go through the same function, so they
//! share one guarantee: it is a single transaction, and "pause then resume" puts back
//! exactly the same set of tools.

use crate::actions::{self, Action};
use crate::engine::{self, Journal, Outcome, Plan};
use crate::manifest::Manifest;
use crate::os::Os;

pub struct Switched {
    pub outcome: Outcome,
    /// Undo information (used by tests; the UI undoes a switch with another switch).
    #[cfg_attr(not(test), allow(dead_code))]
    pub journal: Journal,
    /// Tools in the target that could not be turned on (not installed, no toggle, incompatible).
    pub skipped: Vec<String>,
}

fn run_phase(os: &dyn Os, plan: Plan, journal: &mut Journal, applied: &mut usize) -> Result<(), String> {
    if plan.steps.is_empty() {
        return Ok(());
    }
    let (out, j) = engine::run_journal(os, &plan);
    if out.ok {
        *applied += out.applied;
        journal.extend(j);
        Ok(())
    } else {
        Err(out.error.unwrap_or_default())
    }
}

/// Tools that are on right now and can be switched.
pub fn active_tools(os: &dyn Os, all: &[Manifest]) -> Vec<String> {
    all.iter().filter(|m| actions::status(os, m).active).map(|m| m.id.clone()).collect()
}

fn switch_inner(os: &dyn Os, all: &[Manifest], target: &[String], j: &mut Journal, n: &mut usize, skipped: &mut Vec<String>) -> Result<(), String> {
    // 1) turn off what should not be on, so nothing clashes later
    for m in all.iter().filter(|m| !target.contains(&m.id)) {
        let st = actions::status(os, m);
        if st.toggleable && st.active {
            let p = actions::plan(os, all, m, Action::Deactivate, false).map_err(|e| e.to_string())?;
            run_phase(os, p, j, n)?;
        }
    }
    // 2) turn on what should be on
    for id in target {
        let Some(m) = all.iter().find(|m| &m.id == id) else {
            skipped.push(id.clone());
            continue;
        };
        let st = actions::status(os, m);
        if !st.installed || !st.toggleable || !st.compatible {
            skipped.push(id.clone());
            continue;
        }
        if !st.active {
            let p = actions::plan(os, all, m, Action::Activate, true).map_err(|e| e.to_string())?;
            run_phase(os, p, j, n)?;
        }
    }
    Ok(())
}

/// Makes exactly `target` the set of tools that are on. All or nothing.
pub fn switch(os: &dyn Os, all: &[Manifest], target: &[String]) -> Switched {
    let (mut journal, mut applied, mut skipped) = (Journal::default(), 0, vec![]);
    match switch_inner(os, all, target, &mut journal, &mut applied, &mut skipped) {
        Ok(()) => Switched {
            outcome: Outcome { ok: true, applied, error: None, rolled_back: false, rollback_errors: vec![] },
            journal,
            skipped,
        },
        Err(error) => {
            let rollback_errors = engine::revert(os, &journal);
            Switched {
                outcome: Outcome { ok: false, applied, error: Some(error), rolled_back: true, rollback_errors },
                journal: Journal::default(),
                skipped,
            }
        }
    }
}

/// Resolves a manifest quick action (`widget.quick_actions`) to a program and arguments.
pub fn quick_action_command(os: &dyn Os, m: &Manifest, action_id: &str) -> Result<(String, Vec<String>), String> {
    let qa = m
        .widget
        .as_ref()
        .and_then(|w| w.quick_actions.iter().find(|q| q.id == action_id))
        .ok_or("unknown_action")?;
    let mut parts = qa.command.split_whitespace();
    let prog = parts.next().ok_or("unknown_action")?.to_string();
    let args = parts.map(str::to_string).collect();
    let full = m
        .detect
        .paths
        .iter()
        .map(|p| os.expand(p))
        .find(|p| os.exists(p) && p.rsplit(['\\', '/']).next().is_some_and(|f| f.eq_ignore_ascii_case(&prog)));
    Ok((full.unwrap_or(prog), args))
}
