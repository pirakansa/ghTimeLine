use eframe::egui;

use crate::app::screens::saved_query_manager::SavedQueryManagerState;
use crate::app::screens::stream::StreamEvent;

pub fn show(
    ui: &mut egui::Ui,
    state: &mut SavedQueryManagerState,
    event: &mut Option<StreamEvent>,
) {
    egui::Panel::top("saved-query-transfer-toolbar").show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.heading("Import / export");
            ui.separator();
            if ui.button("Back").clicked() {
                state.transfer_open = false;
            }
        });
    });

    egui::CentralPanel::default().show(ui, |ui| {
        ui.vertical(|ui| {
            ui.set_min_width(360.0);
            ui.label("YAML file");
            ui.text_edit_singleline(&mut state.transfer_path);
            ui.label(
                "Export writes the current host plus saved query and filter stream definitions only.",
            );
            ui.label(
                "Import replaces this host's saved queries and clears cached matches until the next refresh.",
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Export").clicked() {
                    *event = Some(StreamEvent::ExportQueries(state.transfer_path.clone()));
                }
                if ui.button("Import").clicked() {
                    *event = Some(StreamEvent::ImportQueries(state.transfer_path.clone()));
                }
            });
        });
    });
}
