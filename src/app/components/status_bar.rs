use eframe::egui;

use crate::app::screens::stream::StreamState;
use crate::app::StatusEntry;

pub(in crate::app) fn show(
    ctx: &egui::Context,
    state: &mut StreamState,
    status_history: &[StatusEntry],
) {
    let bar_height = ctx.style().spacing.interact_size.y + 4.0;
    let current_status = status_history
        .last()
        .map(|entry| entry.message.as_str())
        .unwrap_or("No status messages yet.");
    egui::TopBottomPanel::bottom("stream-status")
        .exact_height(bar_height)
        .show(ctx, |ui| {
            let available_size = egui::vec2(ui.available_width(), ui.available_height());
            let (bar_rect, _) = ui.allocate_exact_size(available_size, egui::Sense::click());

            // Consume the whole footer area so clicks never leak into the item list behind it.
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(bar_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                |ui| {
                    let response = show_status_button(ui)
                        .on_hover_text(format!("{current_status}\nClick to open the status log."));
                    if response.clicked() {
                        state.status_log.open = true;
                    }
                },
            );
        });
}

fn show_status_button(ui: &mut egui::Ui) -> egui::Response {
    let height = (ui.spacing().interact_size.y - 4.0).max(20.0);
    let desired_size = egui::vec2(height, height);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        let stroke = egui::Stroke::new(1.6, visuals.fg_stroke.color);
        let center = rect.center();
        let radius = rect.height() * 0.34;
        let painter = ui.painter();

        painter.circle_stroke(center, radius, stroke);
        painter.line_segment(
            [
                egui::pos2(center.x, center.y - radius * 0.36),
                egui::pos2(center.x, center.y + radius * 0.18),
            ],
            stroke,
        );
        painter.circle_filled(
            egui::pos2(center.x, center.y - radius * 0.62),
            (radius * 0.18).max(1.4),
            visuals.fg_stroke.color,
        );
    }

    response
}
