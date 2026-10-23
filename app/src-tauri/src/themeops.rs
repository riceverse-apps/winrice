//! Applying, previewing, saving and exporting themes.
//!
//! Applying a theme is ONE logical transaction made of several engine runs (install a
//! tool, write its config, activate it, ...). Each step is planned just in time, because
//! what to do depends on the state left by the previous one (e.g. the program only exists
//! after it is installed). If anything fails, every run done so far is reverted.

use crate::actions::{self, Action};
use crate::engine::{self, Journal, Outcome, Plan, Step};
use crate::manifest::Manifest;
use crate::os::Os;
use crate::settings;
use crate::theme::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Write;

#[derive(Debug, Serialize)]
pub struct PreviewItem {
    pub tool: String,
    pub name: String,
    pub selected: bool,
    pub required: bool,
    pub install: bool,
    pub config_files: usize,
    pub settings: usize,
    pub activate: bool,
    /// Tools that are on now and clash with this one; they get turned off.
    pub turn_off: Vec<String>,
    pub font: Option<String>,
    pub mods_skipped: usize,
    pub incompatible: bool,
}

#[derive(Debug, Serialize)]
pub struct ThemePreview {
    pub theme_id: String,
    pub items: Vec<PreviewItem>,
    pub wallpaper: bool,
}

fn parent_dir(path: &str) -> String {
    path.rsplit_once('\\').map(|(d, _)| d.to_string()).unwrap_or_default()
}

/// Write steps that copy a component's config from the theme over the tool's config.
/// A single file replaces the tool's main config; a folder is copied next to it.
pub fn config_steps(entry: &ThemeEntry, c: &Component, m: &Manifest) -> Result<Vec<Step>, String> {
    let Some(cfg) = &c.config else { return Ok(vec![]) };
    let cfg = cfg.trim_end_matches('/');
    let first = m.config.as_ref().and_then(|x| x.paths.first()).ok_or_else(|| format!("tool_without_config:{}", m.id))?;
    let files = files_under(&entry.files, cfg);
    let single = files.len() == 1 && files[0].0.as_str() == cfg;
    files
        .into_iter()
        .map(|(k, bytes)| {
            let content = String::from_utf8(bytes.clone()).map_err(|_| format!("binary_config:{k}"))?;
            let file = if single {
                first.clone()
            } else {
                format!("{}\\{}", parent_dir(first), k[cfg.len() + 1..].replace('/', "\\"))
            };
            Ok(Step::WriteConfig { tool: m.id.clone(), file, content })
        })
        .collect()
}

pub fn preview(os: &dyn Os, all: &[Manifest], entry: &ThemeEntry, selected: &[String]) -> Result<ThemePreview, String> {
    let mut items = vec![];
    for c in &entry.theme.components {
        let m = all.iter().find(|m| m.id == c.tool).ok_or_else(|| format!("unknown_tool:{}", c.tool))?;
        let st = actions::status(os, m);
        let sel = selected.contains(&c.tool);
        let turn_off = all
            .iter()
            .filter(|o| o.id != m.id && (m.conflicts.contains(&o.id) || o.conflicts.contains(&m.id)))
            .filter(|o| actions::status(os, o).active && !selected.contains(&o.id))
            .map(|o| o.name.clone())
            .collect();
        items.push(PreviewItem {
            tool: c.tool.clone(),
            name: m.name.clone(),
            selected: sel,
            required: c.required,
            install: !st.installed,
            config_files: config_steps(entry, c, m)?.len(),
            settings: c.settings.len(),
            activate: st.toggleable && !st.active,
            turn_off,
            font: c.font.clone(),
            mods_skipped: c.mods.len(),
            incompatible: !st.compatible,
        });
    }
    Ok(ThemePreview { theme_id: entry.theme.id.clone(), items, wallpaper: entry.theme.wallpaper.is_some() })
}

fn phase(os: &dyn Os, plan: Plan, journal: &mut Journal, applied: &mut usize) -> Result<(), String> {
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

fn plan_of(tool: &str, action: &str, steps: Vec<Step>) -> Plan {
    Plan { tool: tool.into(), action: action.into(), steps }
}

fn apply_inner(os: &dyn Os, all: &[Manifest], entry: &ThemeEntry, selected: &[String], j: &mut Journal, n: &mut usize) -> Result<(), String> {
    for c in entry.theme.components.iter().filter(|c| selected.contains(&c.tool)) {
        let m = all.iter().find(|m| m.id == c.tool).ok_or_else(|| format!("unknown_tool:{}", c.tool))?;
        if !actions::status(os, m).compatible {
            if c.required {
                return Err("incompatible".into());
            }
            continue;
        }
        if !actions::status(os, m).installed {
            let p = actions::plan(os, all, m, Action::Install, false).map_err(|e| e.to_string())?;
            phase(os, p, j, n)?;
        }
        phase(os, plan_of(&m.id, "theme_config", config_steps(entry, c, m)?), j, n)?;
        for (key, value) in &c.settings {
            phase(os, settings::plan_set(os, m, key, value.clone())?, j, n)?;
        }
        let st = actions::status(os, m);
        if st.toggleable && !st.active {
            // Tools that clash with this one are turned off as part of the same transaction.
            let p = actions::plan(os, all, m, Action::Activate, true).map_err(|e| e.to_string())?;
            phase(os, p, j, n)?;
        }
    }
    if let Some(w) = &entry.theme.wallpaper {
        let bytes = entry.files.get(w).ok_or_else(|| format!("missing_file:{w}"))?.clone();
        let name = w.rsplit('/').next().unwrap_or(w).to_string();
        let step = Step::SetWallpaper { theme: entry.theme.id.clone(), name, bytes };
        phase(os, plan_of(&entry.theme.id, "theme_wallpaper", vec![step]), j, n)?;
    }
    Ok(())
}

/// Applies the selected components. All-or-nothing: on failure everything is reverted.
pub fn apply(os: &dyn Os, all: &[Manifest], entry: &ThemeEntry, selected: &[String]) -> (Outcome, Journal) {
    let mut journal = Journal::default();
    let mut applied = 0;
    match apply_inner(os, all, entry, selected, &mut journal, &mut applied) {
        Ok(()) => (Outcome { ok: true, applied, error: None, rolled_back: false, rollback_errors: vec![] }, journal),
        Err(error) => {
            let rollback_errors = engine::revert(os, &journal);
            (Outcome { ok: false, applied, error: Some(error), rolled_back: true, rollback_errors }, Journal::default())
        }
    }
}

const SAVE_TEXT_EXT: &[&str] = &["yaml", "yml", "json", "toml", "css", "ini"];

/// "Save current setup as theme": the config of every tool that is on right now.
/// PowerShell profiles are never copied (they often hold personal data).
pub fn save_current(os: &dyn Os, all: &[Manifest], id: &str, name: &str) -> Result<ThemeEntry, String> {
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut components = vec![];
    for m in all.iter().filter(|m| actions::status(os, m).active) {
        let mut cfg_rel = None;
        let private = m.activate.as_ref().is_some_and(|a| a.profile_snippet.is_some());
        if let (false, Some(cfg)) = (private, &m.config) {
            let found = cfg.paths.iter().map(|p| os.expand(p)).find_map(|p| os.read(&p).ok().flatten().map(|t| (p, t)));
            if let Some((path, text)) = found {
                let base = path.rsplit('\\').next().unwrap_or(&path).to_string();
                let ext = base.rsplit('.').next().unwrap_or("").to_lowercase();
                if SAVE_TEXT_EXT.contains(&ext.as_str()) && text.len() <= 512 * 1024 {
                    let rel = format!("configs/{}/{}", m.id, base);
                    files.insert(rel.clone(), text.into_bytes());
                    cfg_rel = Some(rel);
                }
            }
        }
        components.push(Component { tool: m.id.clone(), config: cfg_rel, font: None, required: false, mods: vec![], settings: BTreeMap::new() });
    }
    if components.is_empty() {
        return Err("nothing_active".into());
    }
    let mut description = BTreeMap::new();
    description.insert("en".to_string(), "My setup, saved from WinRice.".to_string());
    description.insert("pt-BR".to_string(), "Meu setup, salvo pelo WinRice.".to_string());
    let theme = Theme {
        id: id.into(),
        name: name.into(),
        author: Author { name: "me".into(), github: None },
        version: "1.0.0".into(),
        license: "Personal use".into(),
        description,
        tags: vec!["mine".into()],
        difficulty: None,
        requires: None,
        palette: None,
        components,
        wallpaper: None,
        media: None,
    };
    files.insert("theme.yaml".into(), serde_yaml::to_string(&theme).map_err(|e| e.to_string())?.into_bytes());
    let entry = entry_from_files(files, Origin::Mine)?;
    validate_catalog(&entry.theme, all)?;
    Ok(entry)
}

pub fn export_zip(entry: &ThemeEntry) -> Result<Vec<u8>, String> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut cursor);
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (path, bytes) in &entry.files {
            z.start_file(path.as_str(), opts).map_err(|e| e.to_string())?;
            z.write_all(bytes).map_err(|e| e.to_string())?;
        }
        z.finish().map_err(|e| e.to_string())?;
    }
    Ok(cursor.into_inner())
}

// ------------------------------------------------------------------ palette sync

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PaletteChange {
    pub tool: String,
    pub name: String,
    pub key: String,
    pub role: String,
    pub color: String,
}

/// Color options of installed tools that map to a role of the palette.
pub fn palette_changes(os: &dyn Os, all: &[Manifest], palette: &Palette) -> Vec<PaletteChange> {
    let mut out = vec![];
    for m in all.iter().filter(|m| actions::status(os, m).installed) {
        for s in &m.settings {
            let Some(role) = &s.palette else { continue };
            if let Some(color) = palette.colors.get(role) {
                out.push(PaletteChange { tool: m.id.clone(), name: m.name.clone(), key: s.key.clone(), role: role.clone(), color: color.clone() });
            }
        }
    }
    out
}

/// Recolors the setup with the palette of a theme, without touching anything else.
/// One transaction: if any option fails, the ones already changed go back.
pub fn apply_palette(os: &dyn Os, all: &[Manifest], palette: &Palette) -> (Outcome, Journal, Vec<PaletteChange>) {
    let changes = palette_changes(os, all, palette);
    let (mut journal, mut applied) = (Journal::default(), 0);
    let result: Result<(), String> = changes.iter().try_for_each(|c| {
        let m = all.iter().find(|m| m.id == c.tool).ok_or("unknown_tool")?;
        phase(os, settings::plan_set(os, m, &c.key, serde_json::Value::String(c.color.clone()))?, &mut journal, &mut applied)
    });
    match result {
        Ok(()) => (Outcome { ok: true, applied, error: None, rolled_back: false, rollback_errors: vec![] }, journal, changes),
        Err(error) => {
            let rollback_errors = engine::revert(os, &journal);
            (Outcome { ok: false, applied, error: Some(error), rolled_back: true, rollback_errors }, Journal::default(), vec![])
        }
    }
}

// ------------------------------------------------------------------ send a theme to the official library

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Issue {
    pub code: String,
    /// "error" blocks the submission, "warning" is advice.
    pub level: &'static str,
}

/// Licenses accepted by the official library (open source / open content).
pub const OPEN_LICENSES: [&str; 14] = [
    "MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "MPL-2.0", "GPL-3.0", "GPL-3.0-only", "GPL-3.0-or-later", "LGPL-3.0",
    "CC0-1.0", "CC-BY-4.0", "CC-BY-SA-4.0", "Unlicense",
];

/// What the official-library CI would complain about, found before opening a pull request.
pub fn submission_issues(entry: &ThemeEntry, tools: &[Manifest], official_ids: &[String]) -> Vec<Issue> {
    let t = &entry.theme;
    let mut out = vec![];
    let mut add = |code: &str, level: &'static str| out.push(Issue { code: code.into(), level });
    if official_ids.contains(&t.id) {
        add("id_taken", "error");
    }
    if t.author.github.as_deref().map_or(true, |g| g.trim().is_empty()) {
        add("author_github_missing", "error");
    }
    if !OPEN_LICENSES.contains(&t.license.as_str()) {
        add("license_not_open", "error");
    }
    if ["en", "pt-BR"].iter().any(|l| t.description.get(*l).map_or(true, |d| d.trim().is_empty())) {
        add("description_lang_missing", "error");
    }
    let cover_ok = t.media.as_ref().and_then(|m| m.cover.as_ref()).is_some_and(|c| entry.files.contains_key(c));
    if !cover_ok {
        add("missing_cover", "error");
    }
    if validate_catalog(t, tools).is_err() {
        add("tools_invalid", "error");
    }
    if t.palette.is_none() {
        add("no_palette", "warning");
    }
    if t.components.iter().any(|c| c.config.is_some()) {
        add("review_configs", "warning");
    }
    out
}

#[derive(Debug, Clone, Deserialize)]
pub struct MetaEdit {
    pub name: String,
    pub license: String,
    pub github: String,
    pub description_en: String,
    pub description_pt: String,
}

fn rebuild(entry: &ThemeEntry, theme: Theme, mut files: BTreeMap<String, Vec<u8>>) -> Result<ThemeEntry, String> {
    files.insert("theme.yaml".into(), serde_yaml::to_string(&theme).map_err(|e| e.to_string())?.into_bytes());
    entry_from_files(files, entry.origin)
}

/// Updates the name, license, author and description of a theme the user owns.
pub fn edit_meta(entry: &ThemeEntry, meta: &MetaEdit) -> Result<ThemeEntry, String> {
    let mut t = entry.theme.clone();
    t.name = meta.name.trim().to_string();
    t.license = meta.license.trim().to_string();
    let gh = meta.github.trim().trim_start_matches('@').to_string();
    t.author.github = (!gh.is_empty()).then_some(gh);
    t.description.insert("en".into(), meta.description_en.trim().to_string());
    t.description.insert("pt-BR".into(), meta.description_pt.trim().to_string());
    if t.name.is_empty() {
        return Err("invalid_theme_name".into());
    }
    rebuild(entry, t, entry.files.clone())
}

pub const MAX_COVER_BYTES: usize = 2 * 1024 * 1024;

/// Adds (or replaces) the cover screenshot.
pub fn set_cover(entry: &ThemeEntry, ext: &str, bytes: Vec<u8>) -> Result<ThemeEntry, String> {
    let ext = ext.to_lowercase();
    if !["png", "jpg", "jpeg", "webp"].contains(&ext.as_str()) {
        return Err(format!("forbidden_file:cover.{ext}"));
    }
    if bytes.len() > MAX_COVER_BYTES {
        return Err("cover_too_large".into());
    }
    let mut t = entry.theme.clone();
    let mut files = entry.files.clone();
    files.retain(|k, _| !k.starts_with("screenshots/cover."));
    let rel = format!("screenshots/cover.{ext}");
    files.insert(rel.clone(), bytes);
    let media = t.media.get_or_insert_with(Media::default);
    media.cover = Some(rel.clone());
    if !media.screenshots.contains(&rel) {
        media.screenshots.insert(0, rel);
    }
    rebuild(entry, t, files)
}

fn pull_request_text(entry: &ThemeEntry) -> String {
    let t = &entry.theme;
    let tools: Vec<&str> = t.components.iter().map(|c| c.tool.as_str()).collect();
    format!(
        "# Add theme: {name}\n\n\
         **Title for the pull request:** `Add theme: {name}`\n\n\
         ## What it is\n\n{en}\n\n\
         ## Details\n\n\
         - Theme id: `{id}`\n- Author: @{gh}\n- License: {license}\n- Tools: {tools}\n- Difficulty: {diff}\n\n\
         ## Checklist\n\n\
         - [x] The theme contains only data files (no executables or scripts)\n\
         - [x] It has an open-source license and I am allowed to share every file in it\n\
         - [x] Descriptions are written in English and Portuguese (pt-BR)\n\
         - [x] A cover screenshot is included\n\
         - [ ] I tested applying and undoing it on a clean Windows 11 VM\n\
         - [ ] The config files contain no personal data (usernames, paths, tokens)\n",
        name = t.name,
        en = t.description.get("en").cloned().unwrap_or_default(),
        id = t.id,
        gh = t.author.github.clone().unwrap_or_default(),
        license = t.license,
        tools = tools.join(", "),
        diff = t.difficulty.clone().unwrap_or_else(|| "beginner".into()),
    )
}

/// Writes the theme exactly as the repository expects it (`catalog/themes/<id>/...`) plus a
/// ready-to-paste pull request text, in a folder the user can upload from. Returns the folder.
pub fn prepare_submission(os: &dyn Os, entry: &ThemeEntry) -> Result<String, String> {
    let dir = format!("{}\\Documents\\WinRice\\submit\\{}", os.expand("%USERPROFILE%"), entry.theme.id);
    for (rel, bytes) in &entry.files {
        let path = format!("{dir}\\catalog\\themes\\{}\\{}", entry.theme.id, rel.replace('/', "\\"));
        os.write_bytes(&path, bytes).map_err(|e| format!("{path}: {e}"))?;
    }
    let pr = format!("{dir}\\PULL_REQUEST.md");
    os.write_bytes(&pr, pull_request_text(entry).as_bytes()).map_err(|e| format!("{pr}: {e}"))?;
    Ok(dir)
}
