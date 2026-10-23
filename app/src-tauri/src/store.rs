//! Local state in SQLite: which tools WinRice installed, and the operation history.

use rusqlite::{params, Connection};
use serde::Serialize;

pub struct Store(Connection);

#[derive(Serialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub ts: i64,
    pub tool: String,
    pub action: String,
    pub status: String,
    pub detail: Option<String>,
    /// This entry can be undone now (it is the newest change and has undo information).
    pub undoable: bool,
    pub undone: bool,
}

impl Store {
    pub fn open(path: &str) -> rusqlite::Result<Self> {
        if let Some(dir) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        Self::init(Connection::open(path)?)
    }

    #[cfg(test)]
    pub fn in_memory() -> rusqlite::Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(c: Connection) -> rusqlite::Result<Self> {
        c.execute_batch(
            "CREATE TABLE IF NOT EXISTS installed_by_app (tool TEXT PRIMARY KEY);
             CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS theme_state (
               id INTEGER PRIMARY KEY AUTOINCREMENT, theme_id TEXT NOT NULL, applied_at INTEGER NOT NULL, journal TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS profiles (name TEXT PRIMARY KEY, tools TEXT NOT NULL, created INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS history (
               id INTEGER PRIMARY KEY AUTOINCREMENT, ts INTEGER NOT NULL, tool TEXT NOT NULL,
               action TEXT NOT NULL, status TEXT NOT NULL, detail TEXT);",
        )?;
        // Older databases do not have these columns yet; the error for "already there" is expected.
        let _ = c.execute("ALTER TABLE history ADD COLUMN journal TEXT", []);
        let _ = c.execute("ALTER TABLE history ADD COLUMN undone INTEGER NOT NULL DEFAULT 0", []);
        Ok(Self(c))
    }

    pub fn meta_get(&self, key: &str) -> Option<String> {
        self.0.query_row("SELECT value FROM meta WHERE key = ?1", params![key], |r| r.get(0)).ok()
    }

    pub fn meta_del(&self, key: &str) {
        let _ = self.0.execute("DELETE FROM meta WHERE key = ?1", params![key]);
    }

    pub fn save_profile(&self, name: &str, tools: &[String], ts: u64) {
        let json = serde_json::to_string(tools).unwrap_or_else(|_| "[]".into());
        let _ = self.0.execute("INSERT OR REPLACE INTO profiles (name, tools, created) VALUES (?1, ?2, ?3)", params![name, json, ts as i64]);
    }

    pub fn delete_profile(&self, name: &str) {
        let _ = self.0.execute("DELETE FROM profiles WHERE name = ?1", params![name]);
    }

    pub fn profiles(&self) -> Vec<(String, Vec<String>)> {
        let Ok(mut st) = self.0.prepare("SELECT name, tools FROM profiles ORDER BY created, name") else { return vec![] };
        st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map(|rows| rows.filter_map(Result::ok).map(|(n, t)| (n, serde_json::from_str(&t).unwrap_or_default())).collect())
            .unwrap_or_default()
    }

    pub fn meta_set(&self, key: &str, value: &str) {
        let _ = self.0.execute("INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)", params![key, value]);
    }

    /// Remembers an applied theme with the journal needed to undo it (survives restarts).
    pub fn push_theme(&self, theme_id: &str, ts: u64, journal: &str) {
        let _ = self.0.execute("INSERT INTO theme_state (theme_id, applied_at, journal) VALUES (?1, ?2, ?3)", params![theme_id, ts as i64, journal]);
    }

    /// The most recently applied theme that was not undone: (theme id, journal).
    pub fn current_theme(&self) -> Option<(String, String)> {
        self.0
            .query_row("SELECT theme_id, journal FROM theme_state ORDER BY id DESC LIMIT 1", [], |r| Ok((r.get(0)?, r.get(1)?)))
            .ok()
    }

    pub fn pop_theme(&self) {
        let _ = self.0.execute("DELETE FROM theme_state WHERE id = (SELECT MAX(id) FROM theme_state)", []);
    }

    pub fn mark_installed(&self, tool: &str) -> rusqlite::Result<()> {
        self.0.execute("INSERT OR IGNORE INTO installed_by_app (tool) VALUES (?1)", params![tool]).map(|_| ())
    }

    pub fn unmark_installed(&self, tool: &str) -> rusqlite::Result<()> {
        self.0.execute("DELETE FROM installed_by_app WHERE tool = ?1", params![tool]).map(|_| ())
    }

    pub fn installed_by_app(&self) -> Vec<String> {
        let Ok(mut st) = self.0.prepare("SELECT tool FROM installed_by_app") else { return vec![] };
        st.query_map([], |r| r.get(0)).map(|rows| rows.filter_map(Result::ok).collect()).unwrap_or_default()
    }

    pub fn log(&self, ts: u64, tool: &str, action: &str, status: &str, detail: Option<&str>) {
        self.log_j(ts, tool, action, status, detail, None);
    }

    /// Logs an operation together with the information needed to undo it later. Returns its id.
    pub fn log_j(&self, ts: u64, tool: &str, action: &str, status: &str, detail: Option<&str>, journal: Option<&str>) -> i64 {
        let _ = self.0.execute(
            "INSERT INTO history (ts, tool, action, status, detail, journal) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![ts as i64, tool, action, status, detail, journal],
        );
        self.0.last_insert_rowid()
    }

    pub fn mark_undone(&self, id: i64) {
        let _ = self.0.execute("UPDATE history SET undone = 1 WHERE id = ?1", params![id]);
    }

    pub fn journal_of(&self, id: i64) -> Option<String> {
        self.0.query_row("SELECT journal FROM history WHERE id = ?1", params![id], |r| r.get::<_, Option<String>>(0)).ok().flatten()
    }

    /// Newest first. Only the newest state-changing entry that kept undo information can be
    /// undone: undoing an older one would overwrite what newer changes did.
    pub fn history(&self, limit: i64) -> Vec<HistoryEntry> {
        let Ok(mut st) = self.0.prepare(
            "SELECT id, ts, tool, action, status, detail, journal IS NOT NULL, undone FROM history ORDER BY id DESC LIMIT ?1",
        ) else {
            return vec![];
        };
        let rows: Vec<(HistoryEntry, bool)> = st
            .query_map(params![limit], |r| {
                let e = HistoryEntry {
                    id: r.get(0)?,
                    ts: r.get(1)?,
                    tool: r.get(2)?,
                    action: r.get(3)?,
                    status: r.get(4)?,
                    detail: r.get(5)?,
                    undoable: false,
                    undone: r.get::<_, i64>(7)? != 0,
                };
                Ok((e, r.get::<_, bool>(6)?))
            })
            .map(|rows| rows.filter_map(Result::ok).collect())
            .unwrap_or_default();

        let mut blocked = false;
        rows.into_iter()
            .map(|(mut e, has_journal)| {
                let informational = e.action == "keep" || e.action == "undo" || e.action == "theme_undo" || e.action.starts_with("revert_") || e.action.starts_with("quick_");
                if !e.undone && e.status == "ok" && !informational {
                    e.undoable = has_journal && !blocked;
                    blocked = true;
                }
                e
            })
            .collect()
    }
}
