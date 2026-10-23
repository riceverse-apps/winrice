mod actions;
mod backup;
mod engine;
mod hardware;
mod manifest;
mod net;
mod os;
mod profiles;
mod settings;
mod store;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests3;
#[cfg(test)]
mod tests4;
#[cfg(test)]
mod tests5;
#[cfg(test)]
mod tests6;
mod theme;
mod themeops;
mod widget;
#[cfg(test)]
mod tests2;
mod trial;

use actions::{Action, ToolStatus};
use engine::{Journal, Outcome, Plan, Step};
use manifest::Manifest;
use os::{Os, RealOs};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::{Mutex, RwLock};
use theme::{Origin, Theme, ThemeEntry};
use widget::WidgetSettings;
use store::{HistoryEntry, Store};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use trial::{LastOp, Watchdog, Who};

impl App {
    fn manifests(&self) -> Vec<Manifest> {
        self.manifests.read().unwrap().clone()
    }
    fn root(&self) -> PathBuf {
        PathBuf::from(self.os.data_dir())
    }
}

struct App {
    os: RealOs,
    store: Mutex<Store>,
    manifests: RwLock<Vec<Manifest>>,
    themes: RwLock<Vec<ThemeEntry>>,
    last: LastOp,
    /// Only one operation at a time: two transactions must never interleave.
    op_lock: Mutex<()>,
    /// Windows build the app saw on the previous run (None on first run).
    previous_build: Option<u32>,
    sampler: Mutex<hardware::Sampler>,
    widget: Mutex<WidgetSettings>,
    widget_shortcut: Mutex<Option<tauri_plugin_global_shortcut::Shortcut>>,
}

#[derive(Serialize)]
struct ToolView {
    manifest: Manifest,
    #[serde(flatten)]
    status: ToolStatus,
    installed_by_app: bool,
    favorite: bool,
}

#[derive(Serialize)]
struct ActionResult {
    outcome: Outcome,
    /// Present when the change is on probation (try mode): token + seconds left.
    token: Option<u64>,
    try_seconds: Option<u64>,
}

#[derive(Serialize, Clone)]
struct Reverted {
    tool: String,
    action: String,
    /// "timeout" | "shortcut" | "watchdog" | "user"
    reason: String,
}

#[derive(Serialize)]
struct SystemInfo {
    build: u32,
    edition: String,
    /// The build changed since WinRice last ran (Windows was updated).
    build_changed: bool,
    previous_build: Option<u32>,
}

fn find(app: &App, id: &str) -> Result<Manifest, String> {
    app.manifests().into_iter().find(|m| m.id == id).ok_or_else(|| format!("unknown_tool:{id}"))
}

fn tool_favorites(app: &App) -> Vec<String> {
    app.store.lock().unwrap().meta_get("tool_favorites").and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

#[tauri::command(async)]
fn toggle_tool_favorite(app: State<App>, id: String) -> bool {
    let mut f = tool_favorites(&app);
    let now_fav = if let Some(i) = f.iter().position(|x| x == &id) {
        f.remove(i);
        false
    } else {
        f.push(id);
        true
    };
    app.store.lock().unwrap().meta_set("tool_favorites", &serde_json::to_string(&f).unwrap_or_default());
    now_fav
}

#[tauri::command(async)]
fn list_tools(app: State<App>) -> Vec<ToolView> {
    let mine = app.store.lock().unwrap().installed_by_app();
    let fav = tool_favorites(&app);
    app.manifests()
        .into_iter()
        .map(|m| ToolView { status: actions::status(&app.os, &m), installed_by_app: mine.contains(&m.id), favorite: fav.contains(&m.id), manifest: m })
        .collect()
}

#[tauri::command]
fn get_docs(app: State<App>, id: String, lang: String) -> Option<String> {
    manifest::docs_with(&app.root(), &id, &lang)
}

#[tauri::command(async)]
fn system_info(app: State<App>) -> SystemInfo {
    let build = app.os.windows_build();
    SystemInfo {
        build,
        edition: app.os.windows_edition(),
        build_changed: app.previous_build.is_some_and(|p| p != build) && app.store.lock().unwrap().meta_get("build_ack").as_deref() != Some(&build.to_string()),
        previous_build: app.previous_build,
    }
}

#[tauri::command(async)]
fn ack_build(app: State<App>) {
    app.store.lock().unwrap().meta_set("build_ack", &app.os.windows_build().to_string());
}

#[tauri::command(async)]
fn plan_action(app: State<App>, id: String, action: String, resolve_conflicts: bool) -> Result<Plan, String> {
    let m = find(&app, &id)?;
    let action = Action::parse(&action).ok_or("unknown_action")?;
    actions::plan(&app.os, &app.manifests(), &m, action, resolve_conflicts).map_err(|e| e.to_string())
}

fn record(app: &App, plan: &Plan, outcome: &Outcome, journal: Option<&str>) -> i64 {
    let store = app.store.lock().unwrap();
    if outcome.ok {
        for s in &plan.steps {
            match s {
                Step::WingetInstall { tool, .. } => {
                    let _ = store.mark_installed(tool);
                }
                Step::WingetUninstall { tool, .. } => {
                    let _ = store.unmark_installed(tool);
                }
                _ => {}
            }
        }
    }
    let status = if outcome.ok { "ok" } else { "rolled_back" };
    store.log_j(app.os.now(), &plan.tool, &plan.action, status, outcome.error.as_deref(), journal)
}

/// Runs a plan and logs it. With `keep_undo` the history entry can be undone later.
fn execute(app: &App, plan: &Plan, keep_undo: bool) -> (Outcome, Journal, i64) {
    let _guard = app.op_lock.lock().unwrap();
    execute_locked(app, plan, keep_undo)
}

/// Same, for callers that already hold `op_lock`.
fn execute_locked(app: &App, plan: &Plan, keep_undo: bool) -> (Outcome, Journal, i64) {
    let (outcome, journal) = engine::run_journal(&app.os, plan);
    let json = (outcome.ok && keep_undo).then(|| journal.to_json());
    let id = record(app, plan, &outcome, json.as_deref());
    (outcome, journal, id)
}

fn set_topmost(handle: &AppHandle, on: bool) {
    if let Some(w) = handle.get_webview_window("main") {
        let _ = w.set_always_on_top(on);
        if on {
            let _ = w.unminimize();
            let _ = w.show();
            let _ = w.set_focus();
        }
    }
}

/// Reverts the last operation if the slot is still held by `who`. Used by the try-mode
/// timer, the Explorer watchdog, the emergency shortcut and the "Revert now" button.
fn revert_last(handle: &AppHandle, who: Who, only_if_try_pending: bool, reason: &str) -> bool {
    let app = handle.state::<App>();
    let Some(slot) = app.last.take(who, only_if_try_pending) else { return false };
    let errors = {
        let _guard = app.op_lock.lock().unwrap();
        engine::revert(&app.os, &slot.journal)
    };
    let detail = format!("{reason}{}", if errors.is_empty() { String::new() } else { format!(" (errors: {})", errors.join("; ")) });
    {
        let store = app.store.lock().unwrap();
        if let Some(id) = slot.history_id {
            store.mark_undone(id);
        }
        store.log(app.os.now(), &slot.tool, &format!("revert_{}", slot.action), "ok", Some(&detail));
    }
    set_topmost(handle, false);
    let _ = handle.emit("winrice://reverted", Reverted { tool: slot.tool, action: slot.action, reason: reason.into() });
    true
}

/// Watches Explorer for ~60 s after an operation; a crash loop reverts that operation.
fn start_watchdog(handle: AppHandle, token: u64) {
    std::thread::spawn(move || {
        let mut dog = Watchdog::default();
        for _ in 0..trial::WATCHDOG_POLLS {
            std::thread::sleep(std::time::Duration::from_millis(trial::WATCHDOG_INTERVAL_MS));
            let app = handle.state::<App>();
            if !app.last.is_current(token) {
                return;
            }
            if dog.observe(app.os.process_ids("explorer.exe")) {
                revert_last(&handle, Who::Token(token), false, "watchdog");
                return;
            }
        }
    });
}

fn start_try_timer(handle: AppHandle, token: u64) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(trial::TRY_SECONDS));
        revert_last(&handle, Who::Token(token), true, "timeout");
    });
}

/// Re-plans on the backend (never trusts a plan sent by the UI) and runs it.
/// With `try_mode` the change is applied for real but reverts by itself unless confirmed.
#[tauri::command(async)]
fn run_action(
    handle: AppHandle,
    app: State<App>,
    id: String,
    action: String,
    resolve_conflicts: bool,
    try_mode: bool,
) -> Result<ActionResult, String> {
    let m = find(&app, &id)?;
    let act = Action::parse(&action).ok_or("unknown_action")?;
    // Plan AND run under the same lock: a duplicate request (double click) that waited here
    // re-plans against the state the first one left, and is refused ("already installed").
    let guard = app.op_lock.lock().unwrap();
    let plan = actions::plan(&app.os, &app.manifests(), &m, act, resolve_conflicts).map_err(|e| e.to_string())?;
    let (outcome, journal, history_id) = execute_locked(&app, &plan, true);
    drop(guard);
    if !outcome.ok || plan.steps.is_empty() {
        return Ok(ActionResult { outcome, token: None, try_seconds: None });
    }
    let trying = try_mode && matches!(act, Action::Activate);
    let token = app.last.store(&plan.tool, &plan.action, journal, trying, Some(history_id));
    if matches!(act, Action::Activate) {
        start_watchdog(handle.clone(), token);
    }
    if trying {
        set_topmost(&handle, true);
        start_try_timer(handle, token);
    }
    Ok(ActionResult { outcome, token: trying.then_some(token), try_seconds: trying.then_some(trial::TRY_SECONDS) })
}

#[tauri::command(async)]
fn keep_try(handle: AppHandle, app: State<App>, token: u64) -> bool {
    let kept = app.last.keep(token);
    if kept {
        set_topmost(&handle, false);
        app.store.lock().unwrap().log(app.os.now(), "-", "keep", "ok", None);
    }
    kept
}

#[tauri::command(async)]
fn revert_now(handle: AppHandle) -> bool {
    revert_last(&handle, Who::Anyone, false, "user")
}

#[tauri::command(async)]
fn plan_restore(app: State<App>, uninstall: bool) -> Plan {
    let mine = app.store.lock().unwrap().installed_by_app();
    actions::plan_restore(&app.os, &app.manifests(), &mine, uninstall)
}

#[tauri::command(async)]
fn run_restore(app: State<App>, uninstall: bool) -> Outcome {
    let mine = app.store.lock().unwrap().installed_by_app();
    let plan = actions::plan_restore(&app.os, &app.manifests(), &mine, uninstall);
    // Restoring is final: it is not kept as something to revert.
    execute(&app, &plan, false).0
}

#[tauri::command(async)]
fn get_settings(app: State<App>, id: String) -> Result<std::collections::BTreeMap<String, serde_json::Value>, String> {
    Ok(settings::read_all(&app.os, &find(&app, &id)?))
}

#[tauri::command(async)]
fn set_setting(app: State<App>, id: String, key: String, value: serde_json::Value) -> Result<Outcome, String> {
    let m = find(&app, &id)?;
    let plan = settings::plan_set(&app.os, &m, &key, value)?;
    Ok(execute(&app, &plan, true).0)
}

#[tauri::command(async)]
fn list_history(app: State<App>) -> Vec<HistoryEntry> {
    app.store.lock().unwrap().history(100)
}


// ---------------------------------------------------------------------- themes

#[derive(Serialize)]
struct ThemeView {
    theme: Theme,
    origin: Origin,
    favorite: bool,
    applied: bool,
    compatible: bool,
    /// Files that make up the theme, to show what an import contains.
    files: Vec<String>,
}

#[derive(Serialize)]
struct ActiveTheme {
    id: String,
    name: String,
    accent: Option<String>,
    base: Option<String>,
    text: Option<String>,
}

fn favorites(app: &App) -> Vec<String> {
    app.store.lock().unwrap().meta_get("favorites").and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn active_theme_id(app: &App) -> Option<String> {
    app.store.lock().unwrap().current_theme().map(|(id, _)| id)
}

/// Loads manifests and themes from every source: synced catalog, bundled, imported, mine.
fn reload(app: &App) {
    let root = app.root();
    let manifests = manifest::all_with(&root);
    let themes = theme::merge(vec![
        theme::load_dir(&root.join("catalog").join("themes"), Origin::Official),
        theme::load_embedded(),
        theme::load_dir(&root.join("themes").join("imported"), Origin::Imported),
        theme::load_dir(&root.join("themes").join("mine"), Origin::Mine),
    ])
    .into_iter()
    .filter(|e| theme::validate_catalog(&e.theme, &manifests).is_ok())
    .collect();
    *app.manifests.write().unwrap() = manifests;
    *app.themes.write().unwrap() = themes;
}

fn find_theme(app: &App, id: &str) -> Result<ThemeEntry, String> {
    app.themes.read().unwrap().iter().find(|t| t.theme.id == id).cloned().ok_or_else(|| format!("unknown_theme:{id}"))
}

fn theme_view(app: &App, e: &ThemeEntry) -> ThemeView {
    let build = app.os.windows_build();
    ThemeView {
        theme: e.theme.clone(),
        origin: e.origin,
        favorite: favorites(app).contains(&e.theme.id),
        applied: active_theme_id(app).as_deref() == Some(e.theme.id.as_str()),
        compatible: e.theme.requires.as_ref().and_then(|r| r.windows_min_build).map_or(true, |min| build >= min),
        files: e.files.keys().cloned().collect(),
    }
}

#[tauri::command(async)]
fn list_themes(app: State<App>) -> Vec<ThemeView> {
    app.themes.read().unwrap().iter().map(|e| theme_view(&app, e)).collect()
}

#[tauri::command(async)]
fn theme_preview(app: State<App>, id: String, selected: Vec<String>) -> Result<themeops::ThemePreview, String> {
    themeops::preview(&app.os, &app.manifests(), &find_theme(&app, &id)?, &selected)
}

#[tauri::command(async)]
fn apply_theme(app: State<App>, id: String, selected: Vec<String>) -> Result<Outcome, String> {
    let entry = find_theme(&app, &id)?;
    if !entry.theme.components.iter().any(|c| selected.contains(&c.tool)) {
        return Err("nothing_selected".into());
    }
    let _guard = app.op_lock.lock().unwrap();
    let manifests = app.manifests();
    // Tools the theme will install count as installed by WinRice ("Restore original" removes them).
    let to_install: Vec<String> = manifests
        .iter()
        .filter(|m| selected.contains(&m.id) && entry.theme.components.iter().any(|c| c.tool == m.id))
        .filter(|m| !actions::status(&app.os, m).installed)
        .map(|m| m.id.clone())
        .collect();
    let (outcome, journal) = themeops::apply(&app.os, &manifests, &entry, &selected);
    let store = app.store.lock().unwrap();
    let json = outcome.ok.then(|| journal.to_json());
    if let Some(j) = &json {
        store.push_theme(&id, app.os.now(), j);
        for t in &to_install {
            let _ = store.mark_installed(t);
        }
    }
    store.log_j(app.os.now(), &id, "theme_apply", if outcome.ok { "ok" } else { "rolled_back" }, outcome.error.as_deref(), json.as_deref());
    Ok(outcome)
}

#[tauri::command(async)]
fn undo_theme(app: State<App>) -> Result<Outcome, String> {
    undo_theme_inner(&app)
}

fn undo_theme_inner(app: &App) -> Result<Outcome, String> {
    let _guard = app.op_lock.lock().unwrap();
    let store = app.store.lock().unwrap();
    let (id, raw) = store.current_theme().ok_or("no_theme_applied")?;
    let journal = Journal::from_json(&raw).ok_or("corrupt_journal")?;
    let errors = engine::revert(&app.os, &journal);
    store.pop_theme();
    if let Some(row) = store.history(200).into_iter().find(|h| h.action == "theme_apply" && h.tool == id && !h.undone && h.status == "ok") {
        store.mark_undone(row.id);
    }
    // Forget tools that the undo uninstalled.
    for m in app.manifests() {
        if !actions::status(&app.os, &m).installed {
            let _ = store.unmark_installed(&m.id);
        }
    }
    store.log(app.os.now(), &id, "theme_undo", if errors.is_empty() { "ok" } else { "rolled_back" }, errors.first().map(String::as_str));
    Ok(Outcome { ok: errors.is_empty(), applied: 0, error: errors.first().cloned(), rolled_back: false, rollback_errors: errors })
}

#[tauri::command(async)]
fn active_theme(app: State<App>) -> Option<ActiveTheme> {
    let id = active_theme_id(&app)?;
    let e = find_theme(&app, &id).ok()?;
    let color = |k: &str| e.theme.palette.as_ref().and_then(|p| p.colors.get(k).cloned());
    Some(ActiveTheme { id, name: e.theme.name.clone(), accent: color("accent"), base: color("base"), text: color("text") })
}

#[tauri::command(async)]
fn toggle_favorite(app: State<App>, id: String) -> bool {
    let mut f = favorites(&app);
    let now_fav = if let Some(i) = f.iter().position(|x| x == &id) {
        f.remove(i);
        false
    } else {
        f.push(id);
        true
    };
    app.store.lock().unwrap().meta_set("favorites", &serde_json::to_string(&f).unwrap_or_default());
    now_fav
}

fn store_user_theme(app: &App, entry: &ThemeEntry, folder: &str) -> Result<(), String> {
    if app.themes.read().unwrap().iter().any(|t| t.theme.id == entry.theme.id && t.origin == Origin::Official) {
        return Err("theme_id_taken".into());
    }
    theme::store_dir(&app.root().join("themes").join(folder), entry)?;
    reload(app);
    Ok(())
}

#[tauri::command(async)]
fn import_theme(app: State<App>, url: String) -> Result<ThemeView, String> {
    let entry = net::import_github(&net::RealHttp, &url, &app.manifests())?;
    store_user_theme(&app, &entry, "imported")?;
    Ok(theme_view(&app, &find_theme(&app, &entry.theme.id)?))
}

#[tauri::command(async)]
fn save_theme(app: State<App>, id: String, name: String) -> Result<ThemeView, String> {
    let entry = themeops::save_current(&app.os, &app.manifests(), &id, &name)?;
    store_user_theme(&app, &entry, "mine")?;
    Ok(theme_view(&app, &find_theme(&app, &id)?))
}

/// Writes `<Downloads>\<id>.zip` and returns its path.
#[tauri::command(async)]
fn export_theme(app: State<App>, id: String) -> Result<String, String> {
    let entry = find_theme(&app, &id)?;
    let zip = themeops::export_zip(&entry)?;
    let path = format!("{}\\Downloads\\{id}.zip", app.os.expand("%USERPROFILE%"));
    app.os.write_bytes(&path, &zip).map_err(|e| e.to_string())?;
    Ok(path)
}

#[tauri::command(async)]
fn delete_theme(app: State<App>, id: String) -> Result<(), String> {
    let entry = find_theme(&app, &id)?;
    let folder = match entry.origin {
        Origin::Imported => "imported",
        Origin::Mine => "mine",
        Origin::Official => return Err("cannot_delete_official".into()),
    };
    let _ = std::fs::remove_dir_all(app.root().join("themes").join(folder).join(&id));
    reload(&app);
    Ok(())
}

/// Downloads new tools/themes from the remote catalog (hash-checked) and reloads.
#[tauri::command(async)]
fn sync_catalog(app: State<App>) -> Result<net::SyncSummary, String> {
    let summary = net::sync_catalog(&net::RealHttp, &app.root(), net::INDEX_URL)?;
    reload(&app);
    Ok(summary)
}

// ------------------------------------------------------------ widget, hardware, profiles

#[derive(Serialize)]
struct SwitchResult {
    outcome: Outcome,
    skipped: Vec<String>,
}

#[derive(Serialize)]
struct ProfileView {
    name: String,
    tools: Vec<String>,
}

#[derive(Serialize)]
struct WidgetSaved {
    settings: WidgetSettings,
    /// Something that could not be applied (e.g. the shortcut is taken by another program).
    warning: Option<String>,
}

#[tauri::command(async)]
fn hardware_snapshot(app: State<App>) -> hardware::Snapshot {
    app.sampler.lock().unwrap().sample()
}

#[tauri::command(async)]
fn list_profiles(app: State<App>) -> Vec<ProfileView> {
    app.store.lock().unwrap().profiles().into_iter().map(|(name, tools)| ProfileView { name, tools }).collect()
}

#[tauri::command(async)]
fn save_profile(app: State<App>, name: String) -> Result<ProfileView, String> {
    let name = name.trim().to_string();
    if name.is_empty() || name.len() > 60 {
        return Err("invalid_profile_name".into());
    }
    let tools = profiles::active_tools(&app.os, &app.manifests());
    app.store.lock().unwrap().save_profile(&name, &tools, app.os.now());
    Ok(ProfileView { name, tools })
}

#[tauri::command(async)]
fn delete_profile(app: State<App>, name: String) {
    app.store.lock().unwrap().delete_profile(&name);
}

fn switch_logged(app: &App, tool: &str, action: &str, target: &[String]) -> SwitchResult {
    let _guard = app.op_lock.lock().unwrap();
    let sw = profiles::switch(&app.os, &app.manifests(), target);
    let status = if sw.outcome.ok { "ok" } else { "rolled_back" };
    // Pause/resume have their own buttons; only a profile switch is undone from the history.
    let journal = (sw.outcome.ok && action == "profile_apply").then(|| sw.journal.to_json());
    app.store.lock().unwrap().log_j(app.os.now(), tool, action, status, sw.outcome.error.as_deref(), journal.as_deref());
    SwitchResult { outcome: sw.outcome, skipped: sw.skipped }
}

#[tauri::command(async)]
fn apply_profile(app: State<App>, name: String) -> Result<SwitchResult, String> {
    let tools = app
        .store
        .lock()
        .unwrap()
        .profiles()
        .into_iter()
        .find(|(n, _)| n == &name)
        .map(|(_, t)| t)
        .ok_or("unknown_profile")?;
    Ok(switch_logged(&app, &name, "profile_apply", &tools))
}

/// "Pause customizations": turns every tool off, remembering which were on.
#[tauri::command(async)]
fn pause_customizations(app: State<App>) -> Result<SwitchResult, String> {
    if app.store.lock().unwrap().meta_get("paused").is_some() {
        return Err("already_paused".into());
    }
    let active = profiles::active_tools(&app.os, &app.manifests());
    if active.is_empty() {
        return Err("nothing_active".into());
    }
    let res = switch_logged(&app, "all", "pause", &[]);
    if res.outcome.ok {
        app.store.lock().unwrap().meta_set("paused", &serde_json::to_string(&active).unwrap_or_default());
    }
    Ok(res)
}

#[tauri::command(async)]
fn resume_customizations(app: State<App>) -> Result<SwitchResult, String> {
    let raw = app.store.lock().unwrap().meta_get("paused").ok_or("not_paused")?;
    let tools: Vec<String> = serde_json::from_str(&raw).unwrap_or_default();
    let res = switch_logged(&app, "all", "resume", &tools);
    if res.outcome.ok {
        app.store.lock().unwrap().meta_del("paused");
    }
    Ok(res)
}

/// Tools that were on before "Pause customizations" (None when not paused).
#[tauri::command(async)]
fn paused_state(app: State<App>) -> Option<Vec<String>> {
    app.store.lock().unwrap().meta_get("paused").and_then(|s| serde_json::from_str(&s).ok())
}

#[tauri::command(async)]
fn quick_action(app: State<App>, id: String, action: String) -> Result<(), String> {
    let m = find(&app, &id)?;
    if !actions::status(&app.os, &m).active {
        return Err("not_active".into());
    }
    let (prog, args) = profiles::quick_action_command(&app.os, &m, &action)?;
    let code = app.os.run(&prog, &args)?;
    app.store.lock().unwrap().log(app.os.now(), &id, &format!("quick_{action}"), if code == 0 { "ok" } else { "rolled_back" }, None);
    if code == 0 { Ok(()) } else { Err(format!("command_failed:{code}")) }
}

#[tauri::command]
fn get_widget_settings(app: State<App>) -> WidgetSettings {
    app.widget.lock().unwrap().clone()
}

/// Applies tray / floating window / shortcut. Returns a warning for the part that failed.
fn apply_widget(handle: &AppHandle, s: &WidgetSettings) -> Option<String> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
    let app = handle.state::<App>();
    let mut warning = None;
    if let Some(tray) = handle.tray_by_id(widget::TRAY_ID) {
        let _ = tray.set_visible(s.flyout);
    }
    if !s.flyout {
        widget::hide(handle, widget::FLYOUT);
    }
    if s.floating {
        if let Err(e) = widget::show_floating(handle, s) {
            warning = Some(e);
        }
    } else {
        widget::hide(handle, widget::FLOAT);
    }
    let gs = handle.global_shortcut();
    let mut current = app.widget_shortcut.lock().unwrap();
    if let Some(old) = current.take() {
        let _ = gs.unregister(old);
    }
    if s.shortcut_enabled {
        match s.shortcut.parse::<Shortcut>() {
            Err(_) => warning = Some("invalid_shortcut".into()),
            Ok(sc) => match gs.register(sc) {
                Ok(()) => *current = Some(sc),
                Err(_) => warning = Some("shortcut_in_use".into()),
            },
        }
    }
    warning
}

#[tauri::command(async)]
fn set_widget_settings(handle: AppHandle, app: State<App>, settings: WidgetSettings) -> WidgetSaved {
    let s = settings.sanitized();
    app.store.lock().unwrap().meta_set("widget_settings", &serde_json::to_string(&s).unwrap_or_default());
    *app.widget.lock().unwrap() = s.clone();
    let warning = apply_widget(&handle, &s);
    let _ = handle.emit("winrice://widget-settings", &s);
    WidgetSaved { settings: s, warning }
}

#[tauri::command(async)]
fn toggle_floating_widget(handle: AppHandle, app: State<App>) {
    let s = app.widget.lock().unwrap().clone();
    widget::toggle_floating(&handle, &s);
}

#[tauri::command]
fn hide_window(handle: AppHandle, label: String) {
    if label == widget::FLYOUT || label == widget::FLOAT {
        widget::hide(&handle, &label);
    }
}

#[tauri::command]
fn open_main_window(handle: AppHandle) {
    widget::open_main(&handle);
}

/// The tray menu text comes from the UI language.
#[tauri::command]
fn set_tray_labels(handle: AppHandle, open: String, widget_label: String, quit: String) {
    widget::set_tray_labels(&handle, &open, &widget_label, &quit);
}


// ------------------------------------------------------------ history undo, palette, submission

#[derive(Serialize)]
struct PaletteResult {
    outcome: Outcome,
    changes: Vec<themeops::PaletteChange>,
}

/// Undoes the newest change from the history. Older entries cannot be undone out of order.
#[tauri::command(async)]
fn undo_history(app: State<App>, id: i64) -> Result<Outcome, String> {
    let row = app.store.lock().unwrap().history(200).into_iter().find(|h| h.id == id).ok_or("unknown_entry")?;
    if !row.undoable {
        return Err("not_undoable".into());
    }
    if row.action == "theme_apply" {
        return undo_theme_inner(&app);
    }
    let _guard = app.op_lock.lock().unwrap();
    let store = app.store.lock().unwrap();
    let journal = store.journal_of(id).and_then(|j| Journal::from_json(&j)).ok_or("not_undoable")?;
    let errors = engine::revert(&app.os, &journal);
    store.mark_undone(id);
    app.last.drop_history(id);
    for m in app.manifests() {
        if !actions::status(&app.os, &m).installed {
            let _ = store.unmark_installed(&m.id);
        }
    }
    store.log(app.os.now(), &row.tool, "undo", if errors.is_empty() { "ok" } else { "rolled_back" }, errors.first().map(String::as_str));
    Ok(Outcome { ok: errors.is_empty(), applied: 0, error: errors.first().cloned(), rolled_back: false, rollback_errors: errors })
}

#[tauri::command(async)]
fn palette_preview(app: State<App>, id: String) -> Result<Vec<themeops::PaletteChange>, String> {
    let entry = find_theme(&app, &id)?;
    let palette = entry.theme.palette.ok_or("no_palette")?;
    Ok(themeops::palette_changes(&app.os, &app.manifests(), &palette))
}

#[tauri::command(async)]
fn apply_palette(app: State<App>, id: String) -> Result<PaletteResult, String> {
    let entry = find_theme(&app, &id)?;
    let palette = entry.theme.palette.ok_or("no_palette")?;
    let _guard = app.op_lock.lock().unwrap();
    let (outcome, journal, changes) = themeops::apply_palette(&app.os, &app.manifests(), &palette);
    if changes.is_empty() && outcome.ok {
        return Err("nothing_to_recolor".into());
    }
    let json = outcome.ok.then(|| journal.to_json());
    app.store.lock().unwrap().log_j(app.os.now(), &id, "palette_apply", if outcome.ok { "ok" } else { "rolled_back" }, outcome.error.as_deref(), json.as_deref());
    Ok(PaletteResult { outcome, changes })
}

fn own_theme(app: &App, id: &str) -> Result<ThemeEntry, String> {
    let e = find_theme(app, id)?;
    if e.origin == Origin::Official {
        return Err("already_official".into());
    }
    Ok(e)
}

fn folder_of(o: Origin) -> &'static str {
    if o == Origin::Imported { "imported" } else { "mine" }
}

#[tauri::command(async)]
fn submission_check(app: State<App>, id: String) -> Result<Vec<themeops::Issue>, String> {
    let e = own_theme(&app, &id)?;
    let official: Vec<String> = app.themes.read().unwrap().iter().filter(|t| t.origin == Origin::Official).map(|t| t.theme.id.clone()).collect();
    Ok(themeops::submission_issues(&e, &app.manifests(), &official))
}

#[tauri::command(async)]
fn edit_theme_meta(app: State<App>, id: String, meta: themeops::MetaEdit) -> Result<ThemeView, String> {
    let e = own_theme(&app, &id)?;
    let updated = themeops::edit_meta(&e, &meta)?;
    store_user_theme(&app, &updated, folder_of(e.origin))?;
    Ok(theme_view(&app, &find_theme(&app, &id)?))
}

#[tauri::command(async)]
fn set_theme_cover(app: State<App>, id: String, path: String) -> Result<ThemeView, String> {
    let e = own_theme(&app, &id)?;
    let p = std::path::Path::new(path.trim().trim_matches('"'));
    let ext = p.extension().and_then(|x| x.to_str()).unwrap_or("").to_string();
    let len = std::fs::metadata(p).map_err(|_| "cover_not_found".to_string())?.len() as usize;
    if len > themeops::MAX_COVER_BYTES {
        return Err("cover_too_large".into());
    }
    let bytes = std::fs::read(p).map_err(|_| "cover_not_found".to_string())?;
    let updated = themeops::set_cover(&e, &ext, bytes)?;
    store_user_theme(&app, &updated, folder_of(e.origin))?;
    Ok(theme_view(&app, &find_theme(&app, &id)?))
}

/// Writes the theme in repository layout plus the pull request text. Returns the folder.
#[tauri::command(async)]
fn prepare_submission(app: State<App>, id: String) -> Result<String, String> {
    let e = own_theme(&app, &id)?;
    let official: Vec<String> = app.themes.read().unwrap().iter().filter(|t| t.origin == Origin::Official).map(|t| t.theme.id.clone()).collect();
    if themeops::submission_issues(&e, &app.manifests(), &official).iter().any(|i| i.level == "error") {
        return Err("submission_has_errors".into());
    }
    themeops::prepare_submission(&app.os, &e)
}

/// Opens a page of the project repository in the default browser. Nothing else can be opened.
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    if !net::repo_url_allowed(&url) {
        return Err("url_not_allowed".into());
    }
    std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", &url])
        .creation_flags(0x0800_0000)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_glossary() -> Vec<manifest::GlossaryTerm> {
    manifest::glossary()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

    let os = RealOs;
    let store = Store::open(&format!("{}\\state.db", os.data_dir())).expect("cannot open WinRice state database");
    let build = os.windows_build();
    let previous_build = store.meta_get("last_build").and_then(|s| s.parse().ok());
    store.meta_set("last_build", &build.to_string());
    let widget_settings: WidgetSettings =
        store.meta_get("widget_settings").and_then(|s| serde_json::from_str::<WidgetSettings>(&s).ok()).unwrap_or_default().sanitized_or_default();
    let app = App {
        os,
        store: Mutex::new(store),
        manifests: RwLock::new(vec![]),
        themes: RwLock::new(vec![]),
        last: LastOp::default(),
        op_lock: Mutex::new(()),
        previous_build,
        sampler: Mutex::new(hardware::Sampler::new()),
        widget: Mutex::new(widget_settings),
        widget_shortcut: Mutex::new(None),
    };

    reload(&app);

    // Emergency shortcut: Ctrl + Alt + Shift + R reverts the last change, even if the UI is gone.
    let emergency = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT), Code::KeyR);

    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |handle, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    if shortcut == &emergency {
                        revert_last(handle, Who::Anyone, false, "shortcut");
                    } else {
                        let state = handle.state::<App>();
                        let is_widget = state.widget_shortcut.lock().unwrap().as_ref() == Some(shortcut);
                        if is_widget {
                            let (h, s) = (handle.clone(), state.widget.lock().unwrap().clone());
                            std::thread::spawn(move || widget::toggle_floating(&h, &s));
                        }
                    }
                })
                .build(),
        )
        .manage(app)
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            let handle = window.app_handle();
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    let s = handle.state::<App>().widget.lock().unwrap().clone();
                    if s.close_to_tray && s.flyout {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                }
                // The widget windows must not keep WinRice alive once the main window is gone.
                WindowEvent::Destroyed => handle.exit(0),
                _ => {}
            }
        })
        .setup(move |tauri_app| {
            // If another program owns the shortcut we keep running; the on-screen button still works.
            if let Err(e) = tauri_app.global_shortcut().register(emergency) {
                eprintln!("could not register the emergency shortcut: {e}");
            }
            let handle = tauri_app.handle().clone();
            let getter = {
                let h = handle.clone();
                move || h.state::<App>().widget.lock().unwrap().clone()
            };
            widget::build_tray(&handle, getter)?;
            let s = handle.state::<App>().widget.lock().unwrap().clone();
            std::thread::spawn(move || {
                let _ = apply_widget(&handle, &s);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_tools,
            get_docs,
            system_info,
            ack_build,
            plan_action,
            run_action,
            keep_try,
            revert_now,
            plan_restore,
            run_restore,
            get_settings,
            set_setting,
            list_history,
            list_themes,
            theme_preview,
            apply_theme,
            undo_theme,
            active_theme,
            toggle_favorite,
            import_theme,
            save_theme,
            export_theme,
            delete_theme,
            sync_catalog,
            hardware_snapshot,
            list_profiles,
            save_profile,
            delete_profile,
            apply_profile,
            pause_customizations,
            resume_customizations,
            paused_state,
            quick_action,
            get_widget_settings,
            set_widget_settings,
            toggle_floating_widget,
            hide_window,
            open_main_window,
            set_tray_labels,
            toggle_tool_favorite,
            undo_history,
            palette_preview,
            apply_palette,
            submission_check,
            edit_theme_meta,
            set_theme_cover,
            prepare_submission,
            open_url,
            get_glossary
        ])
        .run(tauri::generate_context!())
        .expect("error while running WinRice");
}

