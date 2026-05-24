use eframe::egui;

use crate::app::screens::stream::StreamState;
use crate::app::StatusEntry;

#[derive(Default)]
pub struct StatusLogState {
    pub(in crate::app) open: bool,
}

pub(in crate::app) fn show(
    ctx: &egui::Context,
    state: &mut StreamState,
    status_history: &[StatusEntry],
) {
    egui::TopBottomPanel::top("status-log-toolbar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading("Status log");
            ui.separator();
            if ui.button("Back").clicked() {
                state.status_log.open = false;
            }
        });
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("Recent messages");
        ui.add_space(8.0);

        if status_history.is_empty() {
            ui.label("No status messages yet.");
            return;
        }

        egui::ScrollArea::vertical()
            .id_salt("status-log-scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (index, entry) in status_history.iter().rev().enumerate() {
                    let prefix = if index == 0 { "Latest" } else { "Earlier" };
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.label(egui::RichText::new(prefix).strong());
                        ui.add_space(4.0);
                        ui.label(&entry.message);
                    });
                    ui.add_space(8.0);
                }
            });
    });
}
