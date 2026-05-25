#![windows_subsystem = "windows"]
use eframe::egui;

use gh_stream_listner::app::fonts;
use gh_stream_listner::app::GhStreamApp;
use gh_stream_listner::APP_TITLE;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(APP_TITLE)
            .with_icon(app_icon()),
        ..Default::default()
    };

    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(|cc| {
            fonts::install_fonts(&cc.egui_ctx);
            Ok(Box::new(GhStreamApp::new()))
        }),
    )
}

fn app_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
        .expect("embedded app icon should be a valid PNG")
}
