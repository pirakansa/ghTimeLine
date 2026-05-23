use eframe::egui;

use gh_stream_listner::app::fonts;
use gh_stream_listner::app::GhStreamApp;
use gh_stream_listner::APP_TITLE;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title(APP_TITLE),
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
