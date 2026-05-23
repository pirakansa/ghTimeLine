use gh_stream_listner::models::{
    AppConfig, HostKind, ItemType, LibraryView, SortOrder, StreamFilter,
};
use gh_stream_listner::storage::items::StreamItemUpsert;
use gh_stream_listner::storage::Storage;

#[test]
fn item_state_survives_metadata_upsert() {
    let storage = Storage::in_memory().expect("storage");
    let mut config = AppConfig::default_with_pat("token".to_owned());
    config.host.kind = HostKind::Ghes;
    config.host.hostname = "ghe.example.test".to_owned();
    config.host.rest_api_base_path = "/api/v3/".to_owned();
    let host_id = storage.ensure_host(&config.host).expect("host");
    let query_id = storage
        .add_saved_query(host_id, "Mine", "assignee:@me", SortOrder::UpdatedDesc)
        .expect("query");

    let mut item = sample_item(host_id);
    let item_id = storage.upsert_stream_item(&item).expect("item");
    storage
        .record_saved_query_match(query_id, item_id, Some(0))
        .expect("match");
    storage.set_read_state(item_id, false).expect("read");
    storage.set_bookmarked(item_id, true).expect("bookmark");

    item.title = "Updated title".to_owned();
    storage.upsert_stream_item(&item).expect("updated item");

    let items = storage
        .list_items_for_saved_query(query_id, None, SortOrder::UpdatedDesc)
        .expect("items");

    assert_eq!(items[0].title, "Updated title");
    assert!(!items[0].is_unread);
    assert!(items[0].is_bookmarked);
}

#[test]
fn archived_unread_items_are_excluded_from_query_badges() {
    let storage = Storage::in_memory().expect("storage");
    let config = AppConfig::default_with_pat("token".to_owned());
    let host_id = storage.ensure_host(&config.host).expect("host");
    let query_id = storage
        .add_saved_query(host_id, "Inbox", "is:open", SortOrder::UpdatedDesc)
        .expect("query");
    let item_id = storage
        .upsert_stream_item(&sample_item(host_id))
        .expect("item");
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
        .add_saved_query(host_id, "Inbox", "is:open", SortOrder::UpdatedDesc)
        .expect("query");

    let inbox_item_id = storage
        .upsert_stream_item(&sample_item(host_id))
        .expect("inbox item");
    storage
        .record_saved_query_match(query_id, inbox_item_id, None)
        .expect("inbox match");

    let mut bookmarked_item = sample_item(host_id);
    bookmarked_item.number = 43;
    let bookmarked_item_id = storage
        .upsert_stream_item(&bookmarked_item)
        .expect("bookmarked item");
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
        .expect("archived item");
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
fn saved_query_updates_are_persisted() {
    let storage = Storage::in_memory().expect("storage");
    let config = AppConfig::default_with_pat("token".to_owned());
    let host_id = storage.ensure_host(&config.host).expect("host");
    let query_id = storage
        .add_saved_query(host_id, "Old", "is:open", SortOrder::UpdatedDesc)
        .expect("query");

    storage
        .update_saved_query(
            query_id,
            "Reviews",
            "is:pr review-requested:@me",
            SortOrder::CommentsDesc,
        )
        .expect("update query");

    let queries = storage.list_saved_queries(host_id).expect("queries");

    assert_eq!(queries.len(), 1);
    assert_eq!(queries[0].id, query_id);
    assert_eq!(queries[0].name, "Reviews");
    assert_eq!(queries[0].query, "is:pr review-requested:@me");
    assert_eq!(queries[0].sort, SortOrder::CommentsDesc);
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
