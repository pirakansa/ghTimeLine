use eframe::egui;

use crate::app::stream::{StreamEvent, StreamState};
use crate::models::{LibraryCounts, LibraryView, SavedQuery, Selection, SortOrder};

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
            new_query_form(ui, state, event);
            edit_selected_query_form(ui, state, saved_queries, event);
            delete_selected_query_button(ui, state, event);
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
        let label = format!(
            "{} ({})",
            library.label(),
            library_counts.unread_count(library)
        );
        if ui
            .selectable_label(state.selection == Selection::Library(library), label)
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

fn edit_selected_query_form(
    ui: &mut egui::Ui,
    state: &mut StreamState,
    saved_queries: &[SavedQuery],
    event: &mut Option<StreamEvent>,
) {
    let Selection::SavedQuery(selected_id) = state.selection else {
        state.edit_query_id = None;
        return;
    };
    let Some(saved_query) = saved_queries.iter().find(|query| query.id == selected_id) else {
        state.edit_query_id = None;
        return;
    };

    if state.edit_query_id != Some(selected_id) {
        state.edit_query_id = Some(selected_id);
        state.edit_query_name = saved_query.name.clone();
        state.edit_query_text = saved_query.query.clone();
        state.edit_query_sort = saved_query.sort;
    }

    ui.separator();
    ui.label("Edit selected query");
    ui.text_edit_singleline(&mut state.edit_query_name);
    ui.text_edit_singleline(&mut state.edit_query_text);
    egui::ComboBox::from_id_salt("edit-query-sort")
        .selected_text(state.edit_query_sort.label())
        .show_ui(ui, |ui| {
            for sort in SortOrder::ALL {
                ui.selectable_value(&mut state.edit_query_sort, sort, sort.label());
            }
        });
    if ui.button("Save changes").clicked() {
        *event = Some(StreamEvent::UpdateQuery {
            id: selected_id,
            name: state.edit_query_name.clone(),
            query: state.edit_query_text.clone(),
            sort: state.edit_query_sort,
        });
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
