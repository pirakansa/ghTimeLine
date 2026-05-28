use eframe::egui;

use crate::app::components::status_icon;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ActionIcon {
    MarkRead,
    MarkUnread,
    Bookmark(bool),
    Archive(bool),
}

pub(super) fn action_icon_button(
    ui: &mut egui::Ui,
    icon: ActionIcon,
    accessible_label: &'static str,
) -> egui::Response {
    let icon_size = (status_icon::size_for_ui(ui) * 0.9).clamp(16.0, 22.0);
    let button_size = (icon_size + 8.0).clamp(22.0, 30.0);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(button_size, button_size), egui::Sense::click());

    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), accessible_label)
    });

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let visuals = ui.style().interact(&response);
        let corner_radius = 4.0;
        let frame_rect = rect.shrink(1.0);

        if response.hovered() || response.has_focus() || response.is_pointer_button_down_on() {
            painter.rect(
                frame_rect,
                corner_radius,
                visuals.weak_bg_fill,
                visuals.bg_stroke,
                egui::StrokeKind::Inside,
            );
        }

        draw_action_icon(
            painter,
            frame_rect.shrink2(egui::vec2(4.0, 4.0)),
            icon,
            visuals.fg_stroke.color,
        );
    }

    response.on_hover_text(accessible_label)
}

fn draw_action_icon(
    painter: &egui::Painter,
    rect: egui::Rect,
    icon: ActionIcon,
    color: egui::Color32,
) {
    match icon {
        ActionIcon::MarkRead => draw_mark_read_icon(painter, rect, color),
        ActionIcon::MarkUnread => draw_mark_unread_icon(painter, rect, color),
        ActionIcon::Bookmark(active) => draw_bookmark_icon(painter, rect, color, active),
        ActionIcon::Archive(active) => draw_archive_icon(painter, rect, color, active),
    }
}

fn draw_mark_read_icon(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let radius = rect.width().min(rect.height()) * 0.42;
    let stroke = egui::Stroke::new(stroke_width(rect, 0.10), color);
    painter.circle_stroke(rect.center(), radius, stroke);
    painter.line_segment(
        [lerp_point(rect, 0.24, 0.54), lerp_point(rect, 0.43, 0.72)],
        stroke,
    );
    painter.line_segment(
        [lerp_point(rect, 0.43, 0.72), lerp_point(rect, 0.77, 0.30)],
        stroke,
    );
}

fn draw_mark_unread_icon(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let radius = rect.width().min(rect.height()) * 0.42;
    painter.circle_stroke(
        rect.center(),
        radius,
        egui::Stroke::new(stroke_width(rect, 0.10), color),
    );
    painter.circle_filled(rect.center(), radius * 0.34, color);
}

fn draw_bookmark_icon(
    painter: &egui::Painter,
    rect: egui::Rect,
    color: egui::Color32,
    active: bool,
) {
    let points = vec![
        lerp_point(rect, 0.28, 0.16),
        lerp_point(rect, 0.72, 0.16),
        lerp_point(rect, 0.72, 0.82),
        lerp_point(rect, 0.50, 0.63),
        lerp_point(rect, 0.28, 0.82),
    ];
    let stroke = egui::Stroke::new(stroke_width(rect, 0.10), color);

    if active {
        painter.add(egui::Shape::convex_polygon(
            points,
            color,
            egui::Stroke::NONE,
        ));
    } else {
        painter.add(egui::Shape::closed_line(points, stroke));
    }
}

fn draw_archive_icon(
    painter: &egui::Painter,
    rect: egui::Rect,
    color: egui::Color32,
    active: bool,
) {
    let rect = rect.expand(1.5);
    let stroke = egui::Stroke::new(stroke_width(rect, 0.09), color);
    let box_rect =
        egui::Rect::from_min_max(lerp_point(rect, 0.12, 0.36), lerp_point(rect, 0.88, 0.82));
    let lid_rect =
        egui::Rect::from_min_max(lerp_point(rect, 0.18, 0.20), lerp_point(rect, 0.82, 0.40));
    let corner_radius = rect.width().min(rect.height()) * 0.08;

    painter.rect_stroke(box_rect, corner_radius, stroke, egui::StrokeKind::Inside);
    painter.line_segment(
        [
            egui::pos2(lid_rect.left(), lid_rect.bottom()),
            egui::pos2(lid_rect.right(), lid_rect.bottom()),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(lid_rect.left(), lid_rect.bottom()),
            egui::pos2(lid_rect.left() + lid_rect.width() * 0.10, lid_rect.top()),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(lid_rect.right(), lid_rect.bottom()),
            egui::pos2(lid_rect.right() - lid_rect.width() * 0.10, lid_rect.top()),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::pos2(lid_rect.left() + lid_rect.width() * 0.10, lid_rect.top()),
            egui::pos2(lid_rect.right() - lid_rect.width() * 0.10, lid_rect.top()),
        ],
        stroke,
    );

    let (arrow_start, arrow_tip, wing_y) = if active {
        (
            lerp_point(rect, 0.50, 0.68),
            lerp_point(rect, 0.50, 0.26),
            0.40,
        )
    } else {
        (
            lerp_point(rect, 0.50, 0.26),
            lerp_point(rect, 0.50, 0.68),
            0.54,
        )
    };
    let wing_left = lerp_point(rect, 0.36, wing_y);
    let wing_right = lerp_point(rect, 0.64, wing_y);

    painter.line_segment([arrow_start, arrow_tip], stroke);
    painter.line_segment([wing_left, arrow_tip], stroke);
    painter.line_segment([wing_right, arrow_tip], stroke);
}

fn stroke_width(rect: egui::Rect, scale: f32) -> f32 {
    (rect.width().min(rect.height()) * scale).max(1.4)
}

fn lerp_point(rect: egui::Rect, x: f32, y: f32) -> egui::Pos2 {
    egui::pos2(
        egui::lerp(rect.left()..=rect.right(), x),
        egui::lerp(rect.top()..=rect.bottom(), y),
    )
}
