pub mod components;
pub mod fonts;
mod item_actions;
mod preferences;
mod refresh;
mod saved_queries;
pub mod setup;
pub mod stream;
mod view;

use std::path::PathBuf;
use std::time::Instant;

use eframe::egui;

use crate::config;
use crate::models::{AppConfig, LibraryCounts, SavedQuery, StreamItem};
use crate::storage::Storage;

pub struct GhStreamApp {
    config_path: PathBuf,
    database_path: PathBuf,
    mode: AppMode,
    setup: setup::SetupState,
    stream: stream::StreamState,
    status: String,
    last_poll_at: Option<Instant>,
    refresh_rx: Option<std::sync::mpsc::Receiver<refresh::RefreshOutcome>>,
}

pub(super) struct Runtime {
    config: AppConfig,
    storage: Storage,
    host_id: i64,
    library_counts: LibraryCounts,
    saved_queries: Vec<SavedQuery>,
    items: Vec<StreamItem>,
}

pub(super) enum AppMode {
    Setup,
    Main(Box<Runtime>),
}

impl GhStreamApp {
    pub fn new() -> Self {
        let config_path = config::default_config_path();
        let database_path = config::default_database_path();
        let setup = setup::SetupState::default();
        let stream = stream::StreamState::default();

        match config::load_config(&config_path) {
            Ok(config) => match Self::open_runtime(config, &database_path) {
                Ok(runtime) => {
                    let mut app = Self {
                        config_path,
                        database_path,
                        mode: AppMode::Main(Box::new(runtime)),
                        setup,
                        stream,
                        status: "Ready".to_owned(),
                        last_poll_at: None,
                        refresh_rx: None,
                    };
                    app.reload_current_view();
                    app
                }
                Err(err) => Self {
                    config_path,
                    database_path,
                    mode: AppMode::Setup,
                    setup,
                    stream,
                    status: format!("Database initialization failed: {err}"),
                    last_poll_at: None,
                    refresh_rx: None,
                },
            },
            Err(err) => Self {
                config_path,
                database_path,
                mode: AppMode::Setup,
                setup,
                stream,
                status: first_run_status(&err),
                last_poll_at: None,
                refresh_rx: None,
            },
        }
    }

    fn open_runtime(
        config: AppConfig,
        database_path: &std::path::Path,
    ) -> crate::storage::Result<Runtime> {
        let storage = Storage::open(database_path)?;
        let host_id = storage.ensure_host(&config.host)?;
        let (library_counts, saved_queries) = view::load_sidebar_data(&storage, host_id)?;
        Ok(Runtime {
            config,
            storage,
            host_id,
            library_counts,
            saved_queries,
            items: Vec::new(),
        })
    }

    fn save_setup_config(&mut self, config: AppConfig) {
        match config::write_config(&self.config_path, &config) {
            Ok(()) => match Self::open_runtime(config, &self.database_path) {
                Ok(runtime) => {
                    self.mode = AppMode::Main(Box::new(runtime));
                    self.status =
                        "Configuration saved. PAT is stored as plain text in v1.".to_owned();
                    self.reload_current_view();
                }
                Err(err) => {
                    self.status = format!("Configuration saved, but database failed: {err}");
                }
            },
            Err(err) => {
                self.status = err.to_string();
            }
        }
    }
}

impl Default for GhStreamApp {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for GhStreamApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_refresh_result();
        self.maybe_poll(ctx);

        let mode = std::mem::replace(&mut self.mode, AppMode::Setup);
        match mode {
            AppMode::Setup => {
                let event = setup::show(ctx, &mut self.setup, &self.status);
                self.mode = AppMode::Setup;
                if let Some(config) = event {
                    self.save_setup_config(config);
                }
            }
            AppMode::Main(runtime) => {
                preferences::apply_theme_from_config(ctx, &runtime.config);
                preferences::apply_font_size_from_config(ctx, &runtime.config);
                let event = stream::show(
                    ctx,
                    &mut self.stream,
                    &runtime.config,
                    &runtime.library_counts,
                    &runtime.saved_queries,
                    &runtime.items,
                    &self.status,
                );
                self.mode = AppMode::Main(runtime);
                match event {
                    Some(stream::StreamEvent::Select(selection)) => self.select(selection),
                    Some(stream::StreamEvent::SetFilter(filter)) => self.set_filter(filter),
                    Some(stream::StreamEvent::AddQuery {
                        name,
                        query,
                        enabled,
                    }) => self.add_query(&name, &query, enabled),
                    Some(stream::StreamEvent::SetQueryEnabled { id, enabled }) => {
                        self.set_query_enabled(id, enabled)
                    }
                    Some(stream::StreamEvent::UpdateQuery {
                        id,
                        name,
                        query,
                        sort,
                    }) => self.update_query(id, &name, &query, sort),
                    Some(stream::StreamEvent::DeleteQuery(id)) => self.delete_query(id),
                    Some(stream::StreamEvent::RefreshNow) => self.refresh_now(ctx.clone()),
                    Some(stream::StreamEvent::SetDefaultSort(sort)) => {
                        self.update_default_sort(sort)
                    }
                    Some(stream::StreamEvent::SetPollingInterval(seconds)) => {
                        self.update_polling_interval(seconds);
                        self.stream.polling_interval_draft = 0; // reset so it re-syncs from config
                    }
                    Some(stream::StreamEvent::SetTheme(theme)) => self.update_theme(ctx, theme),
                    Some(stream::StreamEvent::SetFontSize(size)) => {
                        self.update_font_size(ctx, size)
                    }
                    Some(stream::StreamEvent::ItemAction(action)) => self.item_action(action),
                    None => {}
                }
            }
        }
    }
}

fn first_run_status(error: &config::ConfigError) -> String {
    match error {
        config::ConfigError::Read(err) if err.kind() == std::io::ErrorKind::NotFound => {
            "First-run setup required. No config.yml exists yet.".to_owned()
        }
        _ => format!("Setup required: {error}"),
    }
}

#[cfg(test)]
mod tests;
