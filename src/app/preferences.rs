use eframe::egui;

use crate::app::{AppMode, GhStreamApp};
use crate::config;
use crate::models::{AppConfig, FontSize, SortOrder, Theme};

impl GhStreamApp {
    pub(super) fn update_default_sort(&mut self, sort: SortOrder) {
        if let AppMode::Main(runtime) = &mut self.mode {
            runtime.config.ui.default_sort = sort;
            match config::write_config(&self.config_path, &runtime.config) {
                Ok(()) => Self::replace_status(
                    &mut self.status,
                    &mut self.status_history,
                    "Sort preference saved.",
                ),
                Err(err) => Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    format!("Could not save sort preference: {err}"),
                ),
            }
        }
        self.reload_current_view();
    }

    pub(super) fn update_theme(&mut self, ctx: &egui::Context, theme: Theme) {
        if let AppMode::Main(runtime) = &mut self.mode {
            runtime.config.ui.theme = theme;
            let config_snapshot = runtime.config.clone();
            let write_result = config::write_config(&self.config_path, &config_snapshot);
            apply_theme_from_config(ctx, &config_snapshot);
            match write_result {
                Ok(()) => {
                    Self::replace_status(&mut self.status, &mut self.status_history, "Theme saved.")
                }
                Err(err) => Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    format!("Could not save theme: {err}"),
                ),
            }
        }
    }

    pub(super) fn update_font_size(&mut self, ctx: &egui::Context, font_size: FontSize) {
        if let AppMode::Main(runtime) = &mut self.mode {
            runtime.config.ui.font_size = font_size;
            apply_font_size_from_config(ctx, &runtime.config);
            match config::write_config(&self.config_path, &runtime.config) {
                Ok(()) => Self::replace_status(
                    &mut self.status,
                    &mut self.status_history,
                    "Font size saved.",
                ),
                Err(err) => Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    format!("Could not save font size: {err}"),
                ),
            }
        }
    }

    pub(super) fn update_polling_interval(&mut self, seconds: u32) {
        if let AppMode::Main(runtime) = &mut self.mode {
            runtime.config.refresh.polling_interval_seconds = seconds;
            match config::write_config(&self.config_path, &runtime.config) {
                Ok(()) => Self::replace_status(
                    &mut self.status,
                    &mut self.status_history,
                    "Polling interval saved.",
                ),
                Err(err) => Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    format!("Could not save polling interval: {err}"),
                ),
            }
        }
    }
}

pub(super) fn apply_theme_from_config(ctx: &egui::Context, config: &AppConfig) {
    let visuals = match config.ui.theme {
        Theme::Light => egui::Visuals::light(),
        Theme::Dark => egui::Visuals::dark(),
        Theme::System => {
            if ctx.system_theme() == Some(egui::Theme::Dark) {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            }
        }
    };
    ctx.set_visuals(visuals);
}

pub(super) fn apply_font_size_from_config(ctx: &egui::Context, config: &AppConfig) {
    let scale = config.ui.font_size.scale();
    ctx.all_styles_mut(|style| {
        for (text_style, font_id) in &mut style.text_styles {
            let base = match text_style {
                egui::TextStyle::Small => 10.0,
                egui::TextStyle::Body => 14.0,
                egui::TextStyle::Button => 14.0,
                egui::TextStyle::Heading => 20.0,
                egui::TextStyle::Monospace => 14.0,
                egui::TextStyle::Name(_) => font_id.size,
            };
            font_id.size = base * scale;
        }
    });
}
