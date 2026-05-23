use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::{AppConfig, ItemType, LibraryView, Selection, SortOrder, StreamFilter};
use crate::storage::items::StreamItemUpsert;

use super::*;

#[test]
fn item_action_updates_storage_and_reloads_current_view() {
    let (mut app, item_id) = app_with_one_item();

    app.item_action(screens::stream::ItemAction::MarkRead(item_id));

    let AppMode::Main(runtime) = &app.mode else {
        panic!("app should be in main mode");
    };
    assert_eq!(runtime.items.len(), 1);
    assert!(!runtime.items[0].is_unread);
    assert_eq!(runtime.saved_queries[0].unread_count, 0);

    app.item_action(screens::stream::ItemAction::Bookmark(item_id, true));

    let AppMode::Main(runtime) = &app.mode else {
        panic!("app should be in main mode");
    };
    assert!(runtime.items[0].is_bookmarked);

    app.item_action(screens::stream::ItemAction::Archive(item_id, true));

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

    app.item_action(screens::stream::ItemAction::Archive(item_id, false));

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

    app.item_action(screens::stream::ItemAction::MarkRead(item_id));
    assert_items_len(&app, 0);

    app.set_filter(None);
    assert_items_len(&app, 1);
}

#[test]
fn polling_interval_change_updates_runtime_and_yaml_config() {
    let (mut app, _) = app_with_one_item();
    app.update_polling_interval(90);

    let AppMode::Main(runtime) = &app.mode else {
        panic!("app should be in main mode");
    };
    assert_eq!(runtime.config.refresh.polling_interval_seconds, 90);

    let written = config::load_config(&app.config_path).expect("written config should load");
    assert_eq!(written.refresh.polling_interval_seconds, 90);
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
        setup: screens::setup::SetupState::default(),
        stream: screens::stream::StreamState {
            selection: Selection::SavedQuery(query_id),
            ..Default::default()
        },
        status: "Ready".to_owned(),
        last_poll_at: None,
        refresh_rx: None,
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
