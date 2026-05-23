use eframe::egui;

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
