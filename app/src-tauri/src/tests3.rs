//! Phase 3 tests: themes (load, validate, apply, undo, import, export) and remote catalog.

use crate::engine::{self, Journal};
use crate::manifest::{self, Manifest};
use crate::net::{self, Http};
use crate::os::fake::{FakeOs, World};
use crate::tests::*;
use crate::theme::{self, Origin, ThemeEntry};
use crate::themeops;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn official(id: &str) -> ThemeEntry {
    theme::load_embedded().into_iter().find(|e| e.theme.id == id).unwrap_or_else(|| panic!("no theme {id}"))
}

/// A fake machine where every tool of the catalog can be installed by "winget".
fn machine(initial: World) -> FakeOs {
    let os = FakeOs::new(initial);
    for m in manifest::all() {
        if let (Some(id), Some(first)) = (&m.install.id, m.detect.paths.first()) {
            os.packages.lock().unwrap().insert(id.clone(), vec![first.clone()]);
        }
    }
    os
}

fn ids(e: &ThemeEntry) -> Vec<String> {
    e.theme.components.iter().map(|c| c.tool.clone()).collect()
}

fn entry_of(files: &[(&str, &str)]) -> Result<ThemeEntry, String> {
    let map: BTreeMap<String, Vec<u8>> = files.iter().map(|(k, v)| (k.to_string(), v.as_bytes().to_vec())).collect();
    theme::entry_from_files(map, Origin::Imported)
}

const BASE: &str = "id: my-theme\nname: My theme\nauthor: { name: Me }\nversion: 1.0.0\nlicense: MIT\ndescription: { en: x, pt-BR: y }\n";

fn theme_yaml(components: &str) -> String {
    format!("{BASE}components:\n{components}")
}

#[test]
fn official_themes_load_and_match_the_catalog() {
    let themes = theme::load_embedded();
    assert_eq!(themes.len(), 5);
    let tools = manifest::all();
    for e in &themes {
        theme::validate_catalog(&e.theme, &tools).unwrap_or_else(|err| panic!("{}: {err}", e.theme.id));
        assert!(e.theme.palette.is_some(), "{} has no palette", e.theme.id);
        assert!(e.theme.description.contains_key("pt-BR"));
    }
}

#[test]
fn applying_each_official_theme_changes_the_machine_and_undo_restores_it() {
    let all = manifest::all();
    for e in theme::load_embedded() {
        let os = machine(World::default());
        let initial = os.machine();
        let (out, journal) = themeops::apply(&os, &all, &e, &ids(&e));
        assert!(out.ok, "{}: {:?}", e.theme.id, out.error);
        assert_ne!(os.machine(), initial, "{} changed nothing", e.theme.id);

        // the journal survives a restart: store it as JSON, load it back, undo
        let restored = Journal::from_json(&journal.to_json()).expect("journal json");
        assert!(engine::revert(&os, &restored).is_empty());
        assert_eq!(os.machine(), initial, "{}: undo did not restore the machine", e.theme.id);
    }
}

#[test]
fn theme_apply_is_all_or_nothing_at_every_step() {
    let all = manifest::all();
    for id in ["catppuccin-dev", "macos-like", "tokyo-night-tiling"] {
        let e = official(id);
        let probe = machine(World::default());
        assert!(themeops::apply(&probe, &all, &e, &ids(&e)).0.ok);
        for n in 0..probe.ticks() {
            let os = machine(World::default());
            let initial = os.machine();
            *os.fail_at.lock().unwrap() = Some(n);
            let (out, _) = themeops::apply(&os, &all, &e, &ids(&e));
            assert!(!out.ok && out.rolled_back, "{id}: failure at op {n} must fail the whole theme");
            assert!(out.rollback_errors.is_empty(), "{id} op {n}: {:?}", out.rollback_errors);
            assert_eq!(os.machine(), initial, "{id}: machine changed after failure at op {n}");
        }
    }
}

#[test]
fn partial_apply_only_touches_selected_components() {
    let all = manifest::all();
    let e = official("catppuccin-dev");
    let os = machine(World::default());
    let (out, _) = themeops::apply(&os, &all, &e, &["zebar".to_string()]);
    assert!(out.ok);
    let w = os.machine();
    assert!(w.winget.contains("glzr-io.zebar"));
    assert!(!w.winget.contains("glzr-io.glazewm"));
    assert!(w.processes.contains("zebar.exe") && !w.processes.contains("glazewm.exe"));
}

#[test]
fn theme_settings_are_written_through_the_manifest_options() {
    let all = manifest::all();
    let e = official("catppuccin-dev");
    let os = machine(World::default());
    assert!(themeops::apply(&os, &all, &e, &["glazewm".to_string()]).0.ok);
    let cfg = os.read_for_test(GLAZE_CFG);
    assert!(cfg.contains("inner_gap: 12px"), "{cfg}");
}

#[test]
fn a_theme_turns_off_a_clashing_tool_that_was_on() {
    let all = manifest::all();
    let e = official("tokyo-night-tiling"); // komorebi clashes with glazewm
    let mut w = glaze_active();
    w.processes.insert("glazewm.exe".into());
    let os = machine(w.clone());
    let preview = themeops::preview(&os, &all, &e, &ids(&e)).unwrap();
    assert_eq!(preview.items.iter().find(|i| i.tool == "komorebi").unwrap().turn_off, vec!["GlazeWM".to_string()]);

    let (out, journal) = themeops::apply(&os, &all, &e, &ids(&e));
    assert!(out.ok, "{:?}", out.error);
    let after = os.machine();
    assert!(!after.processes.contains("glazewm.exe"), "GlazeWM was turned off");
    assert!(after.processes.contains("komorebi.exe"));
    assert!(engine::revert(&os, &journal).is_empty());
    assert_eq!(os.machine(), w);
}

#[test]
fn preview_describes_what_will_happen() {
    let all = manifest::all();
    let os = machine(glaze_installed());
    let p = themeops::preview(&os, &all, &official("catppuccin-dev"), &ids(&official("catppuccin-dev"))).unwrap();
    let glaze = p.items.iter().find(|i| i.tool == "glazewm").unwrap();
    assert!(!glaze.install && glaze.activate && glaze.settings == 1);
    let zebar = p.items.iter().find(|i| i.tool == "zebar").unwrap();
    assert!(zebar.install && zebar.activate);
    assert_eq!(p.items.iter().find(|i| i.tool == "windhawk").unwrap().mods_skipped, 1);
}

fn rich_theme() -> ThemeEntry {
    entry_of(&[
        ("theme.yaml", &theme_yaml(
            "  - tool: glazewm\n    config: configs/glazewm/config.yaml\n    required: true\n  - tool: zebar\n    config: configs/zebar/\n    required: false\n",
        ).replace("components:", "wallpaper: assets/wall.png\ncomponents:")),
        ("configs/glazewm/config.yaml", "gaps:\n  inner_gap: 4px\n"),
        ("configs/zebar/settings.json", "{\"a\":1}"),
        ("configs/zebar/widgets/bar.css", "body{}"),
        ("assets/wall.png", "PNGDATA"),
    ])
    .unwrap()
}

#[test]
fn theme_files_wallpaper_and_folders_are_applied_and_rolled_back() {
    let all = manifest::all();
    let e = rich_theme();
    let os = machine(World::default());
    let initial = os.machine();
    let probe = machine(World::default());
    let (out, journal) = themeops::apply(&probe, &all, &e, &ids(&e));
    assert!(out.ok, "{:?}", out.error);
    let apply_ticks = probe.ticks(); // before the undo adds its own
    assert_eq!(probe.read_for_test(GLAZE_CFG), "gaps:\n  inner_gap: 4px\n");
    let zebar_dir = r"%USERPROFILE%\.glzr\zebar";
    assert_eq!(probe.read_for_test(&format!("{zebar_dir}\\settings.json")), "{\"a\":1}");
    assert_eq!(probe.read_for_test(&format!("{zebar_dir}\\widgets\\bar.css")), "body{}");
    assert!(probe.world.lock().unwrap().wallpaper.as_deref().unwrap().ends_with("my-theme-wall.png"));
    assert!(engine::revert(&probe, &journal).is_empty());
    assert_eq!(probe.machine(), initial);
    assert!(probe.world.lock().unwrap().wallpaper.is_none(), "previous (no) wallpaper restored");

    for n in 0..apply_ticks {
        let os = machine(World::default());
        *os.fail_at.lock().unwrap() = Some(n);
        let (out, _) = themeops::apply(&os, &all, &e, &ids(&e));
        assert!(!out.ok, "op {n}");
        assert_eq!(os.machine(), initial, "op {n}");
        assert!(os.world.lock().unwrap().wallpaper.is_none(), "op {n}: wallpaper");
    }
    let _ = os;
}

#[test]
fn themes_can_never_carry_code_or_escape_their_folder() {
    let ok = theme_yaml("  - tool: glazewm\n");
    assert!(entry_of(&[("theme.yaml", &ok)]).is_ok());
    for bad in ["setup.exe", "run.ps1", "x.bat", "lib.dll", "run.cmd", "a.vbs", "a.js", "noext"] {
        let err = entry_of(&[("theme.yaml", &ok), (bad, "x")]).unwrap_err();
        assert!(err.starts_with("forbidden_file"), "{bad}: {err}");
    }
    assert!(entry_of(&[("theme.yaml", &ok), ("../evil.yaml", "x")]).unwrap_err().starts_with("unsafe_path"));
    assert!(entry_of(&[("theme.yaml", &ok), ("C:/x.yaml", "x")]).unwrap_err().starts_with("unsafe_path"));
    let traversal = theme_yaml("  - tool: glazewm\n    config: ../../secret.yaml\n");
    assert!(entry_of(&[("theme.yaml", &traversal)]).unwrap_err().starts_with("unsafe_path"));
    let missing = theme_yaml("  - tool: glazewm\n    config: nope.yaml\n");
    assert!(entry_of(&[("theme.yaml", &missing)]).unwrap_err().starts_with("missing_file"));
    let huge = "x".repeat(theme::MAX_THEME_BYTES + 1);
    assert_eq!(entry_of(&[("theme.yaml", &ok), ("big.yaml", &huge)]).unwrap_err(), "theme_too_large");
    let color = ok.replace("components:", "palette: { name: p, colors: { accent: red } }\ncomponents:");
    assert!(entry_of(&[("theme.yaml", &color)]).unwrap_err().starts_with("invalid_color"));
}

#[test]
fn catalog_validation_rejects_unknown_tools_options_and_clashes() {
    let tools = manifest::all();
    let check = |components: &str| {
        let e = entry_of(&[("theme.yaml", &theme_yaml(components))]).unwrap();
        theme::validate_catalog(&e.theme, &tools)
    };
    assert!(check("  - tool: glazewm\n").is_ok());
    assert_eq!(check("  - tool: not-a-tool\n").unwrap_err(), "unknown_tool:not-a-tool");
    assert!(check("  - tool: glazewm\n    settings: { nope: 1 }\n").unwrap_err().starts_with("unknown_setting"));
    assert!(check("  - tool: glazewm\n  - tool: komorebi\n").unwrap_err().starts_with("components_conflict"));
}

#[test]
fn save_current_setup_and_export_zip_roundtrip() {
    let all = manifest::all();
    let mut w = glaze_active();
    // oh-my-posh is on too: its PowerShell profile must NOT be copied into the theme
    w.files.insert(OMP_EXE.into(), String::new());
    w.files.insert(PS7_PROFILE.into(), "# >>> winrice:oh-my-posh >>>\nx\n# <<< winrice:oh-my-posh <<<\nsecret-token\n".into());
    let os = FakeOs::new(w);
    let e = themeops::save_current(&os, &all, "my-setup", "My setup").unwrap();
    assert_eq!(e.origin, Origin::Mine);
    assert!(ids(&e).contains(&"glazewm".to_string()));
    assert_eq!(e.files["configs/glazewm/config.yaml"], b"gaps: 10\n");
    assert!(e.files.keys().all(|k| !k.contains("PowerShell") && !k.contains("oh-my-posh")), "profile leaked: {:?}", e.files.keys());
    assert!(!String::from_utf8_lossy(&e.files["theme.yaml"]).contains("secret-token"));

    // the saved theme can be applied again on another machine
    let fresh = machine(World::default());
    assert!(themeops::apply(&fresh, &all, &e, &["glazewm".to_string()]).0.ok);
    assert_eq!(fresh.read_for_test(GLAZE_CFG), "gaps: 10\n");

    let zip_bytes = themeops::export_zip(&e).unwrap();
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes)).unwrap();
    let names: Vec<String> = (0..z.len()).map(|i| z.by_index(i).unwrap().name().to_string()).collect();
    assert!(names.contains(&"theme.yaml".to_string()) && names.contains(&"configs/glazewm/config.yaml".to_string()));

    let nothing = FakeOs::new(World::default());
    assert_eq!(themeops::save_current(&nothing, &all, "x", "x").unwrap_err(), "nothing_active");
}

// ---------------------------------------------------------------- network

struct FakeHttp(BTreeMap<String, Vec<u8>>);
impl Http for FakeHttp {
    fn get(&self, url: &str, max: usize) -> Result<Vec<u8>, String> {
        let b = self.0.get(url).cloned().ok_or_else(|| format!("404 {url}"))?;
        if b.len() > max { Err("too_large".into()) } else { Ok(b) }
    }
}

fn repo(files: &[(&str, &str)], truncated: bool) -> FakeHttp {
    let mut m = BTreeMap::new();
    let tree: Vec<serde_json::Value> =
        files.iter().map(|(p, c)| serde_json::json!({"path": p, "type": "blob", "size": c.len()})).collect();
    m.insert("https://api.github.com/repos/me/rice".to_string(), br#"{"default_branch":"main"}"#.to_vec());
    m.insert(
        "https://api.github.com/repos/me/rice/git/trees/main?recursive=1".to_string(),
        serde_json::to_vec(&serde_json::json!({"tree": tree, "truncated": truncated})).unwrap(),
    );
    for (p, c) in files {
        m.insert(format!("https://raw.githubusercontent.com/me/rice/main/{p}"), c.as_bytes().to_vec());
    }
    FakeHttp(m)
}

#[test]
fn github_urls_are_parsed_strictly() {
    let r = net::parse_github_url("https://github.com/me/rice").unwrap();
    assert_eq!((r.owner.as_str(), r.name.as_str(), r.branch, r.subdir.as_str()), ("me", "rice", None, ""));
    let r = net::parse_github_url("https://github.com/me/rice.git/").unwrap();
    assert_eq!(r.name, "rice");
    let r = net::parse_github_url("https://github.com/me/rice/tree/dev/themes/x").unwrap();
    assert_eq!((r.branch.as_deref(), r.subdir.as_str()), (Some("dev"), "themes/x"));
    for bad in ["http://github.com/me/rice", "https://evil.com/me/rice", "https://github.com/me", "https://github.com/me/rice/blob/main/x", "https://github.com/me/rice/tree/main/../x", "https://github.com/../rice"] {
        assert!(net::parse_github_url(bad).is_err(), "{bad}");
    }
}

#[test]
fn import_from_github_downloads_only_what_the_theme_references() {
    let yaml = theme_yaml("  - tool: glazewm\n    config: configs/glazewm/config.yaml\n");
    let http = repo(
        &[("theme.yaml", &yaml), ("configs/glazewm/config.yaml", "gaps: 1\n"), ("README.md", "hi"), ("configs/other/huge.yaml", "zzz")],
        false,
    );
    let e = net::import_github(&http, "https://github.com/me/rice", &manifest::all()).unwrap();
    assert_eq!(e.origin, Origin::Imported);
    let mut names: Vec<_> = e.files.keys().cloned().collect();
    names.sort();
    assert_eq!(names, vec!["configs/glazewm/config.yaml", "theme.yaml"]);

    // theme living in a sub folder
    let http = repo(&[("themes/a/theme.yaml", &yaml), ("themes/a/configs/glazewm/config.yaml", "gaps: 1\n")], false);
    assert!(net::import_github(&http, "https://github.com/me/rice/tree/main/themes/a", &manifest::all()).is_ok());
}

#[test]
fn import_rejects_dangerous_or_incomplete_repos() {
    let tools = manifest::all();
    let yaml = theme_yaml("  - tool: glazewm\n    config: configs\n");
    let exe = repo(&[("theme.yaml", &yaml), ("configs/run.exe", "MZ")], false);
    assert!(net::import_github(&exe, "https://github.com/me/rice", &tools).unwrap_err().starts_with("forbidden_file"));
    let big = repo(&[("theme.yaml", &yaml)], true);
    assert_eq!(net::import_github(&big, "https://github.com/me/rice", &tools).unwrap_err(), "repo_too_large");
    let none = repo(&[("README.md", "x")], false);
    assert!(net::import_github(&none, "https://github.com/me/rice", &tools).unwrap_err().starts_with("missing_file"));
    let unknown = repo(&[("theme.yaml", &theme_yaml("  - tool: nope\n"))], false);
    assert!(net::import_github(&unknown, "https://github.com/me/rice", &tools).unwrap_err().starts_with("unknown_tool"));
    assert!(net::import_github(&exe, "https://example.com/me/rice", &tools).is_err());
    assert_eq!(net::RealHttp.get("https://evil.example/x", 10).unwrap_err(), "host_not_allowed");
    assert_eq!(net::RealHttp.get("http://raw.githubusercontent.com/x", 10).unwrap_err(), "host_not_allowed");
}

fn tmp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("winrice-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn index_json(entries: &[(&str, &str, &[(&str, &str)])]) -> Vec<u8> {
    let part = |kind: &str| -> Vec<serde_json::Value> {
        entries
            .iter()
            .filter(|(k, _, _)| *k == kind)
            .map(|(_, id, files)| {
                serde_json::json!({"id": id, "files": files.iter().map(|(p, c)| serde_json::json!({"path": p, "sha256": net::sha256_hex(c.as_bytes())})).collect::<Vec<_>>()})
            })
            .collect()
    };
    serde_json::to_vec(&serde_json::json!({"version": 1, "tools": part("tools"), "themes": part("themes")})).unwrap()
}

fn raw(kind: &str, id: &str, path: &str) -> String {
    format!("https://raw.githubusercontent.com/riceverse-apps/winrice/main/catalog/{kind}/{id}/{path}")
}

#[test]
fn remote_catalog_adds_tools_and_themes_with_verified_hashes() {
    let tool_yaml = std::fs::read_to_string("../../catalog/tools/zebar/manifest.yaml").unwrap().replace("id: zebar", "id: newtool").replace("name: Zebar", "name: New Tool");
    let theme_y = theme_yaml("  - tool: glazewm\n").replace("my-theme", "remote-theme");
    let tool_files = [("manifest.yaml", tool_yaml.as_str())];
    let theme_files = [("theme.yaml", theme_y.as_str())];
    let index = index_json(&[("tools", "newtool", &tool_files), ("themes", "remote-theme", &theme_files)]);

    let mut m = BTreeMap::new();
    m.insert(net::INDEX_URL.to_string(), index);
    m.insert(raw("tools", "newtool", "manifest.yaml"), tool_yaml.clone().into_bytes());
    m.insert(raw("themes", "remote-theme", "theme.yaml"), theme_y.clone().into_bytes());
    let http = FakeHttp(m);

    let root = tmp("sync");
    let s = net::sync_catalog(&http, &root, net::INDEX_URL).unwrap();
    assert_eq!((s.tools_updated.clone(), s.themes_updated.clone()), (vec!["newtool".to_string()], vec!["remote-theme".to_string()]));
    assert!(s.errors.is_empty(), "{:?}", s.errors);

    // downloaded content is picked up by the loaders
    assert!(manifest::all_with(&root).iter().any(|m| m.id == "newtool"));
    let themes = theme::load_dir(&root.join("catalog").join("themes"), Origin::Official);
    assert!(themes.iter().any(|t| t.theme.id == "remote-theme"));

    // second run: nothing changed, nothing downloaded
    let again = net::sync_catalog(&http, &root, net::INDEX_URL).unwrap();
    assert!(again.tools_updated.is_empty() && again.themes_updated.is_empty());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn remote_catalog_refuses_tampered_or_invalid_files() {
    let good = "id: x\n";
    let files = [("manifest.yaml", good)];
    let index = index_json(&[("tools", "evil", &files)]);
    let mut m = BTreeMap::new();
    m.insert(net::INDEX_URL.to_string(), index);
    // server returns different bytes than the index promised
    m.insert(raw("tools", "evil", "manifest.yaml"), b"id: tampered\n".to_vec());
    let root = tmp("tamper");
    let s = net::sync_catalog(&FakeHttp(m), &root, net::INDEX_URL).unwrap();
    assert!(s.tools_updated.is_empty());
    assert!(s.errors[0].contains("hash_mismatch"), "{:?}", s.errors);
    assert!(!root.join("catalog").join("tools").join("evil").exists(), "nothing was stored");

    // path traversal in the index is refused
    let bad = [("../../x.yaml", "a")];
    let index = index_json(&[("tools", "t", &bad)]);
    let mut m = BTreeMap::new();
    m.insert(net::INDEX_URL.to_string(), index);
    let s = net::sync_catalog(&FakeHttp(m), &root, net::INDEX_URL).unwrap();
    assert!(s.errors[0].contains("unsafe_path"), "{:?}", s.errors);

    // a manifest that is valid YAML but not a manifest is refused even with a correct hash
    let junk = [("manifest.yaml", "hello: world\n")];
    let index = index_json(&[("tools", "junk", &junk)]);
    let mut m = BTreeMap::new();
    m.insert(net::INDEX_URL.to_string(), index);
    m.insert(raw("tools", "junk", "manifest.yaml"), b"hello: world\n".to_vec());
    let s = net::sync_catalog(&FakeHttp(m), &root, net::INDEX_URL).unwrap();
    assert!(s.errors[0].contains("invalid_manifest"), "{:?}", s.errors);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn user_themes_roundtrip_on_disk_and_official_ones_win() {
    let root = tmp("disk");
    let e = rich_theme();
    theme::store_dir(&root, &e).unwrap();
    let loaded = theme::load_dir(&root, Origin::Imported);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].files, e.files);

    let mut clone = official("minimal-clean");
    clone.origin = Origin::Imported;
    clone.theme.name = "Impostor".into();
    let merged = theme::merge(vec![theme::load_embedded(), vec![clone]]);
    assert_eq!(merged.iter().filter(|t| t.theme.id == "minimal-clean").count(), 1);
    assert_eq!(merged.iter().find(|t| t.theme.id == "minimal-clean").unwrap().theme.name, "Minimal Clean");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn applied_theme_journal_is_stored_and_popped() {
    let s = crate::store::Store::in_memory().unwrap();
    assert!(s.current_theme().is_none());
    s.push_theme("a", 1, "[]");
    s.push_theme("b", 2, "[]");
    assert_eq!(s.current_theme().unwrap().0, "b");
    s.pop_theme();
    assert_eq!(s.current_theme().unwrap().0, "a");
    s.pop_theme();
    assert!(s.current_theme().is_none());
}

#[allow(dead_code)]
fn _unused(_: &Manifest) {}
