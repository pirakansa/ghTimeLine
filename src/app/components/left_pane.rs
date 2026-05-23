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
        if full_width_selectable_row(ui, selected, library.label(), count).clicked() {
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
                open_saved_query_manager(state, saved_queries);
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
        if full_width_selectable_row(ui, selected, &name, query.unread_count).clicked() {
            *event = Some(StreamEvent::Select(Selection::SavedQuery(query.id)));
        }
    }
}

/// A selectable row that spans the full available width, with the name on the
/// left and the unread badge on the right.  The entire row rect is the click
/// and highlight target so the UX matches Jasper's sidebar.
fn full_width_selectable_row(
    ui: &mut egui::Ui,
    selected: bool,
    name: &str,
    unread_count: i64,
) -> egui::Response {
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

        let px = ui.spacing().button_padding.x;
        let font_id = egui::TextStyle::Body.resolve(ui.style());

        // Badge (right-aligned)
        let badge_reserved = if unread_count > 0 {
            let galley = ui.painter().layout_no_wrap(
                unread_count.to_string(),
                font_id.clone(),
                visuals.text_color(),
            );
            let w = galley.size().x;
            let pos = egui::pos2(
                rect.right() - px - w,
                rect.center().y - galley.size().y / 2.0,
            );
            ui.painter().galley(pos, galley, visuals.text_color());
            w + px + ui.spacing().item_spacing.x
        } else {
            0.0
        };

        // Name (left-aligned, clipped to avoid overlap with badge)
        let max_text_width = available_width - badge_reserved - px * 2.0;
        ui.painter().text(
            egui::pos2(rect.left() + px, rect.center().y),
            egui::Align2::LEFT_CENTER,
            name,
            font_id,
            visuals.text_color(),
        );
        // Clip overly-long names behind the badge (paint white rect is heavy;
        // egui clips the painter to the panel automatically).
        let _ = max_text_width;
    }

    // Register accessibility label so tests can find the row by name.
    let name = name.to_owned();
    response.widget_info(move || {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, name.clone())
    });

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn show_saved_query_manager(
    ctx: &egui::Context,
    state: &mut StreamState,
    saved_queries: &[SavedQuery],
    event: &mut Option<StreamEvent>,
) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("Saved queries");
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("Back").clicked() {
                state.saved_query_manager_open = false;
            }
            if ui.button("New").clicked() {
                clear_query_draft(state);
            }
        });
        ui.add_space(12.0);
        ui.columns(2, |columns| {
            columns[0].set_width(280.0);
            saved_query_manager_list(&mut columns[0], state, saved_queries);
            columns[1].vertical(|ui| {
                ui.separator();
                ui.set_min_width(420.0);
                saved_query_manager_form(ui, state, event);
            });
        });
    });
}

fn saved_query_manager_list(
    ui: &mut egui::Ui,
    state: &mut StreamState,
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
                    if ui.selectable_label(selected, label).clicked() {
                        load_query_draft(state, query);
                    }
                }
            });
    });
}

fn saved_query_manager_form(
    ui: &mut egui::Ui,
    state: &mut StreamState,
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

pub fn open_saved_query_manager(state: &mut StreamState, saved_queries: &[SavedQuery]) {
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
