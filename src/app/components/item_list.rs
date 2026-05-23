use eframe::egui;

use crate::app::stream::{ItemAction, StreamEvent};
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
            draw_item(ui, item, event);
            ui.separator();
        }
    });
}

fn draw_item(ui: &mut egui::Ui, item: &StreamItem, event: &mut Option<StreamEvent>) {
    ui.horizontal(|ui| {
        let unread_marker = if item.is_unread { "Unread" } else { "Read" };
        ui.label(unread_marker);
        ui.label(item.item_type.label());
        ui.label(format!("#{}", item.number));
        ui.label(item.state.as_str());
        ui.label(item.updated_at_github.as_str());
    });
    ui.heading(&item.title);
    ui.horizontal_wrapped(|ui| {
        ui.label(item.repository_full_name());
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
