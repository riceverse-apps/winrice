//! Backup Service: copies of every file WinRice may change.
//!
//! * `backups\originals\` keeps the FIRST version ever seen of each file (or the fact
//!   that it did not exist). "Restore original Windows" goes back to these.
//! * `backups\<timestamp>\` keeps a copy before each operation.

use crate::os::Os;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Record {
    path: String,
    content: Option<String>,
}

fn sanitize(path: &str) -> String {
    path.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect()
}

fn originals_dir(os: &dyn Os) -> String {
    format!("{}\\backups\\originals", os.data_dir())
}

pub fn backup_file(os: &dyn Os, path: &str) -> Result<(), String> {
    let content = os.read(path).map_err(|e| format!("{path}: {e}"))?;
    let record = serde_json::to_string(&Record { path: path.into(), content: content.clone() }).map_err(|e| e.to_string())?;
    let name = sanitize(path);

    let orig = format!("{}\\{name}.json", originals_dir(os));
    if os.read(&orig).map_err(|e| e.to_string())?.is_none() {
        os.write(&orig, &record).map_err(|e| format!("{orig}: {e}"))?;
    }
    if content.is_some() {
        let stamped = format!("{}\\backups\\{}\\{name}.json", os.data_dir(), os.now());
        os.write(&stamped, &record).map_err(|e| format!("{stamped}: {e}"))?;
    }
    Ok(())
}

/// Writes every original back. Returns `(path, content-before-restore)` for each file
/// touched so the caller can undo the restore.
pub fn restore_originals(os: &dyn Os) -> Result<Vec<(String, Option<String>)>, String> {
    let mut touched = Vec::new();
    for file in os.list_dir(&originals_dir(os)) {
        let Some(raw) = os.read(&file).map_err(|e| e.to_string())? else { continue };
        let rec: Record = serde_json::from_str(&raw).map_err(|e| format!("{file}: {e}"))?;
        let current = os.read(&rec.path).map_err(|e| format!("{}: {e}", rec.path))?;
        match &rec.content {
            Some(c) => os.write(&rec.path, c).map_err(|e| format!("{}: {e}", rec.path))?,
            None => os.remove_file(&rec.path).map_err(|e| format!("{}: {e}", rec.path))?,
        }
        touched.push((rec.path, current));
    }
    Ok(touched)
}
