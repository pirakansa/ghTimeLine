use eframe::egui;

pub(super) fn item_background_fill(visuals: &egui::Visuals, is_unread: bool) -> egui::Color32 {
    if is_unread {
        visuals.selection.bg_fill.gamma_multiply(0.22)
    } else if visuals.dark_mode {
        visuals.panel_fill.gamma_multiply(1.18)
    } else {
        visuals.panel_fill.gamma_multiply(0.97)
    }
}

pub(super) fn item_background_stroke(visuals: &egui::Visuals, is_unread: bool) -> egui::Stroke {
    let color = if is_unread {
        visuals.selection.bg_fill.gamma_multiply(0.55)
    } else {
        visuals
            .widgets
            .noninteractive
            .bg_stroke
            .color
            .gamma_multiply(0.45)
    };
    egui::Stroke::new(1.0, color)
}
