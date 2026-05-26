use std::thread::sleep;
use std::time::Duration;

use ghtl::models::{
    AppConfig, HostKind, ItemPerson, ItemReview, ItemType, LibraryView, SortOrder, StreamFilter,
};
use ghtl::saved_query_io::ImportedSavedQuery;
use ghtl::storage::items::StreamItemUpsert;
use ghtl::storage::Storage;

#[test]
fn item_state_survives_metadata_upsert() {
    let storage = Storage::in_memory().expect("storage");
    let mut config = AppConfig::default_with_pat("token".to_owned());
    config.host.kind = HostKind::Ghes;
    config.host.hostname = "ghe.example.test".to_owned();
    config.host.rest_api_base_path = "/api/v3/".to_owned();
    let host_id = storage.ensure_host(&config.host).expect("host");
    let query_id = storage
        .add_saved_query(host_id, "Mine", "assignee:@me")
        .expect("query");

    let mut item = sample_item(host_id);
    let save = storage.upsert_stream_item(&item).expect("item");
    let item_id = save.id;
    assert!(save.changed);
    storage
        .record_saved_query_match(query_id, item_id, Some(0))
        .expect("match");
    storage.set_read_state(item_id, false).expect("read");
    storage.set_bookmarked(item_id, true).expect("bookmark");

    item.title = "Updated title".to_owned();
    let save = storage.upsert_stream_item(&item).expect("updated item");
    assert_eq!(save.id, item_id);
    assert!(!save.changed);

    let items = storage
        .list_items_for_saved_query(query_id, None, None, SortOrder::UpdatedDesc)
        .expect("items");

    assert_eq!(items[0].title, "Updated title");
    assert!(!items[0].is_unread);
    assert!(items[0].is_bookmarked);
    assert_eq!(items[0].assignees[0].login, "dev");
    assert_eq!(items[0].review_requests[0].login, "triage");
    assert_eq!(items[0].reviewers[0].state, "approved");
}

#[test]
fn read_item_becomes_unread_when_github_updated_at_advances() {
    let storage = Storage::in_memory().expect("storage");
    let config = AppConfig::default_with_pat("token".to_owned());
    let host_id = storage.ensure_host(&config.host).expect("host");
    let query_id = storage
        .add_saved_query(host_id, "Mine", "assignee:@me")
        .expect("query");

    let mut item = sample_item(host_id);
    let item_id = storage.upsert_stream_item(&item).expect("item").id;
    storage
        .record_saved_query_match(query_id, item_id, Some(0))
        .expect("match");
    storage.set_read_state(item_id, false).expect("read");

    item.updated_at_github = "2026-05-24T00:00:00+00:00".to_owned();
    let save = storage.upsert_stream_item(&item).expect("updated item");
    assert_eq!(save.id, item_id);
    assert!(save.changed);

    let items = storage
        .list_items_for_saved_query(query_id, None, None, SortOrder::UpdatedDesc)
        .expect("items");

    assert!(items[0].is_unread);
}

#[test]
fn unchanged_upsert_preserves_existing_relation_rows() {
    let storage = Storage::in_memory().expect("storage");
    let config = AppConfig::default_with_pat("token".to_owned());
    let host_id = storage.ensure_host(&config.host).expect("host");
    let query_id = storage
        .add_saved_query(host_id, "Mine", "assignee:@me")
        .expect("query");

    let item_id = storage
        .upsert_stream_item(&sample_item(host_id))
        .expect("item")
        .id;
    storage
        .record_saved_query_match(query_id, item_id, Some(0))
        .expect("match");

    let mut item = sample_item(host_id);
    item.title = "Retitled".to_owned();
    item.labels = vec!["regression".to_owned()];
    item.assignees = vec![ItemPerson {
        login: "other-dev".to_owned(),
        avatar_url: None,
    }];
    item.review_requests = vec![ItemPerson {
        login: "other-reviewer".to_owned(),
        avatar_url: None,
    }];
    item.reviewers = vec![ItemReview {
        login: "approver".to_owned(),
        avatar_url: None,
        state: "changes_requested".to_owned(),
    }];
    let save = storage.upsert_stream_item(&item).expect("updated item");
    assert!(!save.changed);

    let items = storage
        .list_items_for_saved_query(query_id, None, None, SortOrder::UpdatedDesc)
        .expect("items");

    assert_eq!(items[0].title, "Retitled");
    assert_eq!(items[0].labels, vec!["bug".to_owned()]);
    assert_eq!(items[0].assignees[0].login, "dev");
    assert_eq!(items[0].review_requests[0].login, "triage");
    assert_eq!(items[0].reviewers[0].login, "reviewer");
    assert_eq!(items[0].reviewers[0].state, "approved");
}

#[test]
fn changed_upsert_rewrites_relation_rows() {
    let storage = Storage::in_memory().expect("storage");
    let config = AppConfig::default_with_pat("token".to_owned());
    let host_id = storage.ensure_host(&config.host).expect("host");
    let query_id = storage
        .add_saved_query(host_id, "Mine", "assignee:@me")
        .expect("query");

    let item_id = storage
        .upsert_stream_item(&sample_item(host_id))
        .expect("item")
        .id;
    storage
        .record_saved_query_match(query_id, item_id, Some(0))
        .expect("match");

    let mut item = sample_item(host_id);
    item.updated_at_github = "2026-05-24T00:00:00+00:00".to_owned();
    item.labels = vec!["regression".to_owned()];
    item.assignees = vec![ItemPerson {
        login: "other-dev".to_owned(),
        avatar_url: None,
    }];
    item.review_requests = vec![ItemPerson {
        login: "other-reviewer".to_owned(),
        avatar_url: None,
    }];
    item.reviewers = vec![ItemReview {
        login: "approver".to_owned(),
        avatar_url: None,
        state: "changes_requested".to_owned(),
    }];
    let save = storage.upsert_stream_item(&item).expect("updated item");
    assert!(save.changed);

    let items = storage
        .list_items_for_saved_query(query_id, None, None, SortOrder::UpdatedDesc)
        .expect("items");

    assert_eq!(items[0].labels, vec!["regression".to_owned()]);
    assert_eq!(items[0].assignees[0].login, "other-dev");
    assert_eq!(items[0].review_requests[0].login, "other-reviewer");
    assert_eq!(items[0].reviewers[0].login, "approver");
    assert_eq!(items[0].reviewers[0].state, "changes_requested");
}

#[test]
fn archived_unread_items_are_excluded_from_query_badges() {
    let storage = Storage::in_memory().expect("storage");
    let config = AppConfig::default_with_pat("token".to_owned());
    let host_id = storage.ensure_host(&config.host).expect("host");
    let query_id = storage
        .add_saved_query(host_id, "Inbox", "is:open")
        .expect("query");
    let item_id = storage
        .upsert_stream_item(&sample_item(host_id))
        .expect("item")
        .id;
    storage
        .record_saved_query_match(query_id, item_id, None)
        .expect("match");
    storage.set_archived(item_id, true).expect("archive");

    let queries = storage.list_saved_queries(host_id).expect("queries");
    let archived_items = storage
        .list_items_for_library(
            host_id,
            LibraryView::Archived,
            Some(StreamFilter::Unread),
            None,
            SortOrder::UpdatedDesc,
        )
        .expect("archived");

    assert_eq!(queries[0].unread_count, 0);
    assert_eq!(archived_items.len(), 1);
    assert!(archived_items[0].is_unread);
}

#[test]
fn library_unread_counts_cover_inbox_bookmark_and_archived() {
    let storage = Storage::in_memory().expect("storage");
    let config = AppConfig::default_with_pat("token".to_owned());
    let host_id = storage.ensure_host(&config.host).expect("host");
    let query_id = storage
        .add_saved_query(host_id, "Inbox", "is:open")
        .expect("query");

    let inbox_item_id = storage
        .upsert_stream_item(&sample_item(host_id))
        .expect("inbox item")
        .id;
    storage
        .record_saved_query_match(query_id, inbox_item_id, None)
        .expect("inbox match");

    let mut bookmarked_item = sample_item(host_id);
    bookmarked_item.number = 43;
    let bookmarked_item_id = storage
        .upsert_stream_item(&bookmarked_item)
        .expect("bookmarked item")
        .id;
    storage
        .record_saved_query_match(query_id, bookmarked_item_id, None)
        .expect("bookmarked match");
    storage
        .set_bookmarked(bookmarked_item_id, true)
        .expect("bookmark");

    let mut archived_item = sample_item(host_id);
    archived_item.number = 44;
    let archived_item_id = storage
        .upsert_stream_item(&archived_item)
        .expect("archived item")
        .id;
    storage
        .record_saved_query_match(query_id, archived_item_id, None)
        .expect("archived match");
    storage
        .set_archived(archived_item_id, true)
        .expect("archive");

    let library_counts = storage
        .list_library_counts(host_id)
        .expect("library counts");

    assert_eq!(library_counts.inbox_unread_count, 2);
    assert_eq!(library_counts.bookmark_unread_count, 1);
    assert_eq!(library_counts.archived_unread_count, 1);
}

#[test]
fn filter_streams_are_nested_under_saved_queries_and_filter_items_locally() {
    let storage = Storage::in_memory().expect("storage");
    let config = AppConfig::default_with_pat("token".to_owned());
    let host_id = storage.ensure_host(&config.host).expect("host");
    let query_id = storage
        .add_saved_query(host_id, "Inbox", "is:open")
        .expect("query");
    let filter_stream_id = storage
        .add_filter_stream(query_id, "Assigned to dev", "assignee:dev", true)
        .expect("filter stream");

    let first_item_id = storage
        .upsert_stream_item(&sample_item(host_id))
        .expect("first item")
        .id;
    storage
        .record_saved_query_match(query_id, first_item_id, None)
        .expect("first match");

    let mut other_item = sample_item(host_id);
    other_item.number = 43;
    other_item.title = "Other".to_owned();
    other_item.assignees = vec![ItemPerson {
        login: "ops".to_owned(),
        avatar_url: None,
    }];
    let other_item_id = storage
        .upsert_stream_item(&other_item)
        .expect("other item")
        .id;
    storage
        .record_saved_query_match(query_id, other_item_id, None)
        .expect("other match");

    let queries = storage.list_saved_queries(host_id).expect("queries");
    let filtered_items = storage
        .list_items_for_filter_stream(filter_stream_id, None, None, SortOrder::UpdatedDesc)
        .expect("filter stream items");
    let unread_ids = storage
        .list_unread_item_ids_for_filter_stream(filter_stream_id)
        .expect("filter stream unread ids");

    assert_eq!(queries[0].filter_streams.len(), 1);
    assert_eq!(queries[0].filter_streams[0].name, "Assigned to dev");
    assert_eq!(queries[0].filter_streams[0].unread_count, 1);
    assert_eq!(filtered_items.len(), 1);
    assert_eq!(filtered_items[0].title, "Title");
    assert_eq!(unread_ids, vec![first_item_id]);
    assert_ne!(first_item_id, other_item_id);
}

#[test]
fn timestamp_based_sorts_use_requested_fields() {
    let storage = Storage::in_memory().expect("storage");
    let config = AppConfig::default_with_pat("token".to_owned());
    let host_id = storage.ensure_host(&config.host).expect("host");
    let query_id = storage
        .add_saved_query(host_id, "Inbox", "is:open")
        .expect("query");

    let item_a = storage
        .upsert_stream_item(&sample_item(host_id))
        .expect("item a");
    storage
        .record_saved_query_match(query_id, item_a.id, Some(0))
        .expect("match a");

    let mut item_b = sample_item(host_id);
    item_b.number = 43;
    item_b.title = "Closed".to_owned();
    item_b.created_at_github = "2026-05-24T00:00:00+00:00".to_owned();
    item_b.updated_at_github = "2026-05-25T00:00:00+00:00".to_owned();
    item_b.closed_at_github = Some("2026-05-26T00:00:00+00:00".to_owned());
    let item_b = storage.upsert_stream_item(&item_b).expect("item b");
    storage
        .record_saved_query_match(query_id, item_b.id, Some(1))
        .expect("match b");

    let mut item_c = sample_item(host_id);
    item_c.number = 44;
    item_c.title = "Merged".to_owned();
    item_c.created_at_github = "2026-05-26T00:00:00+00:00".to_owned();
    item_c.updated_at_github = "2026-05-27T00:00:00+00:00".to_owned();
    item_c.merged_at_github = Some("2026-05-28T00:00:00+00:00".to_owned());
    let item_c = storage.upsert_stream_item(&item_c).expect("item c");
    storage
        .record_saved_query_match(query_id, item_c.id, Some(2))
        .expect("match c");

    storage.set_read_state(item_b.id, false).expect("read b");
    sleep(Duration::from_millis(5));
    storage.set_read_state(item_c.id, false).expect("read c");

    let created_items = storage
        .list_items_for_saved_query(query_id, None, None, SortOrder::CreatedDesc)
        .expect("created items");
    let read_items = storage
        .list_items_for_saved_query(query_id, None, None, SortOrder::ReadDesc)
        .expect("read items");
    let closed_items = storage
        .list_items_for_saved_query(query_id, None, None, SortOrder::ClosedDesc)
        .expect("closed items");
    let merged_items = storage
        .list_items_for_saved_query(query_id, None, None, SortOrder::MergedDesc)
        .expect("merged items");

    assert_eq!(
        created_items
            .iter()
            .map(|item| item.title.as_str())
            .collect::<Vec<_>>(),
        vec!["Merged", "Closed", "Title"]
    );
    assert_eq!(
        read_items
            .iter()
            .map(|item| item.title.as_str())
            .collect::<Vec<_>>(),
        vec!["Merged", "Closed", "Title"]
    );
    assert_eq!(closed_items[0].title, "Closed");
    assert_eq!(merged_items[0].title, "Merged");
}

#[test]
fn mark_saved_query_read_marks_only_unarchived_matching_items_read() {
    let storage = Storage::in_memory().expect("storage");
    let config = AppConfig::default_with_pat("token".to_owned());
    let host_id = storage.ensure_host(&config.host).expect("host");
    let query_id = storage
        .add_saved_query(host_id, "Inbox", "is:open")
        .expect("query");
    let other_query_id = storage
        .add_saved_query(host_id, "Other", "is:issue")
        .expect("other query");

    let matching_item_id = storage
        .upsert_stream_item(&sample_item(host_id))
        .expect("matching item")
        .id;
    storage
        .record_saved_query_match(query_id, matching_item_id, None)
        .expect("matching match");

    let mut archived_item = sample_item(host_id);
    archived_item.number = 43;
    let archived_item_id = storage
        .upsert_stream_item(&archived_item)
        .expect("archived item")
        .id;
    storage
        .record_saved_query_match(query_id, archived_item_id, None)
        .expect("archived match");
    storage
        .set_archived(archived_item_id, true)
        .expect("archive");

    let mut other_item = sample_item(host_id);
    other_item.number = 44;
    let other_item_id = storage
        .upsert_stream_item(&other_item)
        .expect("other item")
        .id;
    storage
        .record_saved_query_match(other_query_id, other_item_id, None)
        .expect("other match");

    let updated = storage
        .mark_saved_query_read(query_id)
        .expect("mark query read");

    let queries = storage.list_saved_queries(host_id).expect("queries");
    let inbox_items = storage
        .list_items_for_saved_query(query_id, None, None, SortOrder::UpdatedDesc)
        .expect("inbox items");
    let other_items = storage
        .list_items_for_saved_query(other_query_id, None, None, SortOrder::UpdatedDesc)
        .expect("other items");
    let archived_items = storage
        .list_items_for_library(
            host_id,
            LibraryView::Archived,
            Some(StreamFilter::Unread),
            None,
            SortOrder::UpdatedDesc,
        )
        .expect("archived items");

    assert_eq!(updated, 1);
    assert_eq!(queries[0].unread_count, 0);
    assert_eq!(queries[1].unread_count, 1);
    assert_eq!(inbox_items.len(), 1);
    assert!(!inbox_items[0].is_unread);
    assert_eq!(other_items.len(), 1);
    assert!(other_items[0].is_unread);
    assert_eq!(archived_items.len(), 1);
    assert!(archived_items[0].is_unread);
}

#[test]
fn mark_library_read_respects_each_library_scope_and_enabled_queries() {
    let storage = Storage::in_memory().expect("storage");
    let config = AppConfig::default_with_pat("token".to_owned());
    let host_id = storage.ensure_host(&config.host).expect("host");
    let query_id = storage
        .add_saved_query(host_id, "Inbox", "is:open")
        .expect("query");
    let disabled_query_id = storage
        .add_saved_query(host_id, "Disabled", "is:issue")
        .expect("disabled query");
    storage
        .set_saved_query_enabled(disabled_query_id, false)
        .expect("disable query");

    let inbox_item_id = storage
        .upsert_stream_item(&sample_item(host_id))
        .expect("inbox item")
        .id;
    storage
        .record_saved_query_match(query_id, inbox_item_id, None)
        .expect("inbox match");

    let mut bookmarked_item = sample_item(host_id);
    bookmarked_item.number = 43;
    let bookmarked_item_id = storage
        .upsert_stream_item(&bookmarked_item)
        .expect("bookmarked item")
        .id;
    storage
        .record_saved_query_match(query_id, bookmarked_item_id, None)
        .expect("bookmarked match");
    storage
        .set_bookmarked(bookmarked_item_id, true)
        .expect("bookmark");

    let mut archived_item = sample_item(host_id);
    archived_item.number = 44;
    let archived_item_id = storage
        .upsert_stream_item(&archived_item)
        .expect("archived item")
        .id;
    storage
        .record_saved_query_match(query_id, archived_item_id, None)
        .expect("archived match");
    storage
        .set_archived(archived_item_id, true)
        .expect("archive");

    let mut disabled_item = sample_item(host_id);
    disabled_item.number = 45;
    let disabled_item_id = storage
        .upsert_stream_item(&disabled_item)
        .expect("disabled item")
        .id;
    storage
        .record_saved_query_match(disabled_query_id, disabled_item_id, None)
        .expect("disabled match");

    assert_eq!(
        storage
            .list_unread_item_ids_for_library(host_id, LibraryView::Bookmark)
            .expect("bookmark ids"),
        vec![bookmarked_item_id]
    );
    assert_eq!(
        storage
            .mark_library_read(host_id, LibraryView::Bookmark)
            .expect("mark bookmark read"),
        1
    );
    let counts = storage
        .list_library_counts(host_id)
        .expect("bookmark counts");
    assert_eq!(counts.inbox_unread_count, 1);
    assert_eq!(counts.bookmark_unread_count, 0);
    assert_eq!(counts.archived_unread_count, 1);

    storage
        .set_read_state(bookmarked_item_id, true)
        .expect("restore bookmarked unread");
    assert_eq!(
        storage
            .mark_library_read(host_id, LibraryView::Inbox)
            .expect("mark inbox read"),
        2
    );
    let counts = storage.list_library_counts(host_id).expect("inbox counts");
    assert_eq!(counts.inbox_unread_count, 0);
    assert_eq!(counts.archived_unread_count, 1);

    assert_eq!(
        storage
            .mark_library_read(host_id, LibraryView::Archived)
            .expect("mark archived read"),
        1
    );
    assert!(
        storage
            .list_items_for_saved_query(disabled_query_id, None, None, SortOrder::UpdatedDesc)
            .expect("disabled items")[0]
            .is_unread
    );
}

#[test]
fn saved_query_updates_are_persisted() {
    let storage = Storage::in_memory().expect("storage");
    let config = AppConfig::default_with_pat("token".to_owned());
    let host_id = storage.ensure_host(&config.host).expect("host");
    let query_id = storage
        .add_saved_query(host_id, "Old", "is:open")
        .expect("query");

    storage
        .update_saved_query(query_id, "Reviews", "is:pr review-requested:@me")
        .expect("update query");

    let queries = storage.list_saved_queries(host_id).expect("queries");

    assert_eq!(queries.len(), 1);
    assert_eq!(queries[0].id, query_id);
    assert_eq!(queries[0].name, "Reviews");
    assert_eq!(queries[0].query, "is:pr review-requested:@me");
}

#[test]
fn saved_query_enabled_state_is_persisted() {
    let storage = Storage::in_memory().expect("storage");
    let config = AppConfig::default_with_pat("token".to_owned());
    let host_id = storage.ensure_host(&config.host).expect("host");
    let query_id = storage
        .add_saved_query(host_id, "Inbox", "is:open")
        .expect("query");

    storage
        .set_saved_query_enabled(query_id, false)
        .expect("disable query");

    let queries = storage.list_saved_queries(host_id).expect("queries");

    assert_eq!(queries.len(), 1);
    assert_eq!(queries[0].id, query_id);
    assert!(!queries[0].enabled);
}

#[test]
fn saved_query_positions_can_be_reordered() {
    let storage = Storage::in_memory().expect("storage");
    let config = AppConfig::default_with_pat("token".to_owned());
    let host_id = storage.ensure_host(&config.host).expect("host");
    let first_id = storage
        .add_saved_query(host_id, "First", "is:open")
        .expect("first query");
    let second_id = storage
        .add_saved_query(host_id, "Second", "is:pr")
        .expect("second query");
    let third_id = storage
        .add_saved_query(host_id, "Third", "is:issue")
        .expect("third query");

    assert!(storage
        .move_saved_query_down(first_id)
        .expect("move first down"));
    assert!(storage
        .move_saved_query_up(third_id)
        .expect("move third up"));

    let queries = storage.list_saved_queries(host_id).expect("queries");
    let names = queries
        .iter()
        .map(|query| query.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["Second", "Third", "First"]);
    assert!(!storage
        .move_saved_query_up(second_id)
        .expect("top query cannot move up"));
}

#[test]
fn replacing_saved_queries_clears_old_matches_and_preserves_import_order() {
    let storage = Storage::in_memory().expect("storage");
    let config = AppConfig::default_with_pat("token".to_owned());
    let host_id = storage.ensure_host(&config.host).expect("host");
    let query_id = storage
        .add_saved_query(host_id, "Inbox", "is:open")
        .expect("query");
    let item_id = storage
        .upsert_stream_item(&sample_item(host_id))
        .expect("item")
        .id;
    storage
        .record_saved_query_match(query_id, item_id, None)
        .expect("match");

    let inserted_ids = storage
        .replace_saved_queries(
            host_id,
            &[
                ImportedSavedQuery {
                    name: "Review requested".to_owned(),
                    query: "is:pr review-requested:@me".to_owned(),
                    enabled: true,
                    position: 0,
                },
                ImportedSavedQuery {
                    name: "Disabled inbox".to_owned(),
                    query: "is:issue is:open".to_owned(),
                    enabled: false,
                    position: 1,
                },
            ],
        )
        .expect("replace");

    let queries = storage.list_saved_queries(host_id).expect("queries");
    let library_counts = storage
        .list_library_counts(host_id)
        .expect("library counts");

    assert_eq!(inserted_ids.len(), 2);
    assert_eq!(queries[0].name, "Review requested");
    assert_eq!(queries[0].position, 0);
    assert_eq!(queries[1].name, "Disabled inbox");
    assert_eq!(queries[1].position, 1);
    assert_eq!(library_counts.inbox_unread_count, 0);
}

#[test]
fn local_filter_queries_match_supported_metadata() {
    let storage = Storage::in_memory().expect("storage");
    let config = AppConfig::default_with_pat("token".to_owned());
    let host_id = storage.ensure_host(&config.host).expect("host");
    let query_id = storage
        .add_saved_query(host_id, "Inbox", "is:open")
        .expect("query");

    let first_item_id = storage
        .upsert_stream_item(&sample_item(host_id))
        .expect("first item")
        .id;
    storage
        .record_saved_query_match(query_id, first_item_id, Some(0))
        .expect("first match");

    let mut second_item = sample_item(host_id);
    second_item.number = 43;
    second_item.title = "Backend item".to_owned();
    second_item.repository_name = "api".to_owned();
    second_item.author_login = Some("other".to_owned());
    second_item.labels = vec!["regression".to_owned()];
    second_item.assignees = vec![ItemPerson {
        login: "ops".to_owned(),
        avatar_url: None,
    }];
    second_item.review_requests = vec![ItemPerson {
        login: "qa".to_owned(),
        avatar_url: None,
    }];
    second_item.reviewers = vec![ItemReview {
        login: "approver".to_owned(),
        avatar_url: None,
        state: "approved".to_owned(),
    }];
    second_item.participants = vec![ItemPerson {
        login: "comment-helper".to_owned(),
        avatar_url: None,
    }];
    second_item.mentions = vec!["mentioned-helper".to_owned()];
    let second_item_id = storage
        .upsert_stream_item(&second_item)
        .expect("second item")
        .id;
    storage
        .record_saved_query_match(query_id, second_item_id, Some(1))
        .expect("second match");

    let author_items = storage
        .list_items_for_saved_query(
            query_id,
            None,
            Some("author:author"),
            SortOrder::UpdatedDesc,
        )
        .expect("author filter");
    let assignee_items = storage
        .list_items_for_saved_query(query_id, None, Some("assignee:ops"), SortOrder::UpdatedDesc)
        .expect("assignee filter");
    let label_items = storage
        .list_items_for_saved_query(query_id, None, Some("label:bug"), SortOrder::UpdatedDesc)
        .expect("label filter");
    let repo_items = storage
        .list_items_for_saved_query(
            query_id,
            None,
            Some("repo:owner/api"),
            SortOrder::UpdatedDesc,
        )
        .expect("repo filter");
    let requested_items = storage
        .list_items_for_saved_query(
            query_id,
            None,
            Some("review-requested:triage"),
            SortOrder::UpdatedDesc,
        )
        .expect("review-requested filter");
    let reviewed_items = storage
        .list_items_for_saved_query(
            query_id,
            None,
            Some("reviewed-by:approver"),
            SortOrder::UpdatedDesc,
        )
        .expect("reviewed-by filter");
    let involves_items = storage
        .list_items_for_saved_query(
            query_id,
            None,
            Some("involves:triage"),
            SortOrder::UpdatedDesc,
        )
        .expect("involves filter");
    let involves_participant_items = storage
        .list_items_for_saved_query(
            query_id,
            None,
            Some("involves:comment-helper"),
            SortOrder::UpdatedDesc,
        )
        .expect("participant involves filter");
    let involves_mentions_items = storage
        .list_items_for_saved_query(
            query_id,
            None,
            Some("involves:mentioned-helper"),
            SortOrder::UpdatedDesc,
        )
        .expect("mention involves filter");

    assert_eq!(author_items[0].title, "Title");
    assert_eq!(assignee_items[0].title, "Backend item");
    assert_eq!(label_items[0].title, "Title");
    assert_eq!(repo_items[0].title, "Backend item");
    assert_eq!(requested_items[0].title, "Title");
    assert_eq!(reviewed_items[0].title, "Backend item");
    assert_eq!(involves_items[0].title, "Title");
    assert_eq!(involves_participant_items[0].title, "Backend item");
    assert_eq!(involves_mentions_items[0].title, "Backend item");
}

#[test]
fn local_filter_rejects_unsupported_terms() {
    let storage = Storage::in_memory().expect("storage");
    let error = storage
        .validate_local_filter(Some("milestone:v1"))
        .expect_err("unsupported local filter should fail")
        .to_string();

    assert!(error.contains("Unsupported local filter key"));
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
        author_avatar_url: Some("https://avatars.githubusercontent.com/u/1?v=4".to_owned()),
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
        participants: vec![ItemPerson {
            login: "commenter".to_owned(),
            avatar_url: Some("https://avatars.githubusercontent.com/u/5?v=4".to_owned()),
        }],
        mentions: vec!["mentioned-user".to_owned()],
        graphql_enriched: true,
    }
}
