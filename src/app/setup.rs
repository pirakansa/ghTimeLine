use eframe::egui;

use crate::config;
use crate::models::{AppConfig, HostKind, Scheme};

pub struct SetupState {
    name: String,
    scheme: Scheme,
    hostname: String,
    rest_api_base_path: String,
    kind: HostKind,
    pat: String,
    validation_message: String,
}

impl Default for SetupState {
    fn default() -> Self {
        let config = AppConfig::default_with_pat(String::new());
        Self {
            name: config.host.name,
            scheme: config.host.scheme,
            hostname: config.host.hostname,
            rest_api_base_path: config.host.rest_api_base_path,
            kind: config.host.kind,
            pat: String::new(),
            validation_message: String::new(),
        }
    }
}

pub fn show(ctx: &egui::Context, state: &mut SetupState, status: &str) -> Option<AppConfig> {
    let mut saved = None;

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("First-run setup");
        ui.label("Configure one GitHub or GHES host. The PAT is stored as plain text in config.yml for v1.");
        ui.add_space(12.0);

        egui::Grid::new("setup-grid")
            .num_columns(2)
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                ui.label("Host name");
                ui.text_edit_singleline(&mut state.name);
                ui.end_row();

                ui.label("Scheme");
                egui::ComboBox::from_id_salt("setup-scheme")
                    .selected_text(state.scheme.to_string())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut state.scheme, Scheme::Https, "https");
                        ui.selectable_value(&mut state.scheme, Scheme::Http, "http");
                    });
                ui.end_row();

                ui.label("Hostname");
                ui.text_edit_singleline(&mut state.hostname);
                ui.end_row();

                ui.label("REST API base path");
                ui.text_edit_singleline(&mut state.rest_api_base_path);
                ui.end_row();

                ui.label("Host kind");
                egui::ComboBox::from_id_salt("setup-kind")
                    .selected_text(state.kind.to_string())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut state.kind, HostKind::GitHub, "github");
                        ui.selectable_value(&mut state.kind, HostKind::Ghes, "ghes");
                    });
                ui.end_row();

                ui.label("Personal access token");
                ui.add(egui::TextEdit::singleline(&mut state.pat).password(true));
                ui.end_row();
            });

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.button("Test").clicked() {
                match config::validate_config(build_config(state)) {
                    Ok(config) => {
                        state.validation_message = format!(
                            "Configuration is valid. REST: {} GraphQL: {}",
                            config.host.rest_api_base_url(),
                            config.host.graphql_url()
                        );
                    }
                    Err(err) => state.validation_message = err.to_string(),
                }
            }

            if ui.button("Save").clicked() {
                match config::validate_config(build_config(state)) {
                    Ok(config) => saved = Some(config),
                    Err(err) => state.validation_message = err.to_string(),
                }
            }
        });

        if !state.validation_message.is_empty() {
            ui.add_space(8.0);
            ui.label(&state.validation_message);
        }

        ui.add_space(8.0);
        ui.label(status);
    });

    saved
}

fn build_config(state: &SetupState) -> AppConfig {
    let mut config = AppConfig::default_with_pat(state.pat.clone());
    config.host.name = state.name.clone();
    config.host.scheme = state.scheme.clone();
    config.host.hostname = state.hostname.clone();
    config.host.rest_api_base_path = state.rest_api_base_path.clone();
    config.host.kind = state.kind.clone();
    config
}
