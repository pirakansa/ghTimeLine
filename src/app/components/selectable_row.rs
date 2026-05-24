use eframe::egui;

pub fn show(
    ui: &mut egui::Ui,
    selected: bool,
    label: &str,
    trailing_count: Option<i64>,
) -> egui::Response {
    let row_height = ui.spacing().interact_size.y;
    let available_width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(available_width, row_height),
        egui::Sense::click(),
    );

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact_selectable(&response, selected);
        if selected || response.hovered() {
            ui.painter()
                .rect_filled(rect, visuals.corner_radius, visuals.bg_fill);
        }

        let px = ui.spacing().button_padding.x;
        let font_id = egui::TextStyle::Body.resolve(ui.style());
        let text_color = visuals.text_color();

        if let Some(count) = trailing_count.filter(|count| *count > 0) {
            let galley =
                ui.painter()
                    .layout_no_wrap(count.to_string(), font_id.clone(), text_color);
            let pos = egui::pos2(
                rect.right() - px - galley.size().x,
                rect.center().y - galley.size().y / 2.0,
            );
            ui.painter().galley(pos, galley, text_color);
        }

        ui.painter().text(
            egui::pos2(rect.left() + px, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            font_id,
            text_color,
        );
    }

    let label = label.to_owned();
    response.widget_info(move || {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, label.clone())
    });

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}
