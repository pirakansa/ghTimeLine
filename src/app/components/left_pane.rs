use eframe::egui;

use super::selectable_row;
use crate::app::screens::{saved_query_manager, stream};
use crate::models::{FilterStream, LibraryCounts, LibraryView, SavedQuery, Selection};
use stream::{StreamEvent, StreamState};

pub fn show(
    ui: &mut egui::Ui,
    state: &mut StreamState,
    library_counts: &LibraryCounts,
    saved_queries: &[SavedQuery],
    event: &mut Option<StreamEvent>,
) {
    egui::Panel::left("stream-left")
        .resizable(true)
        .default_size(260.0)
        .show(ui, |ui| {
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
        let response = selectable_row::show(ui, selected, library.label(), Some(count));
        if count > 0 {
            response.context_menu(|ui| {
                if ui.button("Mark all as read").clicked() {
                    *event = Some(StreamEvent::MarkLibraryRead(library));
                    ui.close();
                }
            });
        }
        if response.clicked() {
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
    ui.add_space(6.0);
    egui::ScrollArea::vertical()
        .id_salt("saved-query-list")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for query in saved_queries.iter().filter(|query| query.enabled) {
                let selected = state.selection == Selection::SavedQuery(query.id);
                let response =
                    selectable_row::show(ui, selected, &query.name, Some(query.unread_count));
                if query.unread_count > 0 {
                    response.context_menu(|ui| {
                        if ui.button("Mark all as read").clicked() {
                            *event = Some(StreamEvent::MarkSavedQueryRead(query.id));
                            ui.close();
                        }
                    });
                }
                if response.clicked() {
                    *event = Some(StreamEvent::Select(Selection::SavedQuery(query.id)));
                }

                for filter_stream in query.filter_streams.iter().filter(|stream| stream.enabled) {
                    show_filter_stream(ui, state, filter_stream, event);
                }
            }
        });
}

fn show_filter_stream(
    ui: &mut egui::Ui,
    state: &StreamState,
    filter_stream: &FilterStream,
    event: &mut Option<StreamEvent>,
) {
    let selected = state.selection == Selection::FilterStream(filter_stream.id);
    let label = format!("↳ {}", filter_stream.name);
    let response = selectable_row::show(ui, selected, &label, Some(filter_stream.unread_count));
    if filter_stream.unread_count > 0 {
        response.context_menu(|ui| {
            if ui.button("Mark all as read").clicked() {
                *event = Some(StreamEvent::MarkFilterStreamRead(filter_stream.id));
                ui.close();
            }
        });
    }
    if response.clicked() {
        *event = Some(StreamEvent::Select(Selection::FilterStream(
            filter_stream.id,
        )));
    }
}
