use eframe::egui;

use super::selectable_row;
use crate::app::screens::{saved_query_manager, stream};
use crate::models::{LibraryCounts, LibraryView, SavedQuery, Selection};
use stream::{StreamEvent, StreamState};

pub fn show(
    ctx: &egui::Context,
    state: &mut StreamState,
    library_counts: &LibraryCounts,
    saved_queries: &[SavedQuery],
    event: &mut Option<StreamEvent>,
) {
    egui::SidePanel::left("stream-left")
        .resizable(true)
        .default_width(260.0)
        .show(ctx, |ui| {
            library_section(ui, state, library_counts, event);
            saved_query_section(ui, state, saved_queries, event);
        });
}

fn library_section(
    ui: &mut egui::Ui,
    state: &StreamState,
    library_counts: &LibraryCounts,
    event: &mut Option<StreamEvent>,
) {
    ui.heading("Library");
    for library in LibraryView::ALL {
        let count = library_counts.unread_count(library);
        let selected = state.selection == Selection::Library(library);
        if selectable_row::show(ui, selected, library.label(), Some(count)).clicked() {
            *event = Some(StreamEvent::Select(Selection::Library(library)));
        }
    }
}

fn saved_query_section(
    ui: &mut egui::Ui,
    state: &mut StreamState,
    saved_queries: &[SavedQuery],
    event: &mut Option<StreamEvent>,
) {
    ui.separator();
    ui.horizontal(|ui| {
        ui.heading("Saved queries");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Manage").clicked() {
                saved_query_manager::open(state, saved_queries);
            }
        });
    });
    for query in saved_queries {
        let selected = state.selection == Selection::SavedQuery(query.id);
        let name = if query.enabled {
            query.name.clone()
        } else {
            format!("{} (disabled)", query.name)
        };
        if selectable_row::show(ui, selected, &name, Some(query.unread_count)).clicked() {
            *event = Some(StreamEvent::Select(Selection::SavedQuery(query.id)));
        }
    }
}
