pub mod setup;
pub mod stream;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use eframe::egui;

use crate::config;
use crate::models::{
    AppConfig, LibraryView, SavedQuery, Selection, SortOrder, StreamFilter, StreamItem,
};
use crate::storage::Storage;
use crate::sync;

pub struct GhStreamApp {
    config_path: PathBuf,
    database_path: PathBuf,
    mode: AppMode,
    setup: setup::SetupState,
    stream: stream::StreamState,
    status: String,
    last_poll_at: Option<Instant>,
}

struct Runtime {
    config: AppConfig,
    storage: Storage,
    host_id: i64,
    saved_queries: Vec<SavedQuery>,
    items: Vec<StreamItem>,
}

enum AppMode {
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
            },
        }
    }

    fn open_runtime(
        config: AppConfig,
        database_path: &std::path::Path,
    ) -> crate::storage::Result<Runtime> {
        let storage = Storage::open(database_path)?;
        let host_id = storage.ensure_host(&config.host)?;
        let saved_queries = storage.list_saved_queries(host_id)?;
        Ok(Runtime {
            config,
            storage,
            host_id,
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

    fn reload_queries(&mut self) {
        if let AppMode::Main(runtime) = &mut self.mode {
            match runtime.storage.list_saved_queries(runtime.host_id) {
                Ok(saved_queries) => runtime.saved_queries = saved_queries,
                Err(err) => self.status = format!("Could not load saved queries: {err}"),
            }
        }
    }

    fn reload_current_view(&mut self) {
        if let AppMode::Main(runtime) = &mut self.mode {
            let sort = current_sort(runtime, &self.stream.selection);
            let result = match self.stream.selection.clone() {
                Selection::Library(library) => runtime.storage.list_items_for_library(
                    runtime.host_id,
                    library,
                    self.stream.filter,
                    sort,
                ),
                Selection::SavedQuery(id) => {
                    runtime
                        .storage
                        .list_items_for_saved_query(id, self.stream.filter, sort)
                }
            };

            match result {
                Ok(items) => runtime.items = items,
                Err(err) => self.status = format!("Could not load stream items: {err}"),
            }
        }
    }

    fn add_query(&mut self, name: &str, query: &str) {
        if name.trim().is_empty() || query.trim().is_empty() {
            self.status = "Saved query name and query must not be empty.".to_owned();
            return;
        }

        if let AppMode::Main(runtime) = &mut self.mode {
            match runtime.storage.add_saved_query(
                runtime.host_id,
                name,
                query,
                runtime.config.ui.default_sort,
            ) {
                Ok(id) => {
                    self.stream.selection = Selection::SavedQuery(id);
                    self.status = "Saved query created.".to_owned();
                }
                Err(err) => self.status = format!("Could not create saved query: {err}"),
            }
        }
        self.reload_queries();
        self.reload_current_view();
    }

    fn delete_selected_query(&mut self) {
        let Selection::SavedQuery(id) = self.stream.selection else {
            return;
        };
        if let AppMode::Main(runtime) = &mut self.mode {
            match runtime.storage.delete_saved_query(id) {
                Ok(()) => {
                    self.stream.selection = Selection::Library(LibraryView::Inbox);
                    self.status = "Saved query deleted.".to_owned();
                }
                Err(err) => self.status = format!("Could not delete saved query: {err}"),
            }
        }
        self.reload_queries();
        self.reload_current_view();
    }

    fn set_filter(&mut self, filter: Option<StreamFilter>) {
        self.stream.filter = filter;
        self.reload_current_view();
    }

    fn select(&mut self, selection: Selection) {
        self.stream.selection = selection;
        self.reload_current_view();
    }

    fn update_default_sort(&mut self, sort: SortOrder) {
        if let AppMode::Main(runtime) = &mut self.mode {
            runtime.config.ui.default_sort = sort;
            match config::write_config(&self.config_path, &runtime.config) {
                Ok(()) => self.status = "Sort preference saved.".to_owned(),
                Err(err) => self.status = format!("Could not save sort preference: {err}"),
            }
        }
        self.reload_current_view();
    }

    fn update_polling_interval(&mut self, minutes: u16) {
        if let AppMode::Main(runtime) = &mut self.mode {
            runtime.config.refresh.polling_interval_minutes = minutes;
            match config::write_config(&self.config_path, &runtime.config) {
                Ok(()) => self.status = "Polling interval saved.".to_owned(),
                Err(err) => self.status = format!("Could not save polling interval: {err}"),
            }
        }
    }

    fn item_action(&mut self, action: stream::ItemAction) {
        if let AppMode::Main(runtime) = &mut self.mode {
            let result = match action {
                stream::ItemAction::MarkRead(id) => runtime.storage.set_read_state(id, false),
                stream::ItemAction::MarkUnread(id) => runtime.storage.set_read_state(id, true),
                stream::ItemAction::Bookmark(id, bookmarked) => {
                    runtime.storage.set_bookmarked(id, bookmarked)
                }
                stream::ItemAction::Archive(id) => runtime.storage.set_archived(id, true),
                stream::ItemAction::Open(url) => {
                    return match open::that(url) {
                        Ok(()) => {
                            self.status = "Opened in external browser.".to_owned();
                        }
                        Err(err) => {
                            self.status = format!("Could not open browser: {err}");
                        }
                    };
                }
            };
            match result {
                Ok(()) => self.status = "Item state updated.".to_owned(),
                Err(err) => self.status = format!("Could not update item state: {err}"),
            }
        }
        self.reload_queries();
        self.reload_current_view();
    }

    fn refresh_now(&mut self) {
        self.refresh_selected_scope("Manual refresh");
    }

    fn maybe_poll(&mut self) {
        let AppMode::Main(runtime) = &self.mode else {
            return;
        };
        let interval =
            Duration::from_secs(u64::from(runtime.config.refresh.polling_interval_minutes) * 60);
        if self.last_poll_at.is_none() {
            self.last_poll_at = Some(Instant::now());
            return;
        }
        if self
            .last_poll_at
            .is_none_or(|last_poll_at| last_poll_at.elapsed() >= interval)
        {
            self.last_poll_at = Some(Instant::now());
            self.refresh_all_queries("Polling refresh");
        }
    }

    fn refresh_selected_scope(&mut self, label: &str) {
        let queries = match &self.mode {
            AppMode::Main(runtime) => match self.stream.selection {
                Selection::SavedQuery(id) => runtime
                    .saved_queries
                    .iter()
                    .find(|query| query.id == id)
                    .cloned()
                    .into_iter()
                    .collect::<Vec<_>>(),
                Selection::Library(_) => runtime.saved_queries.clone(),
            },
            AppMode::Setup => Vec::new(),
        };
        self.refresh_queries(label, &queries);
    }

    fn refresh_all_queries(&mut self, label: &str) {
        let queries = match &self.mode {
            AppMode::Main(runtime) => runtime.saved_queries.clone(),
            AppMode::Setup => Vec::new(),
        };
        self.refresh_queries(label, &queries);
    }

    fn refresh_queries(&mut self, label: &str, queries: &[SavedQuery]) {
        if queries.is_empty() {
            self.status = format!("{label}: no saved queries to refresh.");
            return;
        }

        if let AppMode::Main(runtime) = &mut self.mode {
            let results = sync::refresh_saved_queries(
                &runtime.config,
                &runtime.storage,
                runtime.host_id,
                queries,
            );
            let refreshed_count = results
                .iter()
                .filter_map(|(_, result)| result.as_ref().ok())
                .sum::<usize>();
            let failed_count = results.iter().filter(|(_, result)| result.is_err()).count();
            self.status = if failed_count == 0 {
                format!("{label}: refreshed {refreshed_count} items.")
            } else {
                format!(
                    "{label}: refreshed {refreshed_count} items; {failed_count} query refreshes failed."
                )
            };
        }
        self.reload_queries();
        self.reload_current_view();
    }
}

impl Default for GhStreamApp {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for GhStreamApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.maybe_poll();

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
                let event = stream::show(
                    ctx,
                    &mut self.stream,
                    &runtime.config,
                    &runtime.saved_queries,
                    &runtime.items,
                    &self.status,
                );
                self.mode = AppMode::Main(runtime);
                match event {
                    Some(stream::StreamEvent::Select(selection)) => self.select(selection),
                    Some(stream::StreamEvent::SetFilter(filter)) => self.set_filter(filter),
                    Some(stream::StreamEvent::AddQuery { name, query }) => {
                        self.add_query(&name, &query)
                    }
                    Some(stream::StreamEvent::DeleteSelectedQuery) => self.delete_selected_query(),
                    Some(stream::StreamEvent::RefreshNow) => self.refresh_now(),
                    Some(stream::StreamEvent::SetDefaultSort(sort)) => {
                        self.update_default_sort(sort)
                    }
                    Some(stream::StreamEvent::SetPollingInterval(minutes)) => {
                        self.update_polling_interval(minutes)
                    }
                    Some(stream::StreamEvent::ItemAction(action)) => self.item_action(action),
                    None => {}
                }
            }
        }
    }
}

fn current_sort(runtime: &Runtime, selection: &Selection) -> SortOrder {
    match selection {
        Selection::SavedQuery(id) => runtime
            .saved_queries
            .iter()
            .find(|query| query.id == *id)
            .map(|query| query.sort)
            .unwrap_or(runtime.config.ui.default_sort),
        Selection::Library(_) => runtime.config.ui.default_sort,
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
