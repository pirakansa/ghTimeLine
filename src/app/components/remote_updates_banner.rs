use eframe::egui;

use crate::app::screens::stream::StreamEvent;

pub fn show(ui: &mut egui::Ui, updated_item_count: usize, event: &mut Option<StreamEvent>) {
    if updated_item_count == 0 {
        return;
    }

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(format!(
                "{updated_item_count} updated item{} available.",
                if updated_item_count == 1 { "" } else { "s" }
            ));
            if ui.button("Show updates").clicked() {
                *event = Some(StreamEvent::ShowRemoteUpdates);
            }
        });
    });
    ui.add_space(6.0);
}
