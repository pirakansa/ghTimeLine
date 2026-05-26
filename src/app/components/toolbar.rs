use eframe::egui;

use crate::app::screens::stream::{StreamEvent, StreamState};
use crate::models::{AppConfig, SortOrder, StreamFilter};

pub fn show(
    ui: &mut egui::Ui,
    state: &mut StreamState,
    config: &AppConfig,
    event: &mut Option<StreamEvent>,
) {
    ui.horizontal_wrapped(|ui| {
        refresh_button(ui, event);
        filter_controls(ui, state, event);
        sort_selector(ui, config, event);
    });
}

fn refresh_button(ui: &mut egui::Ui, event: &mut Option<StreamEvent>) {
    if ui.button("Refresh").clicked() {
        *event = Some(StreamEvent::RefreshNow);
    }
}

fn filter_controls(ui: &mut egui::Ui, state: &mut StreamState, event: &mut Option<StreamEvent>) {
    ui.separator();
    ui.label("Filter");
    if ui.selectable_label(state.filter.is_none(), "All").clicked() {
        *event = Some(StreamEvent::SetFilter(None));
    }
    for filter in StreamFilter::ALL {
        if ui
            .selectable_label(state.filter == Some(filter), filter.label())
            .clicked()
        {
            *event = Some(StreamEvent::SetFilter(Some(filter)));
        }
    }

    ui.separator();
    ui.label("Local filter");
    let response = ui.add(
        egui::TextEdit::singleline(&mut state.local_filter_input)
            .hint_text("author:octo label:bug"),
    );
    let apply_requested =
        response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
    if apply_requested || ui.button("Apply").clicked() {
        let value = state.local_filter_input.trim();
        *event = Some(StreamEvent::SetLocalFilter(
            (!value.is_empty()).then(|| value.to_owned()),
        ));
    }
    if ui.button("Clear").clicked() {
        state.local_filter_input.clear();
        *event = Some(StreamEvent::SetLocalFilter(None));
    }
}

fn sort_selector(ui: &mut egui::Ui, config: &AppConfig, event: &mut Option<StreamEvent>) {
    ui.separator();
    egui::ComboBox::from_id_salt("default-sort")
        .selected_text(config.ui.default_sort.label())
        .show_ui(ui, |ui| {
            for sort in SortOrder::ALL {
                if ui
                    .selectable_label(config.ui.default_sort == sort, sort.label())
                    .clicked()
                {
                    *event = Some(StreamEvent::SetDefaultSort(sort));
                }
            }
        });
}
