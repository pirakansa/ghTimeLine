use eframe::egui;

use gh_stream_listner::{HelloWorldApp, APP_TITLE};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title(APP_TITLE),
        ..Default::default()
    };

    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(|_cc| Ok(Box::new(HelloWorldApp))),
    )
}
