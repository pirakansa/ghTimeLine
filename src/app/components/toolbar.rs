use eframe::egui;

use crate::app::stream::{StreamEvent, StreamState};
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
        polling_interval_control(ui, state, config, event);
    });
}

fn refresh_button(ui: &mut egui::Ui, event: &mut Option<StreamEvent>) {
    if ui.button("Refresh").clicked() {
        *event = Some(StreamEvent::RefreshNow);
    }
}

fn filter_controls(ui: &mut egui::Ui, state: &StreamState, event: &mut Option<StreamEvent>) {
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

fn polling_interval_control(
    ui: &mut egui::Ui,
    state: &mut StreamState,
    config: &AppConfig,
    event: &mut Option<StreamEvent>,
) {
    ui.separator();
    ui.add(
        egui::DragValue::new(&mut state.polling_interval_draft)
            .range(1..=1440)
            .speed(1),
    );
    ui.label("min");
    if state.polling_interval_draft != config.refresh.polling_interval_minutes
        && ui.button("Save interval").clicked()
    {
        *event = Some(StreamEvent::SetPollingInterval(
            state.polling_interval_draft,
        ));
    }
}
