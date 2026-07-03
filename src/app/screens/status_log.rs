use eframe::egui;

use crate::app::screens::stream::StreamState;
use crate::app::{StatusEntry, StatusLevel};

#[derive(Default)]
pub struct StatusLogState {
    pub(in crate::app) open: bool,
}

pub(in crate::app) fn show(
    ui: &mut egui::Ui,
    state: &mut StreamState,
    status_history: &[StatusEntry],
) {
    egui::Panel::top("status-log-toolbar").show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.heading("Status log");
            ui.separator();
            if ui.button("Back").clicked() {
                state.status_log.open = false;
            }
        });
    });

    egui::CentralPanel::default().show(ui, |ui| {
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
                    let is_error = entry.level == StatusLevel::Error;
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        let prefix_color = if is_error {
                            egui::Color32::from_rgb(220, 50, 50)
                        } else {
                            ui.visuals().text_color()
                        };
                        ui.label(egui::RichText::new(prefix).strong().color(prefix_color));
                        ui.add_space(4.0);
                        ui.label(&entry.message);
                    });
                    ui.add_space(8.0);
                }
            });
    });
}
