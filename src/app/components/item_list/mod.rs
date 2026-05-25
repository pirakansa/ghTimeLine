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
    event: &mut Option<StreamEvent>,
) {
    if items.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label("0 items");
        });
        return;
    }

    egui::ScrollArea::vertical().show_rows(
        ui,
        estimated_row_height(ui),
        items.len(),
        |ui, rows| {
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
        },
    );
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
        if ui.button("Mark read").clicked() {
            *event = Some(StreamEvent::ItemAction(ItemAction::MarkRead(item.id)));
        }
    } else if ui.button("Mark unread").clicked() {
        *event = Some(StreamEvent::ItemAction(ItemAction::MarkUnread(item.id)));
    }
}

fn bookmark_button(ui: &mut egui::Ui, item: &StreamItem, event: &mut Option<StreamEvent>) {
    let bookmark_label = if item.is_bookmarked {
        "Remove bookmark"
    } else {
        "Bookmark"
    };
    if ui.button(bookmark_label).clicked() {
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
    if ui.button(archive_label).clicked() {
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
