# Architecture

WinRice is a Tauri 2 app: a Rust core (`app/src-tauri`) and a Vue 3 + TypeScript interface (`app/src`). There is **no per-tool code**: tools and themes are data (`catalog/`), and the core only knows how to run what a manifest describes.

```
UI (Vue)  ──Tauri commands──▶  Core (Rust)
                               ├─ manifest.rs     tool manifests + embedded catalog (+ synced catalog)
                               ├─ theme.rs        Theme Loader and validation (data only)
                               ├─ actions.rs      manifest + action  ─▶  Plan (list of Steps)
                               ├─ engine.rs       Transaction Engine: apply, journal, rollback
                               ├─ backup.rs       backups and "originals" for Restore original Windows
                               ├─ themeops.rs     apply / preview / save / export themes, palette sync, submission
                               ├─ profiles.rs     profiles, pause/resume (one function: switch)
                               ├─ settings.rs     simple options written into YAML/JSON configs
                               ├─ trial.rs        try mode, Explorer watchdog, the "last operation" slot
                               ├─ net.rs          GitHub import, remote catalog sync (SHA-256), allowed links
                               ├─ hardware.rs     HardwareMonitor (no admin needed)
                               ├─ widget.rs       tray flyout, floating widget, widget settings
                               ├─ store.rs        SQLite: history (with undo journals), profiles, state
                               └─ os.rs           the Os trait: every contact with the machine
```

## Transaction Engine

Every change is a `Plan`: an ordered list of `Step`s (install with winget, back up files, write a config, set an autostart key, start/stop a process, ...). `engine::run_journal` applies the steps one by one. For each step it records how to undo it. If a step fails, the recorded undos run in reverse order, so the machine ends exactly as it started.

The journal of a successful run can be kept and replayed later. That is what powers try mode, the emergency shortcut, the watchdog, "Undo theme" and undo from the History. Journals are stored as JSON in SQLite, so undo survives restarts.

Some operations need the result of a previous step (a program exists only after it is installed), so applying a theme, a profile or a palette is a **sequence of just-in-time plans** whose journals are merged. If any plan fails, all earlier journals are reverted too.

## The `Os` trait

All access to files, processes, the registry, winget and the wallpaper goes through `os::Os`. `RealOs` talks to Windows; `FakeOs` (tests only) is an in-memory machine that can **fail at the N-th change**. The core test pattern is:

> run a plan once to count its changes; then for every N, make the N-th change fail and assert that the machine equals the initial state.

This is how rollback is proven for install, activate, deactivate, themes, profiles, pause/resume, palette sync and settings.

## Safety rules enforced in code

- Plans are re-built in the backend; the UI never sends a plan to execute.
- Themes: only data file types, safe relative paths, 8 MB maximum; PowerShell profiles are never copied into saved themes.
- Network: HTTPS to GitHub hosts only, size limits, SHA-256 verification for catalog files.
- Links opened from the app: only pages of this repository.
- Undo from the History is allowed only for the newest state-changing entry.

## Data locations

`%APPDATA%\WinRice\` — `state.db` (SQLite), `backups\originals\` and `backups\<timestamp>\`, `themes\imported\`, `themes\mine\`, `catalog\` (synced tools/themes), `wallpapers\`.

## Frontend

Views in `app/src/views`, shared state in `app/src/composables`. Colors come only from CSS tokens (`styles/tokens.css`), text only from `locales/*.json`. The widget windows are the same app opened on the `/widget` route (the route is chosen from the window label).
