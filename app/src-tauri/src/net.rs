//! Network access: import a theme from GitHub, and sync the remote catalog.
//!
//! Only GitHub hosts are reachable, only over HTTPS, with size limits. Everything that
//! comes in is validated before it is stored, and catalog files are checked against
//! the SHA-256 listed in the index.

use crate::manifest::Manifest;
use crate::theme::{self, Origin, ThemeEntry, MAX_THEME_BYTES};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

pub const INDEX_URL: &str = "https://raw.githubusercontent.com/riceverse-apps/winrice/main/catalog/index.json";
const RAW_BASE: &str = "https://raw.githubusercontent.com/riceverse-apps/winrice/main/catalog";

pub trait Http: Send + Sync {
    fn get(&self, url: &str, max_bytes: usize) -> Result<Vec<u8>, String>;
}

fn allowed(url: &str) -> bool {
    url.starts_with("https://raw.githubusercontent.com/") || url.starts_with("https://api.github.com/")
}

pub struct RealHttp;

impl Http for RealHttp {
    fn get(&self, url: &str, max_bytes: usize) -> Result<Vec<u8>, String> {
        if !allowed(url) {
            return Err("host_not_allowed".into());
        }
        let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(20)).user_agent("WinRice").build();
        let resp = agent.get(url).call().map_err(|e| format!("network:{e}"))?;
        let mut buf = Vec::new();
        resp.into_reader().take(max_bytes as u64 + 1).read_to_end(&mut buf).map_err(|e| format!("network:{e}"))?;
        if buf.len() > max_bytes {
            return Err("too_large".into());
        }
        Ok(buf)
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

// ---------------------------------------------------------------- import from GitHub

#[derive(Debug, PartialEq)]
pub struct Repo {
    pub owner: String,
    pub name: String,
    pub branch: Option<String>,
    pub subdir: String,
}

/// `https://github.com/owner/repo[/tree/branch[/sub/dir]]`
pub fn parse_github_url(url: &str) -> Result<Repo, String> {
    let rest = url.trim().trim_end_matches('/').strip_prefix("https://github.com/").ok_or("not_a_github_url")?;
    let parts: Vec<&str> = rest.split('/').collect();
    let ok = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c)) && s != "..";
    if parts.len() < 2 || !ok(parts[0]) || !ok(parts[1].trim_end_matches(".git")) {
        return Err("not_a_github_url".into());
    }
    let (branch, subdir) = match parts.get(2) {
        Some(&"tree") if parts.len() >= 4 => (Some(parts[3].to_string()), parts[4..].join("/")),
        None => (None, String::new()),
        _ => return Err("not_a_github_url".into()),
    };
    if !subdir.is_empty() && !theme::safe_rel(&subdir) {
        return Err("not_a_github_url".into());
    }
    Ok(Repo { owner: parts[0].into(), name: parts[1].trim_end_matches(".git").into(), branch, subdir })
}

#[derive(Deserialize)]
struct Tree {
    tree: Vec<TreeItem>,
    #[serde(default)]
    truncated: bool,
}
#[derive(Deserialize)]
struct TreeItem {
    path: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    size: usize,
}

fn join(sub: &str, rel: &str) -> String {
    if sub.is_empty() { rel.to_string() } else { format!("{sub}/{rel}") }
}

/// Downloads `theme.yaml` plus only the files it references (configs, wallpaper),
/// validates everything and returns an *unverified* imported theme.
pub fn import_github(http: &dyn Http, url: &str, tools: &[Manifest]) -> Result<ThemeEntry, String> {
    let repo = parse_github_url(url)?;
    let (o, r) = (&repo.owner, &repo.name);
    let branch = match &repo.branch {
        Some(b) => b.clone(),
        None => {
            let raw = http.get(&format!("https://api.github.com/repos/{o}/{r}"), 256 * 1024)?;
            let v: serde_json::Value = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
            v["default_branch"].as_str().ok_or("repo_not_found")?.to_string()
        }
    };
    let tree: Tree = serde_json::from_slice(&http.get(&format!("https://api.github.com/repos/{o}/{r}/git/trees/{branch}?recursive=1"), 4 * 1024 * 1024)?)
        .map_err(|e| e.to_string())?;
    if tree.truncated {
        return Err("repo_too_large".into());
    }
    let sizes: BTreeMap<&str, usize> = tree.tree.iter().filter(|t| t.kind == "blob").map(|t| (t.path.as_str(), t.size)).collect();
    let fetch = |rel: &str| -> Result<Vec<u8>, String> {
        let path = join(&repo.subdir, rel);
        let size = *sizes.get(path.as_str()).ok_or_else(|| format!("missing_file:{rel}"))?;
        if size > MAX_THEME_BYTES {
            return Err("theme_too_large".into());
        }
        http.get(&format!("https://raw.githubusercontent.com/{o}/{r}/{branch}/{}", path.replace(' ', "%20")), MAX_THEME_BYTES)
    };

    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    files.insert("theme.yaml".into(), fetch("theme.yaml")?);
    let manifest = theme::parse(&files)?;

    let base = join(&repo.subdir, "");
    let mut wanted: Vec<String> = vec![];
    for c in &manifest.components {
        if let Some(cfg) = &c.config {
            if !theme::safe_rel(cfg) {
                return Err(format!("unsafe_path:{cfg}"));
            }
            let cfg = cfg.trim_end_matches('/');
            let abs = join(&repo.subdir, cfg);
            let dir = format!("{abs}/");
            wanted.extend(sizes.keys().filter(|k| **k == abs || k.starts_with(&dir)).map(|k| k[base.len()..].to_string()));
        }
    }
    if let Some(w) = &manifest.wallpaper {
        wanted.push(w.clone());
    }
    wanted.sort();
    wanted.dedup();
    let mut total = 0;
    for rel in wanted {
        let bytes = fetch(&rel)?;
        total += bytes.len();
        if total > MAX_THEME_BYTES {
            return Err("theme_too_large".into());
        }
        files.insert(rel, bytes);
    }
    let entry = theme::entry_from_files(files, Origin::Imported)?;
    theme::validate_catalog(&entry.theme, tools)?;
    Ok(entry)
}

// ---------------------------------------------------------------- remote catalog

#[derive(Deserialize, Serialize, Debug)]
pub struct IndexFile {
    pub path: String,
    pub sha256: String,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct IndexEntry {
    pub id: String,
    pub files: Vec<IndexFile>,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct Index {
    pub version: u32,
    pub tools: Vec<IndexEntry>,
    pub themes: Vec<IndexEntry>,
}

#[derive(Debug, Default, Serialize, PartialEq)]
pub struct SyncSummary {
    pub tools_updated: Vec<String>,
    pub themes_updated: Vec<String>,
    pub errors: Vec<String>,
}

fn up_to_date(dir: &Path, e: &IndexEntry) -> bool {
    e.files.iter().all(|f| std::fs::read(dir.join(&f.path)).map(|b| sha256_hex(&b) == f.sha256).unwrap_or(false))
}

fn download(http: &dyn Http, kind: &str, e: &IndexEntry) -> Result<BTreeMap<String, Vec<u8>>, String> {
    if !theme::safe_rel(&e.id) {
        return Err(format!("unsafe_path:{}", e.id));
    }
    let mut files = BTreeMap::new();
    let mut total = 0;
    for f in &e.files {
        if !theme::safe_rel(&f.path) {
            return Err(format!("unsafe_path:{}", f.path));
        }
        let bytes = http.get(&format!("{RAW_BASE}/{kind}/{}/{}", e.id, f.path), MAX_THEME_BYTES)?;
        total += bytes.len();
        if total > MAX_THEME_BYTES {
            return Err("theme_too_large".into());
        }
        if sha256_hex(&bytes) != f.sha256 {
            return Err(format!("hash_mismatch:{}/{}", e.id, f.path));
        }
        files.insert(f.path.clone(), bytes);
    }
    Ok(files)
}

fn write_all(dir: &Path, files: &BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    let _ = std::fs::remove_dir_all(dir);
    for (rel, bytes) in files {
        let p = dir.join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&p, bytes).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Downloads new/changed tools and themes into `<root>\catalog\`. One bad entry never
/// blocks the others, and nothing is stored unless its hash and content validate.
pub fn sync_catalog(http: &dyn Http, root: &Path, index_url: &str) -> Result<SyncSummary, String> {
    let index: Index = serde_json::from_slice(&http.get(index_url, 1024 * 1024)?).map_err(|e| format!("invalid_index:{e}"))?;
    let mut summary = SyncSummary::default();

    for e in &index.tools {
        let dir = root.join("catalog").join("tools").join(&e.id);
        if theme::safe_rel(&e.id) && up_to_date(&dir, e) {
            continue;
        }
        let result = download(http, "tools", e).and_then(|files| {
            let raw = files.get("manifest.yaml").ok_or("missing_file:manifest.yaml")?;
            let m: Manifest = serde_yaml::from_slice(raw).map_err(|x| format!("invalid_manifest:{x}"))?;
            if m.id != e.id {
                return Err(format!("id_mismatch:{}", e.id));
            }
            write_all(&dir, &files)
        });
        match result {
            Ok(()) => summary.tools_updated.push(e.id.clone()),
            Err(err) => summary.errors.push(format!("tool {}: {err}", e.id)),
        }
    }
    for e in &index.themes {
        let dir = root.join("catalog").join("themes").join(&e.id);
        if theme::safe_rel(&e.id) && up_to_date(&dir, e) {
            continue;
        }
        let result = download(http, "themes", e).and_then(|files| {
            let entry = theme::entry_from_files(files, Origin::Official)?;
            if entry.theme.id != e.id {
                return Err(format!("id_mismatch:{}", e.id));
            }
            write_all(&dir, &entry.files)
        });
        match result {
            Ok(()) => summary.themes_updated.push(e.id.clone()),
            Err(err) => summary.errors.push(format!("theme {}: {err}", e.id)),
        }
    }
    Ok(summary)
}

pub const REPO_URL: &str = "https://github.com/riceverse-apps/winrice";

/// The only links the app opens in the browser: pages of the project repository.
pub fn repo_url_allowed(url: &str) -> bool {
    url == REPO_URL
        || url
            .strip_prefix(REPO_URL)
            .is_some_and(|rest| rest.starts_with('/') && !rest.contains(char::is_whitespace) && !rest.contains(".."))
}
