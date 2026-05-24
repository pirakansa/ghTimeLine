use eframe::egui;

use crate::app::screens::stream::{StreamEvent, StreamState};
use crate::models::{SavedQuery, Selection, SortOrder};

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
                state.saved_query_manager_open = false;
            }
            if ui.button("New").clicked() {
                clear_query_draft(state);
            }
        });
    });

    egui::SidePanel::left("saved-query-manager-list")
        .resizable(true)
        .default_width(280.0)
        .width_range(180.0..=480.0)
        .show(ctx, |ui| {
            saved_query_list(ui, state, saved_queries);
        });

    egui::CentralPanel::default().show(ctx, |ui| {
        saved_query_form(ui, state, event);
    });
}

pub fn open(state: &mut StreamState, saved_queries: &[SavedQuery]) {
    state.saved_query_manager_open = true;
    if let Selection::SavedQuery(id) = state.selection {
        if let Some(query) = saved_queries.iter().find(|query| query.id == id) {
            load_query_draft(state, query);
            return;
        }
    }
    if state.edit_query_id.is_none() && state.edit_query_name.is_empty() {
        if let Some(query) = saved_queries.first() {
            load_query_draft(state, query);
        }
    }
}

fn saved_query_list(ui: &mut egui::Ui, state: &mut StreamState, saved_queries: &[SavedQuery]) {
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
                    if full_width_selectable_row(ui, selected, &label).clicked() {
                        load_query_draft(state, query);
                    }
                }
            });
    });
}

fn full_width_selectable_row(ui: &mut egui::Ui, selected: bool, label: &str) -> egui::Response {
    let row_height = ui.spacing().interact_size.y;
    let available_width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(available_width, row_height),
        egui::Sense::click(),
    );

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact_selectable(&response, selected);
        if selected || response.hovered() {
            ui.painter()
                .rect_filled(rect, visuals.corner_radius, visuals.bg_fill);
        }

        ui.painter().text(
            egui::pos2(rect.left() + ui.spacing().button_padding.x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::TextStyle::Body.resolve(ui.style()),
            visuals.text_color(),
        );
    }

    let label = label.to_owned();
    response.widget_info(move || {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, label.clone())
    });

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn saved_query_form(ui: &mut egui::Ui, state: &mut StreamState, event: &mut Option<StreamEvent>) {
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

fn load_query_draft(state: &mut StreamState, query: &SavedQuery) {
    state.edit_query_id = Some(query.id);
    state.edit_query_name = query.name.clone();
    state.edit_query_text = query.query.clone();
    state.edit_query_sort = query.sort;
    state.edit_query_enabled = query.enabled;
}

fn clear_query_draft(state: &mut StreamState) {
    state.edit_query_id = None;
    state.edit_query_name.clear();
    state.edit_query_text.clear();
    state.edit_query_sort = SortOrder::UpdatedDesc;
    state.edit_query_enabled = true;
}
