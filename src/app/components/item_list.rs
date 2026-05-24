use eframe::egui;

use super::status_icon;
use crate::app::screens::stream::{ItemAction, StreamEvent};
use crate::models::StreamItem;

pub fn show(ui: &mut egui::Ui, items: &[StreamItem], event: &mut Option<StreamEvent>) {
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
                draw_item(ui, item, event);
            });
            ui.add_space(6.0);
        }
    });
}

fn draw_item(ui: &mut egui::Ui, item: &StreamItem, event: &mut Option<StreamEvent>) {
    ui.horizontal(|ui| {
        let icon = status_icon::StatusIcon::for_item(item);
        status_icon::show(ui, icon).on_hover_text(icon.label());
        ui.label(
            egui::RichText::new(format!("{} #{}", item.repository_full_name(), item.number)).weak(),
        );
        ui.label(item.updated_at_github.as_str());
    });
    let title = if item.is_unread {
        egui::RichText::new(&item.title).strong()
    } else {
        egui::RichText::new(&item.title)
    };
    ui.heading(title);
    ui.horizontal_wrapped(|ui| {
        if let Some(author) = &item.author_login {
            ui.label(format!("by {author}"));
        }
        ui.label(format!("{} comments", item.comment_count));
        if let Some(review_status) = &item.review_status {
            ui.label(format!("review: {review_status}"));
        }
    });
    metadata_rows(ui, item);
    action_buttons(ui, item, event);
}

fn metadata_rows(ui: &mut egui::Ui, item: &StreamItem) {
    if !item.assignees.is_empty() {
        ui.label(format!("Assignees: {}", item.assignees.join(", ")));
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
