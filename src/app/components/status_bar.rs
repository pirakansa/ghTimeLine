use eframe::egui;

use crate::config;
use crate::models::AppConfig;

pub fn show(ctx: &egui::Context, config: &AppConfig, status: &str) {
    egui::TopBottomPanel::bottom("stream-status").show(ctx, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(status);
            ui.separator();
            ui.label(format!("Host: {}", config.host.name));
            ui.separator();
            ui.label(format!("PAT: {}", config::redact_pat(&config.auth.pat)));
        });
    });
}
