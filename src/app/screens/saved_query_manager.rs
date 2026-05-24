use eframe::egui;

use crate::app::components::selectable_row;
use crate::app::screens::stream::{StreamEvent, StreamState};
use crate::models::{SavedQuery, Selection, SortOrder};

pub struct SavedQueryManagerState {
    pub(in crate::app) open: bool,
    edit_query_id: Option<i64>,
    edit_query_name: String,
    edit_query_text: String,
    edit_query_sort: SortOrder,
    edit_query_enabled: bool,
}

impl Default for SavedQueryManagerState {
    fn default() -> Self {
        Self {
            open: false,
            edit_query_id: None,
            edit_query_name: String::new(),
            edit_query_text: String::new(),
            edit_query_sort: SortOrder::UpdatedDesc,
            edit_query_enabled: true,
        }
    }
}

pub fn show(
    ctx: &egui::Context,
    state: &mut StreamState,
    saved_queries: &[SavedQuery],
    event: &mut Option<StreamEvent>,
) {
    egui::TopBottomPanel::top("saved-query-manager-toolbar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading("Saved queries");
            ui.separator();
            if ui.button("Back").clicked() {
                state.saved_query_manager.open = false;
            }
            if ui.button("New").clicked() {
                clear_query_draft(&mut state.saved_query_manager);
            }
        });
    });

    egui::SidePanel::left("saved-query-manager-list")
        .resizable(true)
        .default_width(280.0)
        .width_range(180.0..=480.0)
        .show(ctx, |ui| {
            saved_query_list(ui, &mut state.saved_query_manager, saved_queries);
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        saved_query_form(ui, &mut state.saved_query_manager, event);
    });
}

pub fn open(state: &mut StreamState, saved_queries: &[SavedQuery]) {
    state.saved_query_manager.open = true;
    if let Selection::SavedQuery(id) = state.selection {
        if let Some(query) = saved_queries.iter().find(|query| query.id == id) {
            load_query_draft(&mut state.saved_query_manager, query);
            return;
        }
    }
    if state.saved_query_manager.edit_query_id.is_none()
        && state.saved_query_manager.edit_query_name.is_empty()
    {
        if let Some(query) = saved_queries.first() {
            load_query_draft(&mut state.saved_query_manager, query);
        }
    }
}

fn saved_query_list(
    ui: &mut egui::Ui,
    state: &mut SavedQueryManagerState,
    saved_queries: &[SavedQuery],
) {
    ui.vertical(|ui| {
        ui.heading("Queries");
        ui.add_space(6.0);
        egui::ScrollArea::vertical()
            .id_salt("saved-query-manager-list")
            .show(ui, |ui| {
                for query in saved_queries {
                    let selected = state.edit_query_id == Some(query.id);
                    let label = if query.enabled {
                        query.name.clone()
                    } else {
                        format!("{} (disabled)", query.name)
                    };
                    if selectable_row::show(ui, selected, &label, None).clicked() {
                        load_query_draft(state, query);
                    }
                }
            });
    });
}

fn saved_query_form(
    ui: &mut egui::Ui,
    state: &mut SavedQueryManagerState,
    event: &mut Option<StreamEvent>,
) {
    ui.vertical(|ui| {
        ui.set_min_width(360.0);
        ui.heading(if state.edit_query_id.is_some() {
            "Edit query"
        } else {
            "New query"
        });
        ui.label("Name");
        ui.text_edit_singleline(&mut state.edit_query_name);
        ui.label("Query");
        ui.text_edit_singleline(&mut state.edit_query_text);
        ui.label("Sort");
        egui::ComboBox::from_id_salt("saved-query-manager-sort")
            .selected_text(state.edit_query_sort.label())
            .show_ui(ui, |ui| {
                for sort in SortOrder::ALL {
                    ui.selectable_value(&mut state.edit_query_sort, sort, sort.label());
                }
            });

        let enabled_changed = ui
            .checkbox(&mut state.edit_query_enabled, "Enabled")
            .changed();
        if enabled_changed {
            if let Some(id) = state.edit_query_id {
                *event = Some(StreamEvent::SetQueryEnabled {
                    id,
                    enabled: state.edit_query_enabled,
                });
            }
        }

        ui.separator();
        ui.horizontal(|ui| match state.edit_query_id {
            Some(id) => {
                if ui.button("Save changes").clicked() {
                    *event = Some(StreamEvent::UpdateQuery {
                        id,
                        name: state.edit_query_name.clone(),
                        query: state.edit_query_text.clone(),
                        sort: state.edit_query_sort,
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
                        name: state.edit_query_name.clone(),
                        query: state.edit_query_text.clone(),
                        enabled: state.edit_query_enabled,
                    });
                    clear_query_draft(state);
                }
            }
        });
    });
}

fn load_query_draft(state: &mut SavedQueryManagerState, query: &SavedQuery) {
    state.edit_query_id = Some(query.id);
    state.edit_query_name = query.name.clone();
    state.edit_query_text = query.query.clone();
    state.edit_query_sort = query.sort;
    state.edit_query_enabled = query.enabled;
}

fn clear_query_draft(state: &mut SavedQueryManagerState) {
    state.edit_query_id = None;
    state.edit_query_name.clear();
    state.edit_query_text.clear();
    state.edit_query_sort = SortOrder::UpdatedDesc;
    state.edit_query_enabled = true;
}
