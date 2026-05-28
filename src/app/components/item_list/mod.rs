use std::collections::HashMap;

use eframe::egui;

use super::{author_avatar, status_icon};
use crate::app::screens::stream::{ItemAction, StreamEvent};
use crate::models::{ItemType, StreamItem};

mod action_icons;
mod badges;
mod people;
mod styles;

use action_icons::{action_icon_button, ActionIcon};

const ITEM_GAP: f32 = 6.0;

#[derive(Default)]
pub struct ItemListState {
    measured_heights: HashMap<i64, f32>,
    measurement_width: Option<f32>,
    estimated_height: Option<f32>,
}

fn estimated_row_height(ui: &egui::Ui) -> f32 {
    ui.spacing().interact_size.y * 7.0
}

pub fn show(
    ui: &mut egui::Ui,
    items: &[StreamItem],
    avatar_cache: &mut author_avatar::AvatarCache,
    state: &mut ItemListState,
    reset_scroll_to_top: &mut bool,
    event: &mut Option<StreamEvent>,
) {
    if items.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label("0 items");
        });
        return;
    }

    let estimate = estimated_row_height(ui);

    let mut scroll_area = egui::ScrollArea::vertical();
    if std::mem::take(reset_scroll_to_top) {
        scroll_area = scroll_area.vertical_scroll_offset(0.0);
    }

    scroll_area.show_viewport(ui, |ui, viewport| {
        state.prepare(items, ui.available_width(), estimate);
        let mut positions = state.positions(items, estimate);
        let mut content_height = positions.last().map_or(0.0, |row| row.top + row.height);
        ui.set_height(content_height);

        for row in 0..positions.len() {
            let position = positions[row];
            if position.bottom() < viewport.top() || position.top > viewport.bottom() {
                continue;
            }

            let item = &items[row];
            let width = ui.available_width();
            let top_left = ui.max_rect().left_top() + egui::vec2(0.0, position.top);
            let inner = ui.scope_builder(
                egui::UiBuilder::new()
                    .id_salt(("item", item.id))
                    .max_rect(egui::Rect::from_min_size(top_left, egui::vec2(width, 0.0)))
                    .layout(egui::Layout::top_down(egui::Align::Min)),
                |ui| show_row(ui, item, avatar_cache, event),
            );
            let measured_height = inner.response.rect.height();
            let height_delta = measured_height - position.height;
            state.measured_heights.insert(item.id, measured_height);
            positions[row].height = measured_height;
            if height_delta != 0.0 {
                content_height += height_delta;
                for following in positions.iter_mut().skip(row + 1) {
                    following.top += height_delta;
                }
                if height_delta > 0.0 {
                    ui.set_min_height(content_height);
                }
            }
        }
    });
}

#[derive(Clone, Copy)]
struct RowPosition {
    top: f32,
    height: f32,
}

impl RowPosition {
    fn bottom(self) -> f32 {
        self.top + self.height
    }
}

impl ItemListState {
    fn prepare(&mut self, items: &[StreamItem], width: f32, estimate: f32) {
        let dimensions_changed = self
            .measurement_width
            .is_some_and(|last| (last - width).abs() > f32::EPSILON)
            || self
                .estimated_height
                .is_some_and(|last| (last - estimate).abs() > f32::EPSILON);
        if dimensions_changed {
            self.measured_heights.clear();
        } else {
            self.measured_heights
                .retain(|id, _| items.iter().any(|item| item.id == *id));
        }
        self.measurement_width = Some(width);
        self.estimated_height = Some(estimate);
    }

    fn positions(&self, items: &[StreamItem], estimate: f32) -> Vec<RowPosition> {
        let mut top = 0.0;
        items
            .iter()
            .map(|item| {
                let height = self
                    .measured_heights
                    .get(&item.id)
                    .copied()
                    .unwrap_or(estimate);
                let position = RowPosition { top, height };
                top += height + ITEM_GAP;
                position
            })
            .collect()
    }
}

fn show_row(
    ui: &mut egui::Ui,
    item: &StreamItem,
    avatar_cache: &mut author_avatar::AvatarCache,
    event: &mut Option<StreamEvent>,
) {
    let frame = egui::Frame::group(ui.style())
        .fill(styles::item_background_fill(ui.visuals(), item.is_unread))
        .stroke(styles::item_background_stroke(ui.visuals(), item.is_unread));
    let response = frame.show(ui, |ui| {
        let available_width = ui.available_width();
        ui.set_width(available_width);
        show_item_card(ui, item, avatar_cache, event);
    });
    let card_rect = response.response.rect;
    let visible_rect = card_rect.intersect(ui.clip_rect());
    let is_hovered = ui.input(|input| {
        input
            .pointer
            .hover_pos()
            .is_some_and(|pos| visible_rect.contains(pos))
    });

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
}

fn show_item_card(
    ui: &mut egui::Ui,
    item: &StreamItem,
    avatar_cache: &mut author_avatar::AvatarCache,
    event: &mut Option<StreamEvent>,
) {
    show_header_row(ui, item, event);
    show_title(ui, item);
    let avatar_size = author_avatar::size_for_ui(ui);
    people::show_author_and_assignees_row(ui, item, avatar_cache, avatar_size, event);
    show_metadata_rows(ui, item, avatar_cache, avatar_size, event);
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

fn show_header_row(ui: &mut egui::Ui, item: &StreamItem, event: &mut Option<StreamEvent>) {
    ui.horizontal(|ui| {
        let icon = status_icon::StatusIcon::for_item(item);
        let type_filter = match item.item_type {
            ItemType::Issue => "is:issue",
            ItemType::PullRequest => "is:pr",
            ItemType::Discussion => "is:discussion",
        };
        let response = status_icon::show_clickable(ui, icon);
        filter_control(ui, response, type_filter, event);
        let repository_filter = format!("repo:{}", item.repository_full_name());
        let response = ui.add(
            egui::Label::new(
                egui::RichText::new(format!("{} #{}", item.repository_full_name(), item.number))
                    .weak(),
            )
            .sense(egui::Sense::click()),
        );
        filter_control(ui, response, &repository_filter, event);
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

fn filter_control(
    ui: &mut egui::Ui,
    response: egui::Response,
    term: &str,
    event: &mut Option<StreamEvent>,
) {
    let label = format!("Add to local filter: {term}");
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &label)
    });
    if response.on_hover_text(&label).clicked() {
        *event = Some(StreamEvent::AddLocalFilterInputTerm(term.to_owned()));
    }
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
    event: &mut Option<StreamEvent>,
) {
    people::show_reviewer_row(ui, item, avatar_cache, avatar_size, event);
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
