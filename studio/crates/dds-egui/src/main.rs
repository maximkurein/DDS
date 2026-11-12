//! DDS Studio — native desktop UI on egui (no WebView, no Node).
//! The logic lives in `dds-core`, shared with `dds-server` (web mode).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod canvas;
mod content;
mod eco;
mod fmt;
mod plot;
mod shots;
mod theme;
mod views;
mod widgets;
mod worker;

/// Same artwork as `assets/icons/icon.icns`. Without it eframe installs its
/// default egui logo as the application icon at runtime (macOS
/// `setApplicationIconImage`), which replaces the bundle icon in the Dock.
const ICON_PNG: &[u8] = include_bytes!("../../../assets/icons/icon.png");

fn main() -> eframe::Result {
    let mut viewport = eframe::egui::ViewportBuilder::default();
    match eframe::icon_data::from_png_bytes(ICON_PNG) {
        Ok(icon) => viewport = viewport.with_icon(icon),
        Err(e) => eprintln!("app icon not loaded: {e}"),
    }
    let options = eframe::NativeOptions {
        viewport: viewport
            .with_title("DDS Studio")
            .with_app_id("com.dds-studio.egui")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1024.0, 680.0]),
        // the screenshot tour always shoots at 1440×900, not the size the user left
        persist_window: std::env::var_os("DDS_SHOTS").or_else(|| std::env::var_os("APP_SHOTS")).is_none(),
        ..Default::default()
    };
    eframe::run_native("DDS Studio", options, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}
