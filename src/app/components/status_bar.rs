use eframe::egui;

use crate::app::screens::stream::StreamState;
use crate::app::{StatusEntry, StatusLevel};

pub(in crate::app) fn show(
    ctx: &egui::Context,
    state: &mut StreamState,
    status_history: &[StatusEntry],
) {
    let bar_height = ctx.style().spacing.interact_size.y + 4.0;
    let latest = status_history.last();
    let current_status = latest
        .map(|entry| entry.message.as_str())
        .unwrap_or("No status messages yet.");
    let is_error = latest
        .map(|e| e.level == StatusLevel::Error)
        .unwrap_or(false);

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
                    let response = show_status_button(ui, is_error)
                        .on_hover_text(format!("{current_status}\nClick to open the status log."));
                    if response.clicked() {
                        state.status_log.open = true;
                    }
                },
            );
        });
}

fn show_status_button(ui: &mut egui::Ui, is_error: bool) -> egui::Response {
    let height = (ui.spacing().interact_size.y - 4.0).max(20.0);
    let desired_size = egui::vec2(height, height);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        let base_color = if is_error {
            egui::Color32::from_rgb(220, 50, 50)
        } else {
            visuals.fg_stroke.color
        };
        let stroke = egui::Stroke::new(1.6, base_color);
        let center = rect.center();
        let radius = rect.height() * 0.34;
        let painter = ui.painter();

        if is_error {
            // Warning triangle (⚠)
            let top = egui::pos2(center.x, center.y - radius);
            let bot_left = egui::pos2(center.x - radius * 0.9, center.y + radius * 0.7);
            let bot_right = egui::pos2(center.x + radius * 0.9, center.y + radius * 0.7);
            painter.add(egui::Shape::closed_line(
                vec![top, bot_left, bot_right],
                stroke,
            ));
            // Exclamation body
            painter.line_segment(
                [
                    egui::pos2(center.x, center.y - radius * 0.4),
                    egui::pos2(center.x, center.y + radius * 0.2),
                ],
                stroke,
            );
            // Exclamation dot
            painter.circle_filled(
                egui::pos2(center.x, center.y + radius * 0.5),
                (radius * 0.18).max(1.4),
                base_color,
            );
        } else {
            // Info circle (ⓘ)
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
                base_color,
            );
        }
    }

    response
}
