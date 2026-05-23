use eframe::egui;

use crate::app::stream::{StreamEvent, StreamState};
use crate::models::{LibraryView, SavedQuery, Selection};

pub fn show(
    ctx: &egui::Context,
    state: &mut StreamState,
    saved_queries: &[SavedQuery],
    event: &mut Option<StreamEvent>,
) {
    egui::SidePanel::left("stream-left")
        .resizable(true)
        .default_width(260.0)
        .show(ctx, |ui| {
            library_section(ui, state, event);
            saved_query_section(ui, state, saved_queries, event);
            new_query_form(ui, state, event);
            delete_selected_query_button(ui, state, event);
        });
}

fn library_section(ui: &mut egui::Ui, state: &StreamState, event: &mut Option<StreamEvent>) {
    ui.heading("Library");
    for library in LibraryView::ALL {
        if ui
            .selectable_label(
                state.selection == Selection::Library(library),
                library.label(),
            )
            .clicked()
        {
            *event = Some(StreamEvent::Select(Selection::Library(library)));
        }
    }
}

fn saved_query_section(
    ui: &mut egui::Ui,
    state: &StreamState,
    saved_queries: &[SavedQuery],
    event: &mut Option<StreamEvent>,
) {
    ui.separator();
    ui.heading("Saved queries");
    for query in saved_queries {
        let label = format!("{} ({})", query.name, query.unread_count);
        if ui
            .selectable_label(state.selection == Selection::SavedQuery(query.id), label)
            .clicked()
        {
            *event = Some(StreamEvent::Select(Selection::SavedQuery(query.id)));
        }
    }
}

fn new_query_form(ui: &mut egui::Ui, state: &mut StreamState, event: &mut Option<StreamEvent>) {
    ui.separator();
    ui.label("New query");
    ui.text_edit_singleline(&mut state.new_query_name);
    ui.text_edit_singleline(&mut state.new_query_text);
    if ui.button("Add").clicked() {
        *event = Some(StreamEvent::AddQuery {
            name: state.new_query_name.clone(),
            query: state.new_query_text.clone(),
        });
        state.new_query_name.clear();
        state.new_query_text.clear();
    }
}

fn delete_selected_query_button(
    ui: &mut egui::Ui,
    state: &StreamState,
    event: &mut Option<StreamEvent>,
) {
    if matches!(state.selection, Selection::SavedQuery(_)) && ui.button("Delete selected").clicked()
    {
        *event = Some(StreamEvent::DeleteSelectedQuery);
    }
}
