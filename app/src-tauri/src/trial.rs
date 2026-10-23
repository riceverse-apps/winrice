//! State behind "try mode" and the Explorer watchdog.
//!
//! The last successful operation keeps its undo journal in a single slot. Three things
//! can consume it: the 30 s try-mode timer, the Explorer watchdog and the emergency
//! shortcut. Whoever takes the slot first reverts; everyone else finds it empty.

use crate::engine::Journal;
use std::sync::Mutex;

pub const TRY_SECONDS: u64 = 30;
/// Explorer restarting this many times while watched counts as a crash loop.
pub const WATCHDOG_MAX_RESTARTS: u32 = 3;
pub const WATCHDOG_POLLS: u32 = 30; // x 2 s = 60 s
pub const WATCHDOG_INTERVAL_MS: u64 = 2000;

pub struct Slot {
    pub token: u64,
    pub tool: String,
    pub action: String,
    pub journal: Journal,
    /// History row of the operation, so reverting it can mark that row as undone.
    pub history_id: Option<i64>,
    /// True until the user presses "Keep" (try mode only).
    pub try_pending: bool,
}

#[derive(Default)]
pub struct LastOp {
    slot: Mutex<Option<Slot>>,
    counter: Mutex<u64>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Who {
    /// Any holder of the slot (emergency shortcut).
    Anyone,
    /// Only if this token is still current (timers and watchdog of one operation).
    Token(u64),
}

impl LastOp {
    pub fn store(&self, tool: &str, action: &str, journal: Journal, try_pending: bool, history_id: Option<i64>) -> u64 {
        let mut c = self.counter.lock().unwrap();
        *c += 1;
        let token = *c;
        *self.slot.lock().unwrap() = Some(Slot { token, tool: tool.into(), action: action.into(), journal, history_id, try_pending });
        token
    }

    /// Confirms a try ("Keep changes"). False when the slot is gone or already reverted.
    pub fn keep(&self, token: u64) -> bool {
        match self.slot.lock().unwrap().as_mut() {
            Some(s) if s.token == token && s.try_pending => {
                s.try_pending = false;
                true
            }
            _ => false,
        }
    }

    /// Takes the slot to revert it. `only_if_try_pending` is used by the try timer so a
    /// confirmed change is never reverted.
    pub fn take(&self, who: Who, only_if_try_pending: bool) -> Option<Slot> {
        let mut g = self.slot.lock().unwrap();
        let ok = match (&*g, who) {
            (Some(s), Who::Token(t)) => s.token == t && (!only_if_try_pending || s.try_pending),
            (Some(s), Who::Anyone) => !only_if_try_pending || s.try_pending,
            (None, _) => false,
        };
        if ok { g.take() } else { None }
    }

    /// Forgets the slot if it belongs to this history row (the user undid it from the history).
    pub fn drop_history(&self, id: i64) {
        let mut g = self.slot.lock().unwrap();
        if g.as_ref().is_some_and(|s| s.history_id == Some(id)) {
            *g = None;
        }
    }

    pub fn is_current(&self, token: u64) -> bool {
        self.slot.lock().unwrap().as_ref().is_some_and(|s| s.token == token)
    }
}

/// Detects a crash loop from successive Explorer process-id snapshots.
#[derive(Default)]
pub struct Watchdog {
    last: Option<Vec<u32>>,
    restarts: u32,
}

impl Watchdog {
    /// Feed one snapshot; returns true once Explorer restarted too many times.
    pub fn observe(&mut self, mut pids: Vec<u32>) -> bool {
        pids.sort_unstable();
        if let Some(prev) = &self.last {
            if *prev != pids && !pids.is_empty() {
                self.restarts += 1;
            }
        }
        if !pids.is_empty() {
            self.last = Some(pids);
        }
        self.restarts >= WATCHDOG_MAX_RESTARTS
    }
}
