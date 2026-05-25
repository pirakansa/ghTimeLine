use eframe::egui;

use super::{author_avatar, status_icon};
use crate::app::screens::stream::{ItemAction, StreamEvent};
use crate::models::StreamItem;

mod badges;
mod people;
mod styles;

fn estimated_row_height(ui: &egui::Ui) -> f32 {
    ui.spacing().interact_size.y * 7.0
}

pub fn show(
    ui: &mut egui::Ui,
    items: &[StreamItem],
    avatar_cache: &mut author_avatar::AvatarCache,
    reset_scroll_to_top: &mut bool,
    event: &mut Option<StreamEvent>,
) {
    if items.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label("0 items");
        });
        return;
    }

    let mut scroll_area = egui::ScrollArea::vertical();
    if std::mem::take(reset_scroll_to_top) {
        scroll_area = scroll_area.vertical_scroll_offset(0.0);
    }

    scroll_area.show_rows(ui, estimated_row_height(ui), items.len(), |ui, rows| {
        for row in rows {
            let item = &items[row];
            let inner = ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), estimated_row_height(ui)),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    let frame = egui::Frame::group(ui.style())
                        .fill(styles::item_background_fill(ui.visuals(), item.is_unread))
                        .stroke(styles::item_background_stroke(ui.visuals(), item.is_unread));
                    let response = frame.show(ui, |ui| {
                        let available_width = ui.available_width();
                        ui.set_min_width(available_width);
                        ui.set_width(available_width);
                        show_item_card(ui, item, avatar_cache);
                    });
                    let card_rect = response.response.rect;
                    let visible_rect = card_rect.intersect(ui.clip_rect());
                    let is_hovered = ui.input(|input| {
                        input
                            .pointer
                            .hover_pos()
                            .is_some_and(|pos| visible_rect.contains(pos))
                    });
                    (card_rect, visible_rect, is_hovered)
                },
            );

            let (card_rect, visible_rect, is_hovered) = inner.inner;

            if is_hovered {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                show_action_overlay(ui.ctx(), item, card_rect, event);
            }

            let is_clicked = ui.input(|input| {
                input.pointer.primary_clicked()
                    && input
                        .pointer
                        .interact_pos()
                        .is_some_and(|pos| visible_rect.contains(pos))
            });
            if is_clicked && event.is_none() {
                open_item(item, event);
            }

            if row + 1 < items.len() {
                ui.add_space(6.0);
            }
        }
    });
}

fn show_item_card(
    ui: &mut egui::Ui,
    item: &StreamItem,
    avatar_cache: &mut author_avatar::AvatarCache,
) {
    show_header_row(ui, item);
    show_title(ui, item);
    let avatar_size = author_avatar::size_for_ui(ui);
    people::show_author_and_assignees_row(ui, item, avatar_cache, avatar_size);
    show_metadata_rows(ui, item, avatar_cache, avatar_size);
}

fn show_action_overlay(
    ctx: &egui::Context,
    item: &StreamItem,
    card_rect: egui::Rect,
    event: &mut Option<StreamEvent>,
) {
    let area_id = egui::Id::new("item_action_overlay").with(item.id);
    egui::Area::new(area_id)
        .order(egui::Order::Foreground)
        .pivot(egui::Align2::RIGHT_TOP)
        .fixed_pos(card_rect.right_top() + egui::vec2(-4.0, 4.0))
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(ui.visuals().window_fill)
                .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                .corner_radius(4.0)
                .inner_margin(egui::Margin::same(4))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        read_state_button(ui, item, event);
                        bookmark_button(ui, item, event);
                        archive_button(ui, item, event);
                    });
                });
        });
}

fn show_header_row(ui: &mut egui::Ui, item: &StreamItem) {
    ui.horizontal(|ui| {
        let icon = status_icon::StatusIcon::for_item(item);
        status_icon::show(ui, icon).on_hover_text(icon.label());
        ui.label(
            egui::RichText::new(format!("{} #{}", item.repository_full_name(), item.number)).weak(),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(super::relative_time::format(
                    item.updated_at_github.as_str(),
                ))
                .weak(),
            );
        });
    });
}

fn show_title(ui: &mut egui::Ui, item: &StreamItem) {
    let title = if item.is_unread {
        egui::RichText::new(&item.title).strong()
    } else {
        egui::RichText::new(&item.title)
    };
    ui.heading(title);
}

fn show_metadata_rows(
    ui: &mut egui::Ui,
    item: &StreamItem,
    avatar_cache: &mut author_avatar::AvatarCache,
    avatar_size: f32,
) {
    people::show_reviewer_row(ui, item, avatar_cache, avatar_size);
    if !item.labels.is_empty() {
        ui.horizontal_wrapped(|ui| {
            for label in &item.labels {
                badges::show_label_badge(ui, label);
            }
        });
    }
}

fn read_state_button(ui: &mut egui::Ui, item: &StreamItem, event: &mut Option<StreamEvent>) {
    if item.is_unread {
        if action_icon_button(ui, ActionIcon::MarkRead, "Mark read").clicked() {
            *event = Some(StreamEvent::ItemAction(ItemAction::MarkRead(item.id)));
        }
    } else if action_icon_button(ui, ActionIcon::MarkUnread, "Mark unread").clicked() {
        *event = Some(StreamEvent::ItemAction(ItemAction::MarkUnread(item.id)));
    }
}

fn bookmark_button(ui: &mut egui::Ui, item: &StreamItem, event: &mut Option<StreamEvent>) {
    let bookmark_label = if item.is_bookmarked {
        "Remove bookmark"
    } else {
        "Bookmark"
    };
    if action_icon_button(ui, ActionIcon::Bookmark(item.is_bookmarked), bookmark_label).clicked() {
        *event = Some(StreamEvent::ItemAction(ItemAction::Bookmark(
            item.id,
            !item.is_bookmarked,
        )));
    }
}

fn archive_button(ui: &mut egui::Ui, item: &StreamItem, event: &mut Option<StreamEvent>) {
    let archive_label = if item.is_archived {
        "Unarchive"
    } else {
        "Archive"
    };
    if action_icon_button(ui, ActionIcon::Archive(item.is_archived), archive_label).clicked() {
        *event = Some(StreamEvent::ItemAction(ItemAction::Archive(
            item.id,
            !item.is_archived,
        )));
    }
}

fn open_item(item: &StreamItem, event: &mut Option<StreamEvent>) {
    *event = Some(StreamEvent::ItemAction(ItemAction::Open {
        id: item.id,
        url: item.html_url.clone(),
    }));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActionIcon {
    MarkRead,
    MarkUnread,
    Bookmark(bool),
    Archive(bool),
}

fn action_icon_button(
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
    let stroke = egui::Stroke::new(stroke_width(rect, 0.09), color);
    let box_rect =
        egui::Rect::from_min_max(lerp_point(rect, 0.18, 0.28), lerp_point(rect, 0.82, 0.78));
    let lid_y = egui::lerp(rect.top()..=rect.bottom(), 0.34);
    let slot_left = egui::lerp(rect.left()..=rect.right(), 0.38);
    let slot_right = egui::lerp(rect.left()..=rect.right(), 0.62);

    painter.rect_stroke(box_rect, 2.0, stroke, egui::StrokeKind::Inside);
    painter.line_segment(
        [
            egui::pos2(box_rect.left(), lid_y),
            egui::pos2(box_rect.right(), lid_y),
        ],
        stroke,
    );
    painter.line_segment(
        [egui::pos2(slot_left, lid_y), egui::pos2(slot_right, lid_y)],
        stroke,
    );

    let arrow_tip = if active {
        lerp_point(rect, 0.50, 0.20)
    } else {
        lerp_point(rect, 0.50, 0.86)
    };
    let arrow_base = if active {
        lerp_point(rect, 0.50, 0.52)
    } else {
        lerp_point(rect, 0.50, 0.48)
    };
    let wing_left = if active {
        lerp_point(rect, 0.36, 0.34)
    } else {
        lerp_point(rect, 0.36, 0.70)
    };
    let wing_right = if active {
        lerp_point(rect, 0.64, 0.34)
    } else {
        lerp_point(rect, 0.64, 0.70)
    };

    painter.line_segment([arrow_base, arrow_tip], stroke);
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

#[cfg(test)]
mod tests {
    use super::styles::{item_background_fill, item_background_stroke};
    use eframe::egui;

    #[test]
    fn unread_backgrounds_are_distinct_in_both_themes() {
        for visuals in [egui::Visuals::light(), egui::Visuals::dark()] {
            let unread_fill = item_background_fill(&visuals, true);
            let read_fill = item_background_fill(&visuals, false);

            assert_ne!(unread_fill, read_fill);
            assert_ne!(unread_fill, visuals.panel_fill);
        }
    }

    #[test]
    fn unread_strokes_use_accent_color() {
        for visuals in [egui::Visuals::light(), egui::Visuals::dark()] {
            let unread_stroke = item_background_stroke(&visuals, true);
            let read_stroke = item_background_stroke(&visuals, false);

            assert_ne!(unread_stroke.color, read_stroke.color);
        }
    }
}
