//! Theme model, Theme Loader and validation (see 04-biblioteca-de-temas.md).
//!
//! A theme is data only: YAML/JSON/TOML/CSS configs, images and a `theme.yaml`.
//! Anything executable is rejected, so a theme can never run code by itself.

use crate::manifest::{I18n, Manifest};
use include_dir::{include_dir, Dir};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

pub const MAX_THEME_BYTES: usize = 8 * 1024 * 1024;
const TEXT_EXT: &[&str] = &["yaml", "yml", "json", "jsonc", "toml", "css", "ini", "md", "txt"];
const IMAGE_EXT: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "bmp"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub id: String,
    pub name: String,
    pub author: Author,
    pub version: String,
    pub license: String,
    pub description: I18n,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub difficulty: Option<String>,
    #[serde(default)]
    pub requires: Option<ThemeRequires>,
    #[serde(default)]
    pub palette: Option<Palette>,
    pub components: Vec<Component>,
    #[serde(default)]
    pub wallpaper: Option<String>,
    #[serde(default)]
    pub media: Option<Media>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
    #[serde(default)]
    pub github: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeRequires {
    #[serde(default)]
    pub windows_min_build: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Palette {
    pub name: String,
    pub colors: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub tool: String,
    /// File or folder inside the theme, copied over the tool's config.
    #[serde(default)]
    pub config: Option<String>,
    #[serde(default)]
    pub font: Option<String>,
    #[serde(default)]
    pub required: bool,
    /// Windhawk mods. Mod management is not supported yet, so they are reported and skipped.
    #[serde(default)]
    pub mods: Vec<ModRef>,
    /// Values for the simple options declared in the tool manifest (`settings:`).
    #[serde(default)]
    pub settings: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModRef {
    pub id: String,
    #[serde(default)]
    pub settings: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Media {
    #[serde(default)]
    pub cover: Option<String>,
    #[serde(default)]
    pub screenshots: Vec<String>,
    #[serde(default)]
    pub demo: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    Official,
    Imported,
    Mine,
}

/// A theme together with all its files (themes are small, so they live in memory).
#[derive(Debug, Clone)]
pub struct ThemeEntry {
    pub theme: Theme,
    pub origin: Origin,
    /// Path inside the theme (forward slashes) -> bytes. Includes `theme.yaml`.
    pub files: BTreeMap<String, Vec<u8>>,
}

/// A relative path that stays inside the theme folder.
pub fn safe_rel(p: &str) -> bool {
    let p = p.strip_suffix('/').unwrap_or(p); // a folder may end with one slash
    !p.is_empty()
        && !p.starts_with(['/', '\\'])
        && !p.contains(':')
        && !p.split(['/', '\\']).any(|part| part == ".." || part.is_empty())
}

fn ext(p: &str) -> String {
    Path::new(p).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase()
}

pub fn is_image(p: &str) -> bool {
    IMAGE_EXT.contains(&ext(p).as_str())
}

/// Files under `rel` (a file or a folder) inside the theme.
pub fn files_under<'a>(files: &'a BTreeMap<String, Vec<u8>>, rel: &str) -> Vec<(&'a String, &'a Vec<u8>)> {
    let rel = rel.trim_end_matches('/');
    let prefix = format!("{rel}/");
    files.iter().filter(|(k, _)| k.as_str() == rel || k.starts_with(&prefix)).collect()
}

/// Structural validation: safe paths, only data files, size limit, references exist.
pub fn validate_files(theme: &Theme, files: &BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    let id_ok = !theme.id.is_empty()
        && theme.id.split('-').all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
    if !id_ok {
        return Err("invalid_theme_id".into());
    }
    if theme.components.is_empty() {
        return Err("theme_without_components".into());
    }
    let total: usize = files.values().map(Vec::len).sum();
    if total > MAX_THEME_BYTES {
        return Err("theme_too_large".into());
    }
    for path in files.keys() {
        if !safe_rel(path) {
            return Err(format!("unsafe_path:{path}"));
        }
        let e = ext(path);
        if !TEXT_EXT.contains(&e.as_str()) && !IMAGE_EXT.contains(&e.as_str()) {
            return Err(format!("forbidden_file:{path}"));
        }
    }
    for c in &theme.components {
        if let Some(cfg) = &c.config {
            if !safe_rel(cfg) {
                return Err(format!("unsafe_path:{cfg}"));
            }
            if files_under(files, cfg).is_empty() {
                return Err(format!("missing_file:{cfg}"));
            }
        }
    }
    if let Some(w) = &theme.wallpaper {
        if !safe_rel(w) || !is_image(w) {
            return Err(format!("unsafe_path:{w}"));
        }
        if !files.contains_key(w) {
            return Err(format!("missing_file:{w}"));
        }
    }
    if let Some(p) = &theme.palette {
        for v in p.colors.values() {
            let ok = v.len() == 7 && v.starts_with('#') && v[1..].chars().all(|c| c.is_ascii_hexdigit());
            if !ok {
                return Err(format!("invalid_color:{v}"));
            }
        }
    }
    Ok(())
}

/// Validation against the tool catalog: known tools, valid options, no clashing tools.
pub fn validate_catalog(theme: &Theme, tools: &[Manifest]) -> Result<(), String> {
    for c in &theme.components {
        let m = tools.iter().find(|m| m.id == c.tool).ok_or_else(|| format!("unknown_tool:{}", c.tool))?;
        for key in c.settings.keys() {
            if !m.settings.iter().any(|s| &s.key == key) {
                return Err(format!("unknown_setting:{}:{key}", c.tool));
            }
        }
        if c.config.is_some() && m.config.as_ref().map_or(true, |cfg| cfg.paths.is_empty()) {
            return Err(format!("tool_without_config:{}", c.tool));
        }
        for other in theme.components.iter().filter(|o| o.tool != c.tool) {
            if m.conflicts.contains(&other.tool) {
                return Err(format!("components_conflict:{}:{}", c.tool, other.tool));
            }
        }
    }
    Ok(())
}

pub fn parse(files: &BTreeMap<String, Vec<u8>>) -> Result<Theme, String> {
    let raw = files.get("theme.yaml").ok_or("missing_file:theme.yaml")?;
    serde_yaml::from_slice(raw).map_err(|e| format!("invalid_theme:{e}"))
}

/// Builds an entry from raw files, running structural validation.
pub fn entry_from_files(files: BTreeMap<String, Vec<u8>>, origin: Origin) -> Result<ThemeEntry, String> {
    let theme = parse(&files)?;
    validate_files(&theme, &files)?;
    Ok(ThemeEntry { theme, origin, files })
}

static THEMES: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../catalog/themes");

fn collect(dir: &Dir, base: &str, out: &mut BTreeMap<String, Vec<u8>>) {
    for f in dir.files() {
        let rel = f.path().to_string_lossy().replace('\\', "/");
        if let Some(r) = rel.strip_prefix(base) {
            out.insert(r.trim_start_matches('/').to_string(), f.contents().to_vec());
        }
    }
    for d in dir.dirs() {
        collect(d, base, out);
    }
}

/// Themes bundled with the app.
pub fn load_embedded() -> Vec<ThemeEntry> {
    THEMES
        .dirs()
        .filter_map(|d| {
            let base = d.path().to_string_lossy().replace('\\', "/");
            let mut files = BTreeMap::new();
            collect(d, &base, &mut files);
            files.remove(".gitkeep");
            entry_from_files(files, Origin::Official).ok()
        })
        .collect()
}

fn read_tree(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.filter_map(Result::ok) {
        let p = e.path();
        if p.is_dir() {
            read_tree(root, &p, out);
        } else if let (Ok(rel), Ok(bytes)) = (p.strip_prefix(root), std::fs::read(&p)) {
            out.insert(rel.to_string_lossy().replace('\\', "/"), bytes);
        }
    }
}

/// Themes stored on disk: `<root>\<theme-id>\theme.yaml ...`.
pub fn load_dir(root: &Path, origin: Origin) -> Vec<ThemeEntry> {
    let Ok(rd) = std::fs::read_dir(root) else { return vec![] };
    rd.filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let mut files = BTreeMap::new();
            read_tree(&e.path(), &e.path(), &mut files);
            entry_from_files(files, origin).ok()
        })
        .collect()
}

pub fn store_dir(root: &Path, entry: &ThemeEntry) -> Result<(), String> {
    let dir = root.join(&entry.theme.id);
    let _ = std::fs::remove_dir_all(&dir);
    for (rel, bytes) in &entry.files {
        let p = dir.join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&p, bytes).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Official themes win over user themes with the same id; later sources never replace earlier ones.
pub fn merge(sources: Vec<Vec<ThemeEntry>>) -> Vec<ThemeEntry> {
    let mut out: Vec<ThemeEntry> = vec![];
    for e in sources.into_iter().flatten() {
        if !out.iter().any(|o| o.theme.id == e.theme.id) {
            out.push(e);
        }
    }
    out.sort_by(|a, b| a.theme.name.cmp(&b.theme.name));
    out
}
