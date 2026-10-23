//! Phase 4 tests: undoable history, palette sync, theme submission, glossary.

use crate::actions::{self, Action};
use crate::engine::{self, Journal};
use crate::manifest;
use crate::net;
use crate::os::fake::{FakeOs, World};
use crate::store::Store;
use crate::theme::{self, Origin};
use crate::themeops::{self, MetaEdit};
use crate::tests::*;

fn undoable(s: &Store) -> Vec<(String, bool)> {
    s.history(50).into_iter().map(|h| (h.action, h.undoable)).collect()
}

#[test]
fn only_the_newest_change_can_be_undone_and_older_ones_unlock_in_order() {
    let s = Store::in_memory().unwrap();
    let a = s.log_j(1, "glazewm", "activate", "ok", None, Some("[]"));
    let b = s.log_j(2, "zebar", "activate", "ok", None, Some("[]"));
    // newest first
    assert_eq!(undoable(&s), vec![("activate".into(), true), ("activate".into(), false)]);

    s.mark_undone(b);
    let h = s.history(50);
    assert!(h[0].undone && !h[0].undoable);
    assert!(h[1].undoable, "once the newer one is undone the older one unlocks");
    let _ = a;
}

#[test]
fn failed_informational_and_journal_less_entries_are_handled_correctly() {
    let s = Store::in_memory().unwrap();
    s.log_j(1, "glazewm", "activate", "ok", None, Some("[]"));
    s.log(2, "zebar", "activate", "rolled_back", Some("boom")); // failed: nothing to undo, does not block
    s.log(3, "-", "keep", "ok", None);
    s.log(4, "zebar", "revert_activate", "ok", None);
    s.log(5, "glazewm", "quick_reload-config", "ok", None);
    assert_eq!(undoable(&s).iter().filter(|(_, u)| *u).count(), 1, "the old activate is still the one to undo");

    // a state change without undo information (pause) protects what happened before it
    s.log(6, "all", "pause", "ok", None);
    assert!(undoable(&s).iter().all(|(_, u)| !u), "nothing before a pause can be undone out of order");
}

#[test]
fn history_migrates_old_databases() {
    let dir = std::env::temp_dir().join(format!("winrice-test-{}-migrate", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("state.db");
    {
        let c = rusqlite::Connection::open(&path).unwrap();
        c.execute_batch(
            "CREATE TABLE history (id INTEGER PRIMARY KEY AUTOINCREMENT, ts INTEGER NOT NULL, tool TEXT NOT NULL, action TEXT NOT NULL, status TEXT NOT NULL, detail TEXT);
             INSERT INTO history (ts, tool, action, status) VALUES (1, 'glazewm', 'activate', 'ok');",
        )
        .unwrap();
    }
    let s = Store::open(path.to_str().unwrap()).unwrap();
    let id = s.log_j(2, "zebar", "activate", "ok", None, Some("[]"));
    let h = s.history(10);
    assert_eq!(h.len(), 2, "the old row survived");
    assert!(h[0].undoable && h[0].id == id && !h[1].undoable);
    drop(s);
    // opening again must not fail on the already-added columns
    assert!(Store::open(path.to_str().unwrap()).is_ok());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_operation_stored_in_history_can_be_undone_from_its_journal() {
    let all = manifest::all();
    let initial = glaze_installed();
    let os = FakeOs::new(initial.clone());
    let plan = actions::plan(&os, &all, &tool("glazewm"), Action::Activate, false).unwrap();
    let (out, journal) = engine::run_journal(&os, &plan);
    assert!(out.ok);

    let s = Store::in_memory().unwrap();
    let id = s.log_j(1, "glazewm", "activate", "ok", None, Some(&journal.to_json()));
    // what the app would do on "Undo" after a restart: read the journal from the database
    let restored = Journal::from_json(&s.journal_of(id).unwrap()).unwrap();
    assert!(engine::revert(&os, &restored).is_empty());
    assert_eq!(os.machine(), initial);
    assert!(s.journal_of(9999).is_none());
}

// ---------------------------------------------------------------- palette

#[test]
fn palette_sync_recolors_installed_tools_and_can_be_undone() {
    let all = manifest::all();
    let theme = theme::load_embedded().into_iter().find(|e| e.theme.id == "catppuccin-dev").unwrap();
    let palette = theme.theme.palette.clone().unwrap();

    let os = FakeOs::new(glaze_installed());
    let changes = themeops::palette_changes(&os, &all, &palette);
    assert_eq!(changes.len(), 1);
    assert_eq!((changes[0].tool.as_str(), changes[0].role.as_str(), changes[0].color.as_str()), ("glazewm", "accent", "#cba6f7"));

    let initial = os.machine();
    let (out, journal, applied) = themeops::apply_palette(&os, &all, &palette);
    assert!(out.ok && applied.len() == 1, "{:?}", out.error);
    let cfg = os.read_for_test(GLAZE_CFG);
    assert!(cfg.contains("cba6f7") && cfg.contains("gaps"), "other options kept: {cfg}");
    assert!(engine::revert(&os, &journal).is_empty());
    assert_eq!(os.machine(), initial);

    // not installed -> nothing to recolor; a role the palette lacks -> skipped
    assert!(themeops::palette_changes(&FakeOs::new(World::default()), &all, &palette).is_empty());
    let mut no_accent = palette.clone();
    no_accent.colors.remove("accent");
    assert!(themeops::palette_changes(&os, &all, &no_accent).is_empty());
}

#[test]
fn palette_sync_is_atomic() {
    let all = manifest::all();
    let palette = theme::load_embedded().into_iter().find(|e| e.theme.id == "catppuccin-dev").unwrap().theme.palette.unwrap();
    let probe = FakeOs::new(glaze_installed());
    assert!(themeops::apply_palette(&probe, &all, &palette).0.ok);
    for n in 0..probe.ticks() {
        let os = FakeOs::new(glaze_installed());
        *os.fail_at.lock().unwrap() = Some(n);
        let (out, _, _) = themeops::apply_palette(&os, &all, &palette);
        assert!(!out.ok && out.rolled_back, "op {n}");
        assert_eq!(os.machine(), glaze_installed(), "op {n}");
    }
}

// ---------------------------------------------------------------- submission

fn my_theme() -> theme::ThemeEntry {
    let os = FakeOs::new(glaze_active());
    themeops::save_current(&os, &manifest::all(), "my-setup", "My setup").unwrap()
}

fn codes(issues: &[themeops::Issue]) -> Vec<(&str, &str)> {
    issues.iter().map(|i| (i.code.as_str(), i.level)).collect()
}

const PNG: &[u8] = b"\x89PNG fake image bytes";

#[test]
fn a_fresh_theme_is_not_ready_to_submit_and_the_wizard_fixes_each_problem() {
    let tools = manifest::all();
    let official: Vec<String> = theme::load_embedded().iter().map(|e| e.theme.id.clone()).collect();
    let e = my_theme();

    let issues = themeops::submission_issues(&e, &tools, &official);
    let c = codes(&issues);
    for expected in [("author_github_missing", "error"), ("license_not_open", "error"), ("missing_cover", "error"), ("no_palette", "warning"), ("review_configs", "warning")] {
        assert!(c.contains(&expected), "{expected:?} in {c:?}");
    }

    let meta = MetaEdit {
        name: "My setup".into(),
        license: "MIT".into(),
        github: "@someone".into(),
        description_en: "My tiling setup.".into(),
        description_pt: "Meu setup em mosaico.".into(),
    };
    let e = themeops::edit_meta(&e, &meta).unwrap();
    assert_eq!(e.theme.author.github.as_deref(), Some("someone"), "leading @ removed");
    assert_eq!(e.origin, Origin::Mine);
    let e = themeops::set_cover(&e, "PNG", PNG.to_vec()).unwrap();
    assert_eq!(e.theme.media.as_ref().unwrap().cover.as_deref(), Some("screenshots/cover.png"));

    let issues = themeops::submission_issues(&e, &tools, &official);
    assert!(issues.iter().all(|i| i.level != "error"), "{:?}", codes(&issues));
    // replacing the cover with another format leaves one cover file only
    let e = themeops::set_cover(&e, "jpg", b"JPG".to_vec()).unwrap();
    assert_eq!(e.files.keys().filter(|k| k.starts_with("screenshots/cover.")).count(), 1);
}

#[test]
fn submission_rejects_bad_input() {
    let e = my_theme();
    assert!(themeops::set_cover(&e, "exe", vec![1]).unwrap_err().starts_with("forbidden_file"));
    assert!(themeops::set_cover(&e, "svg", vec![1]).is_err(), "svg can carry scripts");
    assert_eq!(themeops::set_cover(&e, "png", vec![0; themeops::MAX_COVER_BYTES + 1]).unwrap_err(), "cover_too_large");
    let blank = MetaEdit { name: "  ".into(), license: "MIT".into(), github: "x".into(), description_en: "a".into(), description_pt: "b".into() };
    assert_eq!(themeops::edit_meta(&e, &blank).unwrap_err(), "invalid_theme_name");

    let tools = manifest::all();
    let taken = vec!["my-setup".to_string()];
    assert!(codes(&themeops::submission_issues(&e, &tools, &taken)).contains(&("id_taken", "error")));
    let noenglish = MetaEdit { name: "x".into(), license: "MIT".into(), github: "x".into(), description_en: " ".into(), description_pt: "b".into() };
    let e2 = themeops::edit_meta(&e, &noenglish).unwrap();
    assert!(codes(&themeops::submission_issues(&e2, &tools, &[])).contains(&("description_lang_missing", "error")));
    for bad in ["Proprietary", "Personal use", "GPL", ""] {
        let m = MetaEdit { name: "x".into(), license: bad.into(), github: "x".into(), description_en: "a".into(), description_pt: "b".into() };
        let e3 = themeops::edit_meta(&e, &m).unwrap();
        assert!(codes(&themeops::submission_issues(&e3, &tools, &[])).contains(&("license_not_open", "error")), "{bad}");
    }
}

#[test]
fn the_submission_folder_follows_the_repository_layout() {
    let e = my_theme();
    let meta = MetaEdit { name: "My setup".into(), license: "MIT".into(), github: "someone".into(), description_en: "My tiling setup.".into(), description_pt: "Meu setup.".into() };
    let e = themeops::set_cover(&themeops::edit_meta(&e, &meta).unwrap(), "png", PNG.to_vec()).unwrap();

    let os = FakeOs::new(World::default());
    let dir = themeops::prepare_submission(&os, &e).unwrap();
    assert_eq!(dir, r"%USERPROFILE%\Documents\WinRice\submit\my-setup");
    let w = os.world.lock().unwrap();
    assert!(w.files.contains_key(&format!(r"{dir}\catalog\themes\my-setup\theme.yaml")));
    assert!(w.files.contains_key(&format!(r"{dir}\catalog\themes\my-setup\configs\glazewm\config.yaml")));
    assert!(w.files.contains_key(&format!(r"{dir}\catalog\themes\my-setup\screenshots\cover.png")));
    let pr = &w.files[&format!(r"{dir}\PULL_REQUEST.md")];
    assert!(pr.contains("Add theme: My setup") && pr.contains("@someone") && pr.contains("MIT") && pr.contains("glazewm"));
}

#[test]
fn only_project_repository_links_can_be_opened() {
    for ok in ["https://github.com/riceverse-apps/winrice", "https://github.com/riceverse-apps/winrice/pulls", "https://github.com/riceverse-apps/winrice/blob/main/CONTRIBUTING.md"] {
        assert!(net::repo_url_allowed(ok), "{ok}");
    }
    for bad in [
        "https://github.com/riceverse-apps/winrice-evil",
        "https://github.com/riceverse-apps/winrice/../../evil",
        "https://evil.example/https://github.com/riceverse-apps/winrice",
        "http://github.com/riceverse-apps/winrice",
        "file:///C:/Windows/System32/calc.exe",
        "https://github.com/riceverse-apps/winrice/x y",
        "calc.exe",
        "",
    ] {
        assert!(!net::repo_url_allowed(bad), "{bad}");
    }
}

#[test]
fn glossary_is_bilingual_and_complete() {
    let g = manifest::glossary();
    assert!(g.len() >= 10);
    for t in &g {
        for l in ["en", "pt-BR"] {
            assert!(t.term.get(l).is_some_and(|s| !s.is_empty()) && t.definition.get(l).is_some_and(|s| !s.is_empty()), "{} {l}", t.id);
        }
    }
    assert!(g.iter().any(|t| t.id == "tiling"));
}
