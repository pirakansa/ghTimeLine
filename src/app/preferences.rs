use eframe::egui;

use crate::app::{AppMode, GhStreamApp};
use crate::config;
use crate::models::{AppConfig, FontSize, SortOrder, Theme};

impl GhStreamApp {
    pub(super) fn update_default_sort(&mut self, sort: SortOrder) {
        if self
            .persist_config_update(
                |config| config.ui.default_sort = sort,
                "Sort preference saved.",
                "Could not save sort preference",
            )
            .is_some()
        {
            self.reload_current_view();
        }
    }

    pub(super) fn update_theme(&mut self, ctx: &egui::Context, theme: Theme) {
        if let Some(config) = self.persist_config_update(
            |config| config.ui.theme = theme,
            "Theme saved.",
            "Could not save theme",
        ) {
            apply_theme_from_config(ctx, &config);
        }
    }

    pub(super) fn update_font_size(&mut self, ctx: &egui::Context, font_size: FontSize) {
        if let Some(config) = self.persist_config_update(
            |config| config.ui.font_size = font_size,
            "Font size saved.",
            "Could not save font size",
        ) {
            apply_font_size_from_config(ctx, &config);
        }
    }

    pub(super) fn update_polling_interval(&mut self, seconds: u32) {
        self.persist_config_update(
            |config| config.refresh.polling_interval_seconds = seconds,
            "Polling interval saved.",
            "Could not save polling interval",
        );
    }

    fn persist_config_update(
        &mut self,
        update: impl FnOnce(&mut AppConfig),
        success_message: &str,
        error_prefix: &str,
    ) -> Option<AppConfig> {
        let AppMode::Main(runtime) = &self.mode else {
            return None;
        };
        let mut candidate = runtime.config.clone();
        update(&mut candidate);

        match config::write_config(&self.config_path, &candidate) {
            Ok(()) => {
                let AppMode::Main(runtime) = &mut self.mode else {
                    unreachable!("app mode cannot change while saving config");
                };
                runtime.config = candidate.clone();
                Self::replace_status(&mut self.status, &mut self.status_history, success_message);
                Some(candidate)
            }
            Err(err) => {
                Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    format!("{error_prefix}: {err}"),
                );
                None
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
