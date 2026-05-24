use eframe::egui;

pub(super) fn show_comment_count(ui: &mut egui::Ui, comment_count: i64) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let font_size = egui::TextStyle::Body.resolve(ui.style()).size;
        ui.label(comment_count.to_string());
        paint_comment_bubble(ui, font_size);
    });
}

pub(super) fn show_label_badge(ui: &mut egui::Ui, label: &str) {
    let visuals = ui.visuals();
    let fill = visuals
        .widgets
        .inactive
        .bg_fill
        .gamma_multiply(if visuals.dark_mode { 1.25 } else { 0.95 });
    let stroke = egui::Stroke::new(
        1.0,
        visuals
            .widgets
            .noninteractive
            .bg_stroke
            .color
            .gamma_multiply(0.8),
    );

    egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(egui::CornerRadius::same(7))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(label).small().strong());
        });
}

pub(super) fn paint_review_badge(ui: &egui::Ui, avatar_rect: egui::Rect, review_state: &str) {
    let Some((fill, kind)) = review_badge_style(review_state) else {
        return;
    };
    let radius = (avatar_rect.width() * 0.22).max(4.0);
    let center = egui::pos2(avatar_rect.right() - radius, avatar_rect.bottom() - radius);
    let painter = ui.painter();
    painter.circle_filled(center, radius, fill);

    let stroke = egui::Stroke::new((radius * 0.35).max(1.2), egui::Color32::WHITE);
    match kind {
        ReviewBadgeKind::Check => {
            painter.line_segment(
                [
                    center + egui::vec2(-radius * 0.55, 0.0),
                    center + egui::vec2(-radius * 0.15, radius * 0.4),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + egui::vec2(-radius * 0.15, radius * 0.4),
                    center + egui::vec2(radius * 0.6, -radius * 0.45),
                ],
                stroke,
            );
        }
        ReviewBadgeKind::Exclamation => {
            painter.line_segment(
                [
                    center + egui::vec2(0.0, -radius * 0.58),
                    center + egui::vec2(0.0, radius * 0.1),
                ],
                stroke,
            );
            painter.circle_filled(
                center + egui::vec2(0.0, radius * 0.55),
                radius * 0.16,
                egui::Color32::WHITE,
            );
        }
        ReviewBadgeKind::Plus => {
            painter.line_segment(
                [
                    center + egui::vec2(-radius * 0.55, 0.0),
                    center + egui::vec2(radius * 0.55, 0.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    center + egui::vec2(0.0, -radius * 0.55),
                    center + egui::vec2(0.0, radius * 0.55),
                ],
                stroke,
            );
        }
        ReviewBadgeKind::Dots => {
            for offset in [-0.45_f32, 0.0, 0.45] {
                painter.circle_filled(
                    center + egui::vec2(radius * offset, 0.0),
                    radius * 0.16,
                    egui::Color32::WHITE,
                );
            }
        }
    }
}

fn paint_comment_bubble(ui: &mut egui::Ui, font_size: f32) {
    let size = font_size * 1.1;
    let tail = size * 0.28;
    let total_h = size + tail;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, total_h), egui::Sense::hover());

    let bubble = egui::Rect::from_min_size(rect.min, egui::vec2(size, size));
    let color = ui
        .visuals()
        .widgets
        .noninteractive
        .fg_stroke
        .color
        .gamma_multiply(0.55);
    let rounding = egui::CornerRadius::same((size * 0.22) as u8);
    ui.painter().rect_filled(bubble, rounding, color);

    let base_x = bubble.left() + size * 0.22;
    let base_y = bubble.bottom();
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(base_x, base_y),
            egui::pos2(base_x + tail, base_y),
            egui::pos2(base_x, base_y + tail),
        ],
        color,
        egui::Stroke::NONE,
    ));
}

fn review_badge_style(review_state: &str) -> Option<(egui::Color32, ReviewBadgeKind)> {
    match review_state {
        "approved" => Some((egui::Color32::from_rgb(31, 136, 61), ReviewBadgeKind::Check)),
        "changes_requested" => Some((
            egui::Color32::from_rgb(217, 119, 6),
            ReviewBadgeKind::Exclamation,
        )),
        "commented" => Some((egui::Color32::from_rgb(9, 105, 218), ReviewBadgeKind::Plus)),
        "requested" => Some((
            egui::Color32::from_rgb(101, 109, 118),
            ReviewBadgeKind::Dots,
        )),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReviewBadgeKind {
    Check,
    Exclamation,
    Plus,
    Dots,
}
