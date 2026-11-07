#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// A release build compiled without Tauri's `custom-protocol` feature (for example a plain
// `cargo build --release`) does not embed the UI: it tries to load the dev server at
// http://localhost:1420 and shows "localhost refused to connect". Refuse to build that.
#[cfg(all(dev, not(debug_assertions)))]
compile_error!(
    "This release build would not contain the UI (it would load http://localhost:1420). \
     Build with `npm --prefix app run tauri -- build --no-bundle` instead of `cargo build --release`."
);

fn main() {
    winrice_lib::run()
}
