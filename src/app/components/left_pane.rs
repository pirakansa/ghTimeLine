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
        let count = library_counts.unread_count(library);
        let selected = state.selection == Selection::Library(library);
        if full_width_selectable_row(ui, selected, library.label(), count).clicked() {
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
        let selected = state.selection == Selection::SavedQuery(query.id);
        if full_width_selectable_row(ui, selected, &query.name, query.unread_count).clicked() {
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
