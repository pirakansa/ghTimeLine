use eframe::egui;

use crate::app::screens::stream::{StreamEvent, StreamState};
use crate::models::{AppConfig, FontSize, Theme};

pub fn show(
    ctx: &egui::Context,
    state: &mut StreamState,
    config: &AppConfig,
    event: &mut Option<StreamEvent>,
) {
    egui::TopBottomPanel::top("menu-bar").show(ctx, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            preferences_menu(ui, state, config, event);
        });
    });
}

fn preferences_menu(
    ui: &mut egui::Ui,
    state: &mut StreamState,
    config: &AppConfig,
    event: &mut Option<StreamEvent>,
) {
    ui.menu_button("Preferences", |ui| {
        if ui.button("Host settings").clicked() {
            *event = Some(StreamEvent::OpenSetup);
            ui.close();
        }
        ui.separator();
        theme_submenu(ui, config, event);
        font_size_submenu(ui, config, event);
        ui.separator();
        polling_interval_control(ui, state, config, event);
    });
}

fn font_size_submenu(ui: &mut egui::Ui, config: &AppConfig, event: &mut Option<StreamEvent>) {
    ui.menu_button("Font size", |ui| {
        for size in [FontSize::Default, FontSize::Large, FontSize::XLarge] {
            if ui
                .selectable_label(config.ui.font_size == size, size.label())
                .clicked()
            {
                *event = Some(StreamEvent::SetFontSize(size));
                ui.close();
            }
        }
    });
}

fn theme_submenu(ui: &mut egui::Ui, config: &AppConfig, event: &mut Option<StreamEvent>) {
    ui.menu_button("Theme", |ui| {
        for theme in [Theme::System, Theme::Light, Theme::Dark] {
            if ui
                .selectable_label(config.ui.theme == theme, theme.label())
                .clicked()
            {
                *event = Some(StreamEvent::SetTheme(theme));
                ui.close();
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
    ui.horizontal(|ui| {
        ui.label("Polling interval:");
        ui.add(
            egui::DragValue::new(&mut state.polling_interval_draft)
                .range(15..=3600)
                .speed(1),
        );
        ui.label("sec");
        if state.polling_interval_draft != config.refresh.polling_interval_seconds
            && ui.button("Save").clicked()
        {
            *event = Some(StreamEvent::SetPollingInterval(
                state.polling_interval_draft,
            ));
        }
    });
}
