use eframe::egui;

use super::saved_query_transfer;
use crate::app::components::selectable_row;
use crate::app::screens::stream::{StreamEvent, StreamState};
use crate::config;
use crate::models::{FilterStream, SavedQuery, Selection, StreamSource};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditKind {
    SavedQuery,
    FilterStream,
}

pub struct SavedQueryManagerState {
    pub(in crate::app) open: bool,
    pub(in crate::app::screens) transfer_open: bool,
    edit_kind: EditKind,
    edit_saved_query_id: Option<i64>,
    edit_filter_stream_id: Option<i64>,
    edit_name: String,
    edit_text: String,
    edit_source: StreamSource,
    edit_enabled: bool,
    pub(in crate::app::screens) transfer_path: String,
}

impl Default for SavedQueryManagerState {
    fn default() -> Self {
        Self {
            open: false,
            transfer_open: false,
            edit_kind: EditKind::SavedQuery,
            edit_saved_query_id: None,
            edit_filter_stream_id: None,
            edit_name: String::new(),
            edit_text: String::new(),
            edit_source: StreamSource::default(),
            edit_enabled: true,
            transfer_path: config::default_saved_queries_path().display().to_string(),
        }
    }
}

pub fn show(
    ui: &mut egui::Ui,
    state: &mut StreamState,
    saved_queries: &[SavedQuery],
    event: &mut Option<StreamEvent>,
) {
    if state.saved_query_manager.transfer_open {
        saved_query_transfer::show(ui, &mut state.saved_query_manager, event);
        return;
    }

    egui::Panel::top("saved-query-manager-toolbar").show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.heading("Saved queries");
            ui.separator();
            if ui.button("Back").clicked() {
                state.saved_query_manager.open = false;
                state.saved_query_manager.transfer_open = false;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Import / export").clicked() {
                    state.saved_query_manager.transfer_open = true;
                }
            });
        });
    });

    egui::Panel::left("saved-query-manager-list")
        .resizable(true)
        .default_size(280.0)
        .size_range(180.0..=480.0)
        .show(ui, |ui| {
            saved_query_list(ui, &mut state.saved_query_manager, saved_queries, event);
        });

    egui::CentralPanel::default().show(ui, |ui| {
        saved_query_form(ui, &mut state.saved_query_manager, saved_queries, event);
    });
}

pub fn open(state: &mut StreamState, saved_queries: &[SavedQuery]) {
    state.saved_query_manager.open = true;
    state.saved_query_manager.transfer_open = false;
    match state.selection {
        Selection::SavedQuery(id) => {
            if let Some(query) = saved_queries.iter().find(|query| query.id == id) {
                load_query_draft(&mut state.saved_query_manager, query);
                return;
            }
        }
        Selection::FilterStream(id) => {
            if let Some((query, filter_stream)) = find_filter_stream(saved_queries, id) {
                load_filter_stream_draft(&mut state.saved_query_manager, query.id, filter_stream);
                return;
            }
        }
        Selection::Library(_) => {}
    }

    if state.saved_query_manager.edit_name.is_empty() {
        if let Some(query) = saved_queries.first() {
            load_query_draft(&mut state.saved_query_manager, query);
        }
    }
}

fn saved_query_list(
    ui: &mut egui::Ui,
    state: &mut SavedQueryManagerState,
    saved_queries: &[SavedQuery],
    event: &mut Option<StreamEvent>,
) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.heading("Queries");
            if ui
                .small_button("+")
                .on_hover_text("New saved query")
                .clicked()
            {
                clear_query_draft(state);
            }
            if ui
                .small_button("F+")
                .on_hover_text("New filter stream")
                .clicked()
            {
                start_new_filter_stream(state, saved_queries);
            }
            let can_move_up = can_move_selected_query(saved_queries, state, true);
            if ui
                .add_enabled(can_move_up, egui::Button::new("▲"))
                .on_hover_text("Move selected query up")
                .clicked()
            {
                if let Some(id) = state.edit_saved_query_id {
                    *event = Some(StreamEvent::MoveQueryUp(id));
                }
            }
            let can_move_down = can_move_selected_query(saved_queries, state, false);
            if ui
                .add_enabled(can_move_down, egui::Button::new("▼"))
                .on_hover_text("Move selected query down")
                .clicked()
            {
                if let Some(id) = state.edit_saved_query_id {
                    *event = Some(StreamEvent::MoveQueryDown(id));
                }
            }
        });
        ui.add_space(6.0);
        egui::ScrollArea::vertical()
            .id_salt("saved-query-manager-list")
            .show(ui, |ui| {
                for query in saved_queries {
                    let selected = state.edit_kind == EditKind::SavedQuery
                        && state.edit_saved_query_id == Some(query.id)
                        && state.edit_filter_stream_id.is_none();
                    let label = if query.enabled {
                        query.name.clone()
                    } else {
                        format!("{} (disabled)", query.name)
                    };
                    if selectable_row::show(ui, selected, &label, None).clicked() {
                        load_query_draft(state, query);
                    }

                    for filter_stream in &query.filter_streams {
                        let selected = state.edit_kind == EditKind::FilterStream
                            && state.edit_filter_stream_id == Some(filter_stream.id);
                        let label = if filter_stream.enabled {
                            format!("↳ {}", filter_stream.name)
                        } else {
                            format!("↳ {} (disabled)", filter_stream.name)
                        };
                        if selectable_row::show(ui, selected, &label, None).clicked() {
                            load_filter_stream_draft(state, query.id, filter_stream);
                        }
                    }
                }
            });
    });
}

fn can_move_selected_query(
    saved_queries: &[SavedQuery],
    state: &SavedQueryManagerState,
    move_up: bool,
) -> bool {
    if state.edit_kind != EditKind::SavedQuery || state.edit_filter_stream_id.is_some() {
        return false;
    }
    let Some(selected_query_id) = state.edit_saved_query_id else {
        return false;
    };
    let Some(selected_query) = saved_queries
        .iter()
        .find(|query| query.id == selected_query_id)
    else {
        return false;
    };

    let group_index = saved_queries
        .iter()
        .filter(|query| query.enabled == selected_query.enabled)
        .position(|query| query.id == selected_query_id);
    let group_len = saved_queries
        .iter()
        .filter(|query| query.enabled == selected_query.enabled)
        .count();

    match group_index {
        Some(0) if move_up => false,
        Some(index) if !move_up => index + 1 < group_len,
        Some(_) => true,
        None => false,
    }
}

fn saved_query_form(
    ui: &mut egui::Ui,
    state: &mut SavedQueryManagerState,
    saved_queries: &[SavedQuery],
    event: &mut Option<StreamEvent>,
) {
    ui.vertical(|ui| {
        ui.set_min_width(360.0);
        match state.edit_kind {
            EditKind::SavedQuery => render_saved_query_form(ui, state, event),
            EditKind::FilterStream => render_filter_stream_form(ui, state, saved_queries, event),
        }
    });
}

fn render_saved_query_form(
    ui: &mut egui::Ui,
    state: &mut SavedQueryManagerState,
    event: &mut Option<StreamEvent>,
) {
    ui.heading(if state.edit_saved_query_id.is_some() {
        "Edit query"
    } else {
        "New query"
    });
    ui.label("Name");
    ui.text_edit_singleline(&mut state.edit_name);
    ui.label("Query");
    egui::ComboBox::from_label("Source")
        .selected_text(state.edit_source.label())
        .show_ui(ui, |ui| {
            for source in StreamSource::ALL {
                ui.selectable_value(&mut state.edit_source, source, source.label());
            }
        });
    ui.horizontal(|ui| {
        ui.text_edit_singleline(&mut state.edit_text);
        let can_preview = !state.edit_text.trim().is_empty();
        if ui
            .add_enabled(can_preview, egui::Button::new("Preview"))
            .on_hover_text("Open this query or project in your browser")
            .clicked()
        {
            *event = Some(StreamEvent::PreviewQuery {
                query: state.edit_text.clone(),
                source: state.edit_source,
            });
        }
    });
    ui.checkbox(&mut state.edit_enabled, "Enabled");

    ui.separator();
    ui.horizontal(|ui| match state.edit_saved_query_id {
        Some(id) => {
            if ui.button("Save changes").clicked() {
                *event = Some(StreamEvent::UpdateQuery {
                    id,
                    name: state.edit_name.clone(),
                    query: state.edit_text.clone(),
                    source: state.edit_source,
                    enabled: state.edit_enabled,
                });
            }
            if ui.button("Delete").clicked() {
                *event = Some(StreamEvent::DeleteQuery(id));
                clear_query_draft(state);
            }
        }
        None => {
            if ui.button("Add").clicked() {
                *event = Some(StreamEvent::AddQuery {
                    name: state.edit_name.clone(),
                    query: state.edit_text.clone(),
                    source: state.edit_source,
                    enabled: state.edit_enabled,
                });
                clear_query_draft(state);
            }
        }
    });
}

fn render_filter_stream_form(
    ui: &mut egui::Ui,
    state: &mut SavedQueryManagerState,
    saved_queries: &[SavedQuery],
    event: &mut Option<StreamEvent>,
) {
    ui.heading(if state.edit_filter_stream_id.is_some() {
        "Edit filter stream"
    } else {
        "New filter stream"
    });
    ui.label("Parent query");
    ui.label(parent_query_name(saved_queries, state.edit_saved_query_id).unwrap_or("Unknown"));
    ui.label("Name");
    ui.text_edit_singleline(&mut state.edit_name);
    ui.label("Local filter");
    ui.text_edit_singleline(&mut state.edit_text);
    ui.checkbox(&mut state.edit_enabled, "Enabled");

    ui.separator();
    ui.horizontal(|ui| match state.edit_filter_stream_id {
        Some(id) => {
            if ui.button("Save changes").clicked() {
                *event = Some(StreamEvent::UpdateFilterStream {
                    id,
                    name: state.edit_name.clone(),
                    filter_query: state.edit_text.clone(),
                    enabled: state.edit_enabled,
                });
            }
            if ui.button("Delete").clicked() {
                *event = Some(StreamEvent::DeleteFilterStream(id));
                clear_query_draft(state);
            }
        }
        None => {
            if let Some(saved_query_id) = state.edit_saved_query_id {
                if ui.button("Add").clicked() {
                    *event = Some(StreamEvent::AddFilterStream {
                        saved_query_id,
                        name: state.edit_name.clone(),
                        filter_query: state.edit_text.clone(),
                        enabled: state.edit_enabled,
                    });
                    clear_query_draft(state);
                }
            }
        }
    });
}

fn start_new_filter_stream(state: &mut SavedQueryManagerState, saved_queries: &[SavedQuery]) {
    let parent_saved_query_id = selected_parent_saved_query_id(state, saved_queries)
        .or_else(|| saved_queries.first().map(|query| query.id));
    if let Some(saved_query_id) = parent_saved_query_id {
        state.edit_kind = EditKind::FilterStream;
        state.edit_saved_query_id = Some(saved_query_id);
        state.edit_filter_stream_id = None;
        state.edit_name.clear();
        state.edit_text.clear();
        state.edit_enabled = true;
    }
}

fn selected_parent_saved_query_id(
    state: &SavedQueryManagerState,
    saved_queries: &[SavedQuery],
) -> Option<i64> {
    match state.edit_kind {
        EditKind::SavedQuery => state.edit_saved_query_id,
        EditKind::FilterStream => state.edit_saved_query_id.or_else(|| {
            state.edit_filter_stream_id.and_then(|filter_stream_id| {
                find_filter_stream(saved_queries, filter_stream_id).map(|(query, _)| query.id)
            })
        }),
    }
}

fn parent_query_name(saved_queries: &[SavedQuery], saved_query_id: Option<i64>) -> Option<&str> {
    let saved_query_id = saved_query_id?;
    saved_queries
        .iter()
        .find(|query| query.id == saved_query_id)
        .map(|query| query.name.as_str())
}

fn find_filter_stream(
    saved_queries: &[SavedQuery],
    filter_stream_id: i64,
) -> Option<(&SavedQuery, &FilterStream)> {
    saved_queries.iter().find_map(|query| {
        query
            .filter_streams
            .iter()
            .find(|filter_stream| filter_stream.id == filter_stream_id)
            .map(|filter_stream| (query, filter_stream))
    })
}

fn load_query_draft(state: &mut SavedQueryManagerState, query: &SavedQuery) {
    state.edit_kind = EditKind::SavedQuery;
    state.edit_saved_query_id = Some(query.id);
    state.edit_filter_stream_id = None;
    state.edit_name = query.name.clone();
    state.edit_text = query.query.clone();
    state.edit_source = query.source;
    state.edit_enabled = query.enabled;
}

fn load_filter_stream_draft(
    state: &mut SavedQueryManagerState,
    saved_query_id: i64,
    filter_stream: &FilterStream,
) {
    state.edit_kind = EditKind::FilterStream;
    state.edit_saved_query_id = Some(saved_query_id);
    state.edit_filter_stream_id = Some(filter_stream.id);
    state.edit_name = filter_stream.name.clone();
    state.edit_text = filter_stream.filter_query.clone();
    state.edit_enabled = filter_stream.enabled;
}

fn clear_query_draft(state: &mut SavedQueryManagerState) {
    state.edit_kind = EditKind::SavedQuery;
    state.edit_saved_query_id = None;
    state.edit_filter_stream_id = None;
    state.edit_name.clear();
    state.edit_text.clear();
    state.edit_source = StreamSource::default();
    state.edit_enabled = true;
}
