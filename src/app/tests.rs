use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::{
    AppConfig, ItemPerson, ItemReview, ItemType, LibraryView, Selection, SortOrder, StreamFilter,
};
use crate::saved_query_io::read_saved_queries;
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
fn mark_saved_query_read_updates_counts_and_current_view() {
    let (mut app, _) = app_with_one_item();
    let Selection::SavedQuery(query_id) = app.stream.selection else {
        panic!("app should select saved query");
    };

    app.set_filter(Some(StreamFilter::Unread));
    assert_items_len(&app, 1);

    app.mark_saved_query_read(query_id);

    let AppMode::Main(runtime) = &app.mode else {
        panic!("app should be in main mode");
    };
    assert_eq!(runtime.saved_queries[0].unread_count, 0);
    assert!(runtime.items.is_empty());
    assert_eq!(app.status, "Marked 1 items as read.");
}

#[test]
fn changing_saved_query_requests_item_list_scroll_reset() {
    let (mut app, _) = app_with_one_item();
    let current_selection = app.stream.selection.clone();
    let other_query_id = add_query_to_app(&mut app, "Backend");

    app.select(current_selection);
    assert!(!app.stream.reset_item_list_scroll);

    app.select(Selection::SavedQuery(other_query_id));
    assert!(app.stream.reset_item_list_scroll);
}

#[test]
fn query_creation_and_selected_query_deletion_request_item_list_scroll_reset() {
    let (mut app, _) = app_with_one_item();

    app.add_query("New", "is:issue", true);
    assert!(app.stream.reset_item_list_scroll);
    let Selection::SavedQuery(new_query_id) = app.stream.selection else {
        panic!("new query should be selected");
    };

    app.stream.reset_item_list_scroll = false;
    app.delete_query(new_query_id);
    assert!(app.stream.reset_item_list_scroll);
    assert_eq!(app.stream.selection, Selection::Library(LibraryView::Inbox));
}

#[test]
fn toolbar_sort_controls_selected_saved_query_view() {
    let (mut app, _) = app_with_one_item();
    let Selection::SavedQuery(query_id) = app.stream.selection else {
        panic!("app should select saved query");
    };
    insert_item_into_query(
        &mut app,
        query_id,
        sample_item_with_number(100, "Fresh item", "2026-05-24T00:00:00+00:00"),
    );

    app.update_default_sort(SortOrder::UpdatedAsc);

    let AppMode::Main(runtime) = &app.mode else {
        panic!("app should be in main mode");
    };
    assert_eq!(runtime.items[0].title, "Title");
    assert_eq!(runtime.items[1].title, "Fresh item");
}

#[test]
fn changed_item_outside_current_view_does_not_reload_visible_items() {
    let (mut app, item_id) = app_with_one_item();
    let other_query_id = add_query_to_app(&mut app, "Backend");
    let other_item_id = insert_item_into_query(
        &mut app,
        other_query_id,
        sample_item_with_number(99, "Other item", "2026-05-24T00:00:00+00:00"),
    );

    app.reload_current_view_for_changed_items(&[other_item_id]);

    let AppMode::Main(runtime) = &app.mode else {
        panic!("app should be in main mode");
    };
    assert_eq!(runtime.items.len(), 1);
    assert_eq!(runtime.items[0].id, item_id);
}

#[test]
fn changed_item_that_enters_current_view_triggers_reload() {
    let (mut app, item_id) = app_with_one_item();
    let Selection::SavedQuery(query_id) = app.stream.selection else {
        panic!("app should select saved query");
    };
    let inserted_item_id = insert_item_into_query(
        &mut app,
        query_id,
        sample_item_with_number(100, "Fresh item", "2026-05-24T00:00:00+00:00"),
    );

    app.reload_current_view_for_changed_items(&[inserted_item_id]);

    let AppMode::Main(runtime) = &app.mode else {
        panic!("app should be in main mode");
    };
    assert_eq!(runtime.items.len(), 2);
    assert_eq!(runtime.items[0].title, "Fresh item");
    assert!(runtime.items.iter().any(|item| item.id == item_id));
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

#[test]
fn export_queries_writes_yaml_without_runtime_fields() {
    let (mut app, _) = app_with_one_item();
    let path = temp_saved_queries_path("export");

    app.export_queries(path.to_str().expect("path should be utf-8"));

    let imported = read_saved_queries(&path).expect("export should deserialize");
    assert_eq!(imported.queries.len(), 1);
    assert_eq!(imported.queries[0].name, "Inbox");
    assert_eq!(imported.queries[0].query, "is:open");
    let yaml = std::fs::read_to_string(&path).expect("yaml should be readable");
    assert!(!yaml.contains("unread_count"));
    assert!(!yaml.contains("id:"));
}

#[test]
fn import_queries_replaces_existing_queries_and_resets_selection() {
    let (mut app, _) = app_with_one_item();
    let path = temp_saved_queries_path("import");
    std::fs::create_dir_all(path.parent().expect("temp dir")).expect("temp dir");
    std::fs::write(
        &path,
        r#"version: 1
host:
  name: GitHub.com
  scheme: https
  hostname: api.github.com
  rest_api_base_path: /
  kind: github
queries:
  - name: Review requested
    query: "is:pr review-requested:@me"
    enabled: true
    position: 0
  - name: Disabled inbox
    query: "is:issue is:open"
    enabled: false
    position: 1
"#,
    )
    .expect("yaml");

    app.import_queries(path.to_str().expect("path should be utf-8"));

    let AppMode::Main(runtime) = &app.mode else {
        panic!("app should be in main mode");
    };
    assert_eq!(runtime.saved_queries.len(), 2);
    assert_eq!(runtime.saved_queries[0].name, "Review requested");
    assert_eq!(runtime.saved_queries[0].position, 0);
    assert_eq!(runtime.saved_queries[1].name, "Disabled inbox");
    assert!(!runtime.saved_queries[1].enabled);
    assert!(matches!(app.stream.selection, Selection::SavedQuery(_)));
    assert_eq!(
        app.status,
        format!(
            "Imported 2 saved queries from {}. Refresh to rebuild matches.",
            path.display()
        )
    );
}

#[test]
fn import_queries_rejects_host_mismatch() {
    let (mut app, _) = app_with_one_item();
    let path = temp_saved_queries_path("host-mismatch");
    std::fs::create_dir_all(path.parent().expect("temp dir")).expect("temp dir");
    std::fs::write(
        &path,
        r#"version: 1
host:
  name: GHES
  scheme: https
  hostname: ghe.example.test
  rest_api_base_path: /api/v3/
  kind: ghes
queries:
  - name: Review requested
    query: "is:pr review-requested:@me"
    enabled: true
    position: 0
"#,
    )
    .expect("yaml");

    app.import_queries(path.to_str().expect("path should be utf-8"));

    let AppMode::Main(runtime) = &app.mode else {
        panic!("app should be in main mode");
    };
    assert_eq!(runtime.saved_queries.len(), 1);
    assert_eq!(runtime.saved_queries[0].name, "Inbox");
    assert_eq!(
        app.status,
        "Could not import saved queries: saved query file host does not match the current host."
    );
}

fn app_with_one_item() -> (GhStreamApp, i64) {
    let config = AppConfig::default_with_pat("ghp_test".to_owned());
    let storage = Storage::in_memory().expect("storage");
    let host_id = storage.ensure_host(&config.host).expect("host");
    let query_id = storage
        .add_saved_query(host_id, "Inbox", "is:open")
        .expect("query");
    let item_id = storage
        .upsert_stream_item(&sample_item(host_id))
        .expect("item")
        .id;
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
        status_history: vec![StatusEntry::new("Ready")],
        last_poll_at: None,
        refresh_rx: None,
    };
    app.reload_current_view();
    (app, item_id)
}

fn sample_item(host_id: i64) -> StreamItemUpsert {
    sample_item_with_number_for_host(host_id, 42, "Title", "2026-05-23T00:00:00+00:00")
}

fn sample_item_with_number(number: i64, title: &str, updated_at_github: &str) -> StreamItemUpsert {
    sample_item_with_number_for_host(0, number, title, updated_at_github)
}

fn sample_item_with_number_for_host(
    host_id: i64,
    number: i64,
    title: &str,
    updated_at_github: &str,
) -> StreamItemUpsert {
    StreamItemUpsert {
        host_id,
        node_id: Some("node".to_owned()),
        repository_owner: "owner".to_owned(),
        repository_name: "repo".to_owned(),
        number,
        item_type: ItemType::PullRequest,
        title: title.to_owned(),
        author_login: Some("author".to_owned()),
        author_avatar_url: Some("https://avatars.githubusercontent.com/u/1?v=4".to_owned()),
        html_url: format!("https://github.example.test/owner/repo/pull/{number}"),
        api_url: None,
        state: "open".to_owned(),
        is_draft: Some(false),
        is_merged: Some(false),
        review_status: Some("review_required".to_owned()),
        comment_count: 3,
        created_at_github: "2026-05-22T00:00:00+00:00".to_owned(),
        updated_at_github: updated_at_github.to_owned(),
        closed_at_github: None,
        merged_at_github: None,
        labels: vec!["bug".to_owned()],
        assignees: vec![ItemPerson {
            login: "dev".to_owned(),
            avatar_url: Some("https://avatars.githubusercontent.com/u/2?v=4".to_owned()),
        }],
        review_requests: vec![ItemPerson {
            login: "triage".to_owned(),
            avatar_url: Some("https://avatars.githubusercontent.com/u/3?v=4".to_owned()),
        }],
        reviewers: vec![ItemReview {
            login: "reviewer".to_owned(),
            avatar_url: Some("https://avatars.githubusercontent.com/u/4?v=4".to_owned()),
            state: "approved".to_owned(),
        }],
        graphql_enriched: true,
    }
}

fn add_query_to_app(app: &mut GhStreamApp, name: &str) -> i64 {
    let AppMode::Main(runtime) = &mut app.mode else {
        panic!("app should be in main mode");
    };
    runtime
        .storage
        .add_saved_query(runtime.host_id, name, "is:open")
        .expect("query")
}

fn insert_item_into_query(app: &mut GhStreamApp, query_id: i64, mut item: StreamItemUpsert) -> i64 {
    let AppMode::Main(runtime) = &mut app.mode else {
        panic!("app should be in main mode");
    };
    item.host_id = runtime.host_id;
    let item_id = runtime.storage.upsert_stream_item(&item).expect("item").id;
    runtime
        .storage
        .record_saved_query_match(query_id, item_id, Some(0))
        .expect("match");
    item_id
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

fn temp_saved_queries_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir()
        .join("ghstreamlistner-tests")
        .join(format!("{label}-{}-{nanos}.yml", std::process::id()))
}

#[test]
fn status_history_keeps_recent_messages() {
    let (mut app, _) = app_with_one_item();

    for index in 0..=STATUS_HISTORY_LIMIT {
        GhStreamApp::replace_status(
            &mut app.status,
            &mut app.status_history,
            format!("Status {index}"),
        );
    }

    assert_eq!(app.status, format!("Status {STATUS_HISTORY_LIMIT}"));
    assert_eq!(app.status_history.len(), STATUS_HISTORY_LIMIT);
    assert_eq!(app.status_history.first().unwrap().message, "Status 1");
    assert_eq!(
        app.status_history.last().unwrap().message,
        format!("Status {STATUS_HISTORY_LIMIT}")
    );
}
