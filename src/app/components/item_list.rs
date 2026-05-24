use eframe::egui;

use super::{author_avatar, status_icon};
use crate::app::screens::stream::{ItemAction, StreamEvent};
use crate::models::{ItemPerson, ItemReview, StreamItem};

const PERSON_AVATAR_SIZE: f32 = 20.0;

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

    egui::ScrollArea::vertical().show(ui, |ui| {
        for item in items {
            let frame = egui::Frame::group(ui.style())
                .fill(item_background_fill(ui.visuals(), item.is_unread))
                .stroke(item_background_stroke(ui.visuals(), item.is_unread));
            frame.show(ui, |ui| {
                let available_width = ui.available_width();
                ui.set_min_width(available_width);
                ui.set_width(available_width);
                draw_item(ui, item, avatar_cache, event);
            });
            ui.add_space(6.0);
        }
    });
}

fn draw_item(
    ui: &mut egui::Ui,
    item: &StreamItem,
    avatar_cache: &mut author_avatar::AvatarCache,
    event: &mut Option<StreamEvent>,
) {
    ui.horizontal(|ui| {
        let icon = status_icon::StatusIcon::for_item(item);
        status_icon::show(ui, icon).on_hover_text(icon.label());
        ui.label(
            egui::RichText::new(format!("{} #{}", item.repository_full_name(), item.number)).weak(),
        );
        ui.label(super::relative_time::format(
            item.updated_at_github.as_str(),
        ));
    });
    let title = if item.is_unread {
        egui::RichText::new(&item.title).strong()
    } else {
        egui::RichText::new(&item.title)
    };
    ui.heading(title);
    ui.horizontal_wrapped(|ui| {
        let avatar_size = author_avatar::size_for_ui(ui);
        if let Some(author) = &item.author_login {
            author_avatar::show(
                ui,
                avatar_cache,
                item.author_avatar_url.as_deref(),
                Some(author.as_str()),
            )
            .on_hover_text(author);
        }
        if !item.assignees.is_empty() {
            ui.label(egui::RichText::new("→").weak());
            for assignee in &item.assignees {
                author_avatar::show_sized(
                    ui,
                    avatar_cache,
                    assignee.avatar_url.as_deref(),
                    Some(assignee.login.as_str()),
                    avatar_size,
                )
                .on_hover_text(&assignee.login);
            }
        }
        ui.label(format!("{} comments", item.comment_count));
    });
    metadata_rows(ui, item, avatar_cache);
    action_buttons(ui, item, event);
}

fn metadata_rows(
    ui: &mut egui::Ui,
    item: &StreamItem,
    avatar_cache: &mut author_avatar::AvatarCache,
) {
    if !item.review_requests.is_empty() || !item.reviewers.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.label("Reviewers:");
            let reviewed_logins = item
                .reviewers
                .iter()
                .map(|review| review.login.as_str())
                .collect::<std::collections::BTreeSet<_>>();
            for reviewer in &item.review_requests {
                if reviewed_logins.contains(reviewer.login.as_str()) {
                    continue;
                }
                show_person_chip(ui, avatar_cache, reviewer, Some("requested"));
            }
            for reviewer in &item.reviewers {
                show_review_chip(ui, avatar_cache, reviewer);
            }
        });
    }
    if !item.labels.is_empty() {
        ui.label(format!("Labels: {}", item.labels.join(", ")));
    }
}

fn action_buttons(ui: &mut egui::Ui, item: &StreamItem, event: &mut Option<StreamEvent>) {
    ui.horizontal(|ui| {
        read_state_button(ui, item, event);
        bookmark_button(ui, item, event);
        archive_button(ui, item, event);
        open_button(ui, item, event);
    });
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

fn open_button(ui: &mut egui::Ui, item: &StreamItem, event: &mut Option<StreamEvent>) {
    if ui.button("Open").clicked() {
        *event = Some(StreamEvent::ItemAction(ItemAction::Open(
            item.html_url.clone(),
        )));
    }
}

fn item_background_fill(visuals: &egui::Visuals, is_unread: bool) -> egui::Color32 {
    if is_unread {
        visuals.selection.bg_fill.gamma_multiply(0.22)
    } else if visuals.dark_mode {
        visuals.panel_fill.gamma_multiply(1.18)
    } else {
        visuals.panel_fill.gamma_multiply(0.97)
    }
}

fn item_background_stroke(visuals: &egui::Visuals, is_unread: bool) -> egui::Stroke {
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

fn show_person_chip(
    ui: &mut egui::Ui,
    avatar_cache: &mut author_avatar::AvatarCache,
    person: &ItemPerson,
    review_state: Option<&str>,
) {
    ui.horizontal(|ui| {
        let response = author_avatar::show_sized(
            ui,
            avatar_cache,
            person.avatar_url.as_deref(),
            Some(person.login.as_str()),
            PERSON_AVATAR_SIZE,
        )
        .on_hover_text(match review_state {
            Some(state) => format!("{} ({state})", person.login),
            None => person.login.clone(),
        });
        if let Some(state) = review_state {
            paint_review_badge(ui, response.rect, state);
        }
    });
}

fn show_review_chip(
    ui: &mut egui::Ui,
    avatar_cache: &mut author_avatar::AvatarCache,
    review: &ItemReview,
) {
    ui.horizontal(|ui| {
        let response = author_avatar::show_sized(
            ui,
            avatar_cache,
            review.avatar_url.as_deref(),
            Some(review.login.as_str()),
            PERSON_AVATAR_SIZE,
        )
        .on_hover_text(format!("{} ({})", review.login, review.state));
        paint_review_badge(ui, response.rect, &review.state);
    });
}

fn paint_review_badge(ui: &egui::Ui, avatar_rect: egui::Rect, review_state: &str) {
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

#[cfg(test)]
mod tests {
    use super::{item_background_fill, item_background_stroke};
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
