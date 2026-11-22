#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::ViewportBuilder;
use openlauncher::app::OpenLauncherApp;
use openlauncher::{theme, tray};

fn main() -> eframe::Result {
    let mut viewport = ViewportBuilder::default()
        .with_title("OpenLauncher")
        .with_decorations(false)
        .with_inner_size([1320.0, 840.0])
        .with_min_inner_size([960.0, 640.0]);

    if let Some(icon) = tray::load_egui_app_icon() {
        viewport = viewport.with_icon(std::sync::Arc::new(icon));
    }

    let options = eframe::NativeOptions {
        viewport,
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "OpenLauncher",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            theme::configure_theme_and_fonts(&cc.egui_ctx);

            let win32_hwnd = tray::extract_win32_hwnd(cc);
            if let Some(hwnd) = win32_hwnd {
                tray::configure_windows_frame(hwnd);
            }

            let app = OpenLauncherApp::new(cc, win32_hwnd).map_err(|err| {
                Box::<dyn std::error::Error + Send + Sync>::from(err.to_string())
            })?;
            Ok(Box::new(app))
        }),
    )
}
