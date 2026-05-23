pub mod components;
mod refresh;
pub mod setup;
pub mod stream;

use std::path::PathBuf;
use std::time::Instant;

use eframe::egui;

use crate::config;
use crate::models::{
    AppConfig, LibraryCounts, LibraryView, SavedQuery, Selection, SortOrder, StreamFilter,
    StreamItem,
};
use crate::storage::Storage;

pub struct GhStreamApp {
    config_path: PathBuf,
    database_path: PathBuf,
    mode: AppMode,
    setup: setup::SetupState,
    stream: stream::StreamState,
    status: String,
    last_poll_at: Option<Instant>,
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
        let (library_counts, saved_queries) = load_sidebar_data(&storage, host_id)?;
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

    pub(super) fn reload_queries(&mut self) {
        if let AppMode::Main(runtime) = &mut self.mode {
            match load_sidebar_data(&runtime.storage, runtime.host_id) {
                Ok((library_counts, saved_queries)) => {
                    runtime.library_counts = library_counts;
                    runtime.saved_queries = saved_queries;
                }
                Err(err) => self.status = format!("Could not load saved queries: {err}"),
            }
        }
    }

    pub(super) fn reload_current_view(&mut self) {
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

    fn update_query(&mut self, id: i64, name: &str, query: &str, sort: SortOrder) {
        if name.trim().is_empty() || query.trim().is_empty() {
            self.status = "Saved query name and query must not be empty.".to_owned();
            return;
        }

        if let AppMode::Main(runtime) = &mut self.mode {
            match runtime.storage.update_saved_query(id, name, query, sort) {
                Ok(()) => self.status = "Saved query updated.".to_owned(),
                Err(err) => self.status = format!("Could not update saved query: {err}"),
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
                stream::ItemAction::Archive(id, archived) => {
                    runtime.storage.set_archived(id, archived)
                }
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
                    &runtime.library_counts,
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
                    Some(stream::StreamEvent::UpdateQuery {
                        id,
                        name,
                        query,
                        sort,
                    }) => self.update_query(id, &name, &query, sort),
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

fn load_sidebar_data(
    storage: &Storage,
    host_id: i64,
) -> crate::storage::Result<(LibraryCounts, Vec<SavedQuery>)> {
    Ok((
        storage.list_library_counts(host_id)?,
        storage.list_saved_queries(host_id)?,
    ))
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
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::models::{AppConfig, ItemType};
    use crate::storage::items::StreamItemUpsert;

    use super::*;

    #[test]
    fn item_action_updates_storage_and_reloads_current_view() {
        let (mut app, item_id) = app_with_one_item();

        app.item_action(stream::ItemAction::MarkRead(item_id));

        let AppMode::Main(runtime) = &app.mode else {
            panic!("app should be in main mode");
        };
        assert_eq!(runtime.items.len(), 1);
        assert!(!runtime.items[0].is_unread);
        assert_eq!(runtime.saved_queries[0].unread_count, 0);

        app.item_action(stream::ItemAction::Bookmark(item_id, true));

        let AppMode::Main(runtime) = &app.mode else {
            panic!("app should be in main mode");
        };
        assert!(runtime.items[0].is_bookmarked);

        app.item_action(stream::ItemAction::Archive(item_id, true));

        let AppMode::Main(runtime) = &app.mode else {
            panic!("app should be in main mode");
        };
        assert!(runtime.items.is_empty());

        app.select(Selection::Library(LibraryView::Archived));

        let AppMode::Main(runtime) = &app.mode else {
            panic!("app should be in main mode");
        };
        assert_eq!(runtime.items.len(), 1);
        assert!(runtime.items[0].is_archived);

        app.item_action(stream::ItemAction::Archive(item_id, false));

        let AppMode::Main(runtime) = &app.mode else {
            panic!("app should be in main mode");
        };
        assert!(runtime.items.is_empty());
    }

    #[test]
    fn filter_state_drives_db_backed_item_reload() {
        let (mut app, item_id) = app_with_one_item();

        app.set_filter(Some(StreamFilter::Unread));
        assert_items_len(&app, 1);

        app.item_action(stream::ItemAction::MarkRead(item_id));
        assert_items_len(&app, 0);

        app.set_filter(None);
        assert_items_len(&app, 1);
    }

    #[test]
    fn polling_interval_change_updates_runtime_and_yaml_config() {
        let (mut app, _) = app_with_one_item();
        app.update_polling_interval(15);

        let AppMode::Main(runtime) = &app.mode else {
            panic!("app should be in main mode");
        };
        assert_eq!(runtime.config.refresh.polling_interval_minutes, 15);

        let written = config::load_config(&app.config_path).expect("written config should load");
        assert_eq!(written.refresh.polling_interval_minutes, 15);
    }

    fn app_with_one_item() -> (GhStreamApp, i64) {
        let config = AppConfig::default_with_pat("ghp_test".to_owned());
        let storage = Storage::in_memory().expect("storage");
        let host_id = storage.ensure_host(&config.host).expect("host");
        let query_id = storage
            .add_saved_query(host_id, "Inbox", "is:open", SortOrder::UpdatedDesc)
            .expect("query");
        let item_id = storage
            .upsert_stream_item(&sample_item(host_id))
            .expect("item");
        storage
            .record_saved_query_match(query_id, item_id, Some(0))
            .expect("match");
        let saved_queries = storage.list_saved_queries(host_id).expect("queries");
        let library_counts = storage
            .list_library_counts(host_id)
            .expect("library counts");
        let mut app = GhStreamApp {
            config_path: temp_config_path(),
            database_path: std::env::temp_dir().join("ghstreamlistner-test-unused.db"),
            mode: AppMode::Main(Box::new(Runtime {
                config,
                storage,
                host_id,
                library_counts,
                saved_queries,
                items: Vec::new(),
            })),
            setup: setup::SetupState::default(),
            stream: stream::StreamState {
                selection: Selection::SavedQuery(query_id),
                ..Default::default()
            },
            status: "Ready".to_owned(),
            last_poll_at: None,
        };
        app.reload_current_view();
        (app, item_id)
    }

    fn sample_item(host_id: i64) -> StreamItemUpsert {
        StreamItemUpsert {
            host_id,
            node_id: Some("node".to_owned()),
            repository_owner: "owner".to_owned(),
            repository_name: "repo".to_owned(),
            number: 42,
            item_type: ItemType::PullRequest,
            title: "Title".to_owned(),
            author_login: Some("author".to_owned()),
            html_url: "https://github.example.test/owner/repo/pull/42".to_owned(),
            api_url: None,
            state: "open".to_owned(),
            is_draft: Some(false),
            is_merged: Some(false),
            review_status: Some("review_required".to_owned()),
            comment_count: 3,
            created_at_github: "2026-05-22T00:00:00+00:00".to_owned(),
            updated_at_github: "2026-05-23T00:00:00+00:00".to_owned(),
            closed_at_github: None,
            merged_at_github: None,
            labels: vec!["bug".to_owned()],
            assignees: vec!["dev".to_owned()],
        }
    }

    fn assert_items_len(app: &GhStreamApp, expected: usize) {
        let AppMode::Main(runtime) = &app.mode else {
            panic!("app should be in main mode");
        };
        assert_eq!(runtime.items.len(), expected);
    }

    fn temp_config_path() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir()
            .join("ghstreamlistner-tests")
            .join(format!("config-{}-{nanos}.yml", std::process::id()))
    }
}
