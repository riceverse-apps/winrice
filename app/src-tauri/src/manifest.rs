//! Tool manifest model (see 03-manifesto-de-ferramentas.md) and the embedded catalog.

use include_dir::{include_dir, Dir};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type I18n = BTreeMap<String, String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub category: String,
    pub summary: I18n,
    pub difficulty: String,
    pub homepage: String,
    pub license: String,
    pub requires: Requires,
    pub install: Install,
    #[serde(default)]
    pub uninstall: Option<Uninstall>,
    pub detect: Detect,
    #[serde(default)]
    pub activate: Option<Activate>,
    #[serde(default)]
    pub deactivate: Option<Deactivate>,
    #[serde(default)]
    pub config: Option<Config>,
    #[serde(default)]
    pub conflicts: Vec<String>,
    #[serde(default)]
    pub pairs_well_with: Vec<String>,
    #[serde(default)]
    pub changes_on_screen: Vec<I18n>,
    #[serde(default)]
    pub shortcuts: Vec<Shortcut>,
    #[serde(default)]
    pub risks: Option<I18n>,
    #[serde(default)]
    pub how_to_undo: Option<I18n>,
    #[serde(default)]
    pub settings: Vec<Setting>,
    #[serde(default)]
    pub widget: Option<WidgetCfg>,
    /// Paid software (trial); flagged in the UI.
    #[serde(default)]
    pub paid: bool,
    /// Free to use but with restrictions (e.g. no commercial use); flagged in the UI.
    #[serde(default)]
    pub restricted_license: bool,
}

/// How a tool shows up in the quick-access widget.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WidgetCfg {
    #[serde(default)]
    pub show_by_default: bool,
    #[serde(default)]
    pub quick_actions: Vec<QuickAction>,
}

/// Extra button next to the toggle (e.g. "Reload config"). The command comes from the
/// official manifest, never from a theme.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickAction {
    pub id: String,
    pub label: I18n,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub label: I18n,
    #[serde(default)]
    pub default: serde_json::Value,
    /// Appended to numbers when written (e.g. "px"), stripped when read.
    #[serde(default)]
    pub suffix: Option<String>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    /// For color options: the palette role (accent, base, text, ...) that "Sync palette" writes here.
    #[serde(default)]
    pub palette: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requires {
    pub windows_min_build: u32,
    #[serde(default)]
    pub windows_max_build: Option<u32>,
    #[serde(default)]
    pub admin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Install {
    pub method: String,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Uninstall {
    pub method: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Detect {
    #[serde(default)]
    pub process: Option<String>,
    /// MSIX/Appx package names, for tools that are installed as Store-style packages.
    #[serde(default)]
    pub appx: Vec<String>,
    #[serde(default)]
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSnippet {
    pub files: Vec<String>,
    pub content: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Activate {
    #[serde(default)]
    pub start: Option<String>,
    /// For MSIX tools: start the app by its package (`explorer.exe shell:AppsFolder\<family>!<app>`).
    /// Takes precedence over `start`.
    #[serde(default)]
    pub start_appx: Option<AppxStart>,
    /// A scheduled task that starts the tool at sign-in on its own (full path, e.g. `\Seelen\Seelen UI Service`).
    /// Turning the tool on enables it, turning it off disables it, so it does not come back after a restart.
    /// Changing it may need administrator permission, which WinRice asks for.
    #[serde(default)]
    pub autostart_task: Option<String>,
    #[serde(default)]
    pub autostart: bool,
    #[serde(default)]
    pub profile_snippet: Option<ProfileSnippet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppxStart {
    /// Package name, as in `detect.appx`.
    pub name: String,
    /// Application id inside the package manifest (usually `App`).
    pub app: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Deactivate {
    #[serde(default)]
    pub stop: Option<String>,
    #[serde(default)]
    pub remove_autostart: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub backup: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shortcut {
    pub keys: String,
    #[serde(flatten)]
    pub text: I18n,
}

static CATALOG: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../catalog/tools");

/// All manifests bundled with the app, sorted by id. Invalid manifests are skipped
/// (CI validates the catalog, so this only guards against a broken local edit).
pub fn all() -> Vec<Manifest> {
    let mut out: Vec<Manifest> = CATALOG
        .dirs()
        .filter_map(|d| d.get_file(d.path().join("manifest.yaml")))
        .filter_map(|f| serde_yaml::from_slice(f.contents()).ok())
        .collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// Embedded manifests plus the ones synced from the remote catalog (`<root>\catalog\tools`),
/// which win when the id exists in both.
pub fn all_with(root: &std::path::Path) -> Vec<Manifest> {
    let mut out = all();
    let dir = root.join("catalog").join("tools");
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.filter_map(Result::ok) {
            let parsed = std::fs::read(e.path().join("manifest.yaml")).ok().and_then(|b| serde_yaml::from_slice::<Manifest>(&b).ok());
            if let Some(m) = parsed {
                out.retain(|o| o.id != m.id);
                out.push(m);
            }
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// Like `docs`, preferring files from the synced catalog.
pub fn docs_with(root: &std::path::Path, id: &str, lang: &str) -> Option<String> {
    let dir = root.join("catalog").join("tools").join(id).join("docs");
    [lang, "en"]
        .iter()
        .find_map(|l| std::fs::read_to_string(dir.join(format!("{l}.md"))).ok())
        .or_else(|| docs(id, lang))
}

/// Long documentation for a tool; falls back to English when the language is missing.
pub fn docs(id: &str, lang: &str) -> Option<String> {
    let dir = CATALOG.get_dir(id)?;
    let get = |l: &str| {
        dir.get_file(dir.path().join("docs").join(format!("{l}.md")))
            .and_then(|f| f.contents_utf8())
            .map(str::to_owned)
    };
    get(lang).or_else(|| get("en"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlossaryTerm {
    pub id: String,
    pub term: I18n,
    pub definition: I18n,
}

static GLOSSARY: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../catalog/glossary");

/// Plain-language explanations of jargon (tiling, Nerd Font, ...), shown as tooltips.
pub fn glossary() -> Vec<GlossaryTerm> {
    GLOSSARY
        .get_file("glossary.yaml")
        .and_then(|f| serde_yaml::from_slice(f.contents()).ok())
        .unwrap_or_default()
}
