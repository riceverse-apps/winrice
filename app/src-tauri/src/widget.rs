//! Quick-access widget: settings, tray flyout and floating window (see 11-widget-e-hardware.md).
//!
//! The widget UI is the same web app, opened in two small extra windows (`flyout`, `widget`).
//! They use the same commands as the main window, so toggles have the same engine, backup
//! and rollback.

use serde::{Deserialize, Serialize};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};

pub const FLYOUT: &str = "flyout";
pub const FLOAT: &str = "widget";
pub const TRAY_ID: &str = "winrice-tray";
pub const METRICS: [&str; 8] = ["cpu", "gpu", "ram", "disk", "net", "system", "pc", "sensors"];
const CORNERS: [&str; 4] = ["top-left", "top-right", "bottom-left", "bottom-right"];
const LAYERS: [&str; 3] = ["top", "normal", "bottom"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WidgetSettings {
    /// Panel that opens from the tray icon.
    pub flyout: bool,
    /// Small borderless window that stays on the desktop.
    pub floating: bool,
    /// Closing the main window keeps WinRice in the tray.
    pub close_to_tray: bool,
    pub shortcut_enabled: bool,
    /// Global shortcut that shows/hides the floating widget, e.g. "Super+Alt+W".
    pub shortcut: String,
    pub corner: String,
    pub opacity: f32,
    /// "top" (always on top), "normal" or "bottom" (always behind other windows).
    pub layer: String,
    pub detailed: bool,
    pub refresh_ms: u32,
    pub metrics: Vec<String>,
    /// Tools shown in the widget, in this order. Empty = every installed tool with a toggle.
    pub tools: Vec<String>,
    pub alert_ram_pct: Option<u32>,
    pub alert_cpu_temp: Option<u32>,
    pub alert_gpu_temp: Option<u32>,
}

impl Default for WidgetSettings {
    fn default() -> Self {
        Self {
            flyout: true,
            floating: false,
            close_to_tray: false,
            shortcut_enabled: true,
            shortcut: "Super+Alt+W".into(),
            corner: "bottom-right".into(),
            opacity: 0.95,
            layer: "normal".into(),
            detailed: false,
            refresh_ms: 1000,
            metrics: ["cpu", "ram", "disk", "net", "system"].map(String::from).to_vec(),
            tools: vec![],
            alert_ram_pct: None,
            alert_cpu_temp: None,
            alert_gpu_temp: None,
        }
    }
}

impl WidgetSettings {
    pub fn sanitized_or_default(self) -> Self {
        self.sanitized()
    }

    /// Clamps every value to what the widget supports (500 ms minimum refresh, etc.).
    pub fn sanitized(mut self) -> Self {
        let d = Self::default();
        self.refresh_ms = self.refresh_ms.clamp(500, 60_000);
        self.opacity = if self.opacity.is_finite() { self.opacity.clamp(0.3, 1.0) } else { d.opacity };
        if !CORNERS.contains(&self.corner.as_str()) {
            self.corner = d.corner;
        }
        if !LAYERS.contains(&self.layer.as_str()) {
            self.layer = d.layer;
        }
        self.metrics.retain(|m| METRICS.contains(&m.as_str()));
        self.metrics.dedup();
        self.tools.dedup();
        let pct = |v: Option<u32>, lo: u32, hi: u32| v.map(|x| x.clamp(lo, hi));
        self.alert_ram_pct = pct(self.alert_ram_pct, 1, 100);
        self.alert_cpu_temp = pct(self.alert_cpu_temp, 30, 120);
        self.alert_gpu_temp = pct(self.alert_gpu_temp, 30, 120);
        if self.shortcut.trim().is_empty() {
            self.shortcut = d.shortcut;
        }
        self
    }
}

fn window(app: &AppHandle, label: &str, w: f64, h: f64, transparent: bool) -> Result<WebviewWindow, String> {
    if let Some(win) = app.get_webview_window(label) {
        return Ok(win);
    }
    // The page picks its layout from the window label (see main.ts).
    WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title("WinRice")
        .inner_size(w, h)
        .decorations(false)
        .resizable(false)
        .skip_taskbar(true)
        .transparent(transparent)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())
}

/// Puts a window in a corner of the primary monitor's work area (taskbar excluded).
fn place(app: &AppHandle, win: &WebviewWindow, corner: &str) {
    let (Ok(Some(mon)), Ok(size)) = (app.primary_monitor(), win.outer_size()) else { return };
    let area = mon.work_area();
    let margin = (16.0 * mon.scale_factor()) as i32;
    let (aw, ah) = (area.size.width as i32, area.size.height as i32);
    let x = if corner.ends_with("left") { area.position.x + margin } else { area.position.x + aw - size.width as i32 - margin };
    let y = if corner.starts_with("top") { area.position.y + margin } else { area.position.y + ah - size.height as i32 - margin };
    let _ = win.set_position(PhysicalPosition::new(x, y));
}

fn apply_layer(win: &WebviewWindow, layer: &str) {
    let _ = win.set_always_on_top(layer == "top");
    let _ = win.set_always_on_bottom(layer == "bottom");
}

pub fn show_flyout(app: &AppHandle, s: &WidgetSettings) -> Result<(), String> {
    let win = window(app, FLYOUT, 340.0, 560.0, false)?;
    let _ = win.set_always_on_top(true);
    place(app, &win, "bottom-right");
    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;
    // A flyout closes when you click elsewhere. (Registered once per window.)
    let w2 = win.clone();
    let _ = s; // settings only decide whether a tray exists; kept for symmetry
    win.on_window_event(move |e| {
        if let WindowEvent::Focused(false) = e {
            let _ = w2.hide();
        }
    });
    Ok(())
}

pub fn toggle_flyout(app: &AppHandle, s: &WidgetSettings) {
    if let Some(win) = app.get_webview_window(FLYOUT) {
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
            return;
        }
    }
    let _ = show_flyout(app, s);
}

pub fn show_floating(app: &AppHandle, s: &WidgetSettings) -> Result<(), String> {
    let win = window(app, FLOAT, 320.0, 560.0, true)?;
    apply_layer(&win, &s.layer);
    place(app, &win, &s.corner);
    win.show().map_err(|e| e.to_string())
}

pub fn toggle_floating(app: &AppHandle, s: &WidgetSettings) {
    if let Some(win) = app.get_webview_window(FLOAT) {
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
            return;
        }
    }
    let _ = show_floating(app, s);
}

pub fn hide(app: &AppHandle, label: &str) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.hide();
    }
}

fn menu(app: &AppHandle, open: &str, widget: &str, quit: &str) -> tauri::Result<Menu<tauri::Wry>> {
    Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, "open", open, true, None::<&str>)?,
            &MenuItem::with_id(app, "widget", widget, true, None::<&str>)?,
            &MenuItem::with_id(app, "quit", quit, true, None::<&str>)?,
        ],
    )
}

/// Labels come from the UI language, so the tray menu is rebuilt when they change.
pub fn set_tray_labels(app: &AppHandle, open: &str, widget: &str, quit: &str) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        if let Ok(m) = menu(app, open, widget, quit) {
            let _ = tray.set_menu(Some(m));
        }
    }
}

pub fn open_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Creates the tray icon (hidden until the flyout is enabled).
pub fn build_tray(app: &AppHandle, current: impl Fn() -> WidgetSettings + Send + Sync + Clone + 'static) -> tauri::Result<()> {
    let click = current.clone();
    let menu_current = current;
    let mut b = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("WinRice")
        .menu(&menu(app, "Open WinRice", "Show widget", "Quit")?)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| {
            let app = app.clone();
            let id = event.id.as_ref().to_string();
            let s = menu_current();
            // Creating windows must not happen on the event thread (it can deadlock on Windows).
            std::thread::spawn(move || match id.as_str() {
                "open" => open_main(&app),
                "widget" => toggle_floating(&app, &s),
                "quit" => app.exit(0),
                _ => {}
            });
        })
        .on_tray_icon_event(move |tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                let (app, s) = (tray.app_handle().clone(), click());
                std::thread::spawn(move || toggle_flyout(&app, &s));
            }
        });
    if let Some(icon) = app.default_window_icon() {
        b = b.icon(icon.clone());
    }
    b.build(app)?;
    Ok(())
}
