use gh_stream_listner::models::{AppConfig, HostConfig, HostKind, Scheme, SortOrder};
use gh_stream_listner::storage::Storage;
use gh_stream_listner::sync;
use httptest::matchers::*;
use httptest::responders::*;
use httptest::{Expectation, Server};
use serde_json::json;

#[test]
fn refresh_writes_rest_results_and_graphql_enrichment_to_storage() {
    let server = Server::run();
    server.expect(
        Expectation::matching(all_of![
            request::method_path("GET", "/search/issues"),
            request::query(url_decoded(contains(("q", "is:pr")))),
        ])
        .respond_with(json_encoded(search_response())),
    );
    server.expect(
        Expectation::matching(request::method_path("POST", "/api/graphql"))
            .respond_with(json_encoded(graphql_response("APPROVED", true))),
    );

    let storage = Storage::in_memory().expect("storage");
    let config = config_for_server(&server);
    let host_id = storage.ensure_host(&config.host).expect("host");
    let query_id = storage
        .add_saved_query(host_id, "PRs", "is:pr", SortOrder::UpdatedDesc)
        .expect("saved query");
    let saved_query = storage
        .list_saved_queries(host_id)
        .expect("queries")
        .into_iter()
        .find(|query| query.id == query_id)
        .expect("query");

    let stats = sync::refresh_saved_query(&config, &storage, host_id, &saved_query)
        .expect("refresh should succeed");
    let items = storage
        .list_items_for_saved_query(query_id, None, SortOrder::UpdatedDesc)
        .expect("items");

    assert_eq!(stats.processed_count, 1);
    assert_eq!(stats.changed_count, 1);
    assert_eq!(stats.changed_item_ids.len(), 1);
    assert_eq!(items.len(), 1);
    assert_eq!(stats.changed_item_ids[0], items[0].id);
    assert_eq!(items[0].repository_full_name(), "acme/project");
    assert_eq!(items[0].review_status.as_deref(), Some("approved"));
    assert_eq!(items[0].is_merged, Some(true));
    assert_eq!(items[0].assignees[0].login, "dev");
    assert_eq!(
        items[0].assignees[0].avatar_url.as_deref(),
        Some("https://avatars.githubusercontent.com/u/2?v=4")
    );
    assert_eq!(items[0].review_requests[0].login, "triage");
    assert_eq!(items[0].reviewers[0].login, "reviewer");
    assert_eq!(items[0].reviewers[0].state, "approved");
    assert_eq!(
        items[0].updated_at_github,
        "2026-05-23T00:00:00Z".to_owned()
    );
}

#[test]
fn failed_refresh_preserves_existing_items_and_records_sync_error() {
    let server = Server::run();
    server.expect(
        Expectation::matching(request::method_path("GET", "/search/issues"))
            .times(2)
            .respond_with(cycle(vec![
                Box::new(json_encoded(search_response())),
                Box::new(status_code(500)),
            ])),
    );
    server.expect(
        Expectation::matching(request::method_path("POST", "/api/graphql"))
            .respond_with(json_encoded(graphql_response("REVIEW_REQUIRED", false))),
    );

    let storage = Storage::in_memory().expect("storage");
    let config = config_for_server(&server);
    let host_id = storage.ensure_host(&config.host).expect("host");
    let query_id = storage
        .add_saved_query(host_id, "PRs", "is:pr", SortOrder::UpdatedDesc)
        .expect("saved query");
    let saved_query = storage
        .list_saved_queries(host_id)
        .expect("queries")
        .into_iter()
        .find(|query| query.id == query_id)
        .expect("query");

    sync::refresh_saved_query(&config, &storage, host_id, &saved_query)
        .expect("first refresh should succeed");
    let results = sync::refresh_saved_queries(&config, &storage, host_id, &[saved_query]);
    let items = storage
        .list_items_for_saved_query(query_id, None, SortOrder::UpdatedDesc)
        .expect("items");
    let sync_error: Option<String> = storage
        .connection()
        .query_row(
            "SELECT last_sync_error FROM saved_queries WHERE id = ?1",
            [query_id],
            |row| row.get(0),
        )
        .expect("sync error query");

    assert!(results[0].1.is_err());
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].review_status.as_deref(), Some("review_required"));
    assert!(sync_error
        .expect("last_sync_error should be set")
        .contains("500"));
}

#[test]
fn graphql_failure_preserves_existing_pull_request_enrichment() {
    let server = Server::run();
    server.expect(
        Expectation::matching(request::method_path("GET", "/search/issues"))
            .times(2)
            .respond_with(cycle(vec![
                Box::new(json_encoded(search_response())),
                Box::new(json_encoded(search_response_updated())),
            ])),
    );
    server.expect(
        Expectation::matching(request::method_path("POST", "/api/graphql"))
            .times(2)
            .respond_with(cycle(vec![
                Box::new(json_encoded(graphql_response("APPROVED", true))),
                Box::new(status_code(500)),
            ])),
    );

    let storage = Storage::in_memory().expect("storage");
    let config = config_for_server(&server);
    let host_id = storage.ensure_host(&config.host).expect("host");
    let query_id = storage
        .add_saved_query(host_id, "PRs", "is:pr", SortOrder::UpdatedDesc)
        .expect("saved query");
    let saved_query = storage
        .list_saved_queries(host_id)
        .expect("queries")
        .into_iter()
        .find(|query| query.id == query_id)
        .expect("query");

    sync::refresh_saved_query(&config, &storage, host_id, &saved_query)
        .expect("first refresh should succeed");
    sync::refresh_saved_query(&config, &storage, host_id, &saved_query)
        .expect("REST data should still be saved when GraphQL fails");

    let items = storage
        .list_items_for_saved_query(query_id, None, SortOrder::UpdatedDesc)
        .expect("items");

    assert_eq!(items[0].title, "Improve stream after comment");
    assert_eq!(items[0].updated_at_github, "2026-05-25T00:00:00Z");
    assert_eq!(items[0].review_status.as_deref(), Some("approved"));
    assert_eq!(items[0].is_merged, Some(true));
    assert_eq!(items[0].review_requests[0].login, "triage");
    assert_eq!(items[0].reviewers[0].login, "reviewer");
}

#[test]
fn refreshing_overlapping_queries_updates_shared_item_metadata_once() {
    let server = Server::run();
    server.expect(
        Expectation::matching(request::method_path("GET", "/search/issues"))
            .times(2)
            .respond_with(json_encoded(search_response())),
    );
    server.expect(
        Expectation::matching(request::method_path("POST", "/api/graphql"))
            .respond_with(json_encoded(graphql_response("APPROVED", false))),
    );

    let storage = Storage::in_memory().expect("storage");
    storage
        .connection()
        .execute_batch(
            "CREATE TEMP TABLE stream_item_update_count (updates INTEGER NOT NULL);
             INSERT INTO stream_item_update_count (updates) VALUES (0);
             CREATE TEMP TRIGGER count_stream_item_updates
             AFTER UPDATE ON stream_items
             BEGIN
               UPDATE stream_item_update_count SET updates = updates + 1;
             END;",
        )
        .expect("update counter trigger");
    let config = config_for_server(&server);
    let host_id = storage.ensure_host(&config.host).expect("host");
    storage
        .add_saved_query(host_id, "PRs", "is:pr", SortOrder::UpdatedDesc)
        .expect("first query");
    storage
        .add_saved_query(
            host_id,
            "Reviews",
            "review-requested:@me",
            SortOrder::UpdatedDesc,
        )
        .expect("second query");
    let saved_queries = storage.list_saved_queries(host_id).expect("queries");

    let results = sync::refresh_saved_queries(&config, &storage, host_id, &saved_queries);
    let update_count: i64 = storage
        .connection()
        .query_row("SELECT updates FROM stream_item_update_count", [], |row| {
            row.get(0)
        })
        .expect("update count");

    assert!(results.iter().all(|(_, result)| result.is_ok()));
    assert_eq!(update_count, 0);
}

#[test]
fn refresh_batches_graphql_enrichment_for_large_pull_request_sets() {
    let server = Server::run();
    server.expect(
        Expectation::matching(request::method_path("GET", "/search/issues"))
            .respond_with(json_encoded(search_response_with_pull_requests(51))),
    );
    server.expect(
        Expectation::matching(request::method_path("POST", "/api/graphql"))
            .times(2)
            .respond_with(json_encoded(json!({ "data": { "nodes": [] } }))),
    );

    let storage = Storage::in_memory().expect("storage");
    let config = config_for_server(&server);
    let host_id = storage.ensure_host(&config.host).expect("host");
    storage
        .add_saved_query(host_id, "PRs", "is:pr", SortOrder::UpdatedDesc)
        .expect("query");
    let saved_queries = storage.list_saved_queries(host_id).expect("queries");

    let results = sync::refresh_saved_queries(&config, &storage, host_id, &saved_queries);

    assert_eq!(results[0].1.as_ref().expect("refresh").processed_count, 51);
}

#[test]
fn failed_graphql_batch_does_not_block_successful_batch_enrichment() {
    let server = Server::run();
    server.expect(
        Expectation::matching(request::method_path("GET", "/search/issues"))
            .respond_with(json_encoded(search_response_with_pull_requests(51))),
    );
    server.expect(
        Expectation::matching(request::method_path("POST", "/api/graphql"))
            .times(2)
            .respond_with(cycle(vec![
                Box::new(status_code(500)),
                Box::new(json_encoded(graphql_response_for_node(
                    "PR_51", "APPROVED", true,
                ))),
            ])),
    );

    let storage = Storage::in_memory().expect("storage");
    let config = config_for_server(&server);
    let host_id = storage.ensure_host(&config.host).expect("host");
    let query_id = storage
        .add_saved_query(host_id, "PRs", "is:pr", SortOrder::UpdatedDesc)
        .expect("query");
    let saved_queries = storage.list_saved_queries(host_id).expect("queries");

    let results = sync::refresh_saved_queries(&config, &storage, host_id, &saved_queries);
    let items = storage
        .list_items_for_saved_query(query_id, None, SortOrder::UpdatedDesc)
        .expect("items");
    let last_item = items
        .iter()
        .find(|item| item.number == 51)
        .expect("last item");

    assert!(results[0].1.is_ok());
    assert_eq!(last_item.review_status.as_deref(), Some("approved"));
    assert_eq!(last_item.is_merged, Some(true));
}

fn config_for_server(server: &Server) -> AppConfig {
    AppConfig {
        host: HostConfig {
            name: "Mock GHES".to_owned(),
            scheme: Scheme::Http,
            hostname: server.addr().to_string(),
            rest_api_base_path: "/".to_owned(),
            kind: HostKind::Ghes,
        },
        auth: gh_stream_listner::models::AuthConfig {
            pat: "ghp_test".to_owned(),
        },
        ui: gh_stream_listner::models::UiConfig {
            theme: gh_stream_listner::models::Theme::System,
            accent_color: "#4F8CC9".to_owned(),
            default_sort: SortOrder::UpdatedDesc,
            font_size: gh_stream_listner::models::FontSize::Default,
        },
        refresh: gh_stream_listner::models::RefreshConfig {
            polling_interval_seconds: 180,
        },
    }
}

fn search_response() -> serde_json::Value {
    json!({
        "total_count": 1,
        "incomplete_results": false,
        "items": [{
            "url": "https://api.github.com/repos/acme/project/issues/7",
            "repository_url": "https://api.github.com/repos/acme/project",
            "html_url": "https://github.com/acme/project/pull/7",
            "node_id": "PR_kwDO",
            "number": 7,
            "title": "Improve stream",
            "user": {
                "login": "octo",
                "avatar_url": "https://avatars.githubusercontent.com/u/1?v=4"
            },
            "labels": [{ "name": "enhancement" }],
            "state": "open",
            "assignees": [{
                "login": "dev",
                "avatar_url": "https://avatars.githubusercontent.com/u/2?v=4"
            }],
            "comments": 5,
            "created_at": "2026-05-22T00:00:00Z",
            "updated_at": "2026-05-23T00:00:00Z",
            "closed_at": null,
            "draft": false,
            "pull_request": { "url": "https://api.github.com/repos/acme/project/pulls/7" }
        }]
    })
}

fn search_response_updated() -> serde_json::Value {
    let mut response = search_response();
    let item = response["items"][0].as_object_mut().expect("search item");
    item.insert(
        "title".to_owned(),
        serde_json::Value::String("Improve stream after comment".to_owned()),
    );
    item.insert(
        "updated_at".to_owned(),
        serde_json::Value::String("2026-05-25T00:00:00Z".to_owned()),
    );
    response
}

fn search_response_with_pull_requests(count: i64) -> serde_json::Value {
    let items = (1..=count)
        .map(|number| {
            json!({
                "url": format!("https://api.github.com/repos/acme/project/issues/{number}"),
                "repository_url": "https://api.github.com/repos/acme/project",
                "html_url": format!("https://github.com/acme/project/pull/{number}"),
                "node_id": format!("PR_{number}"),
                "number": number,
                "title": format!("Pull request {number}"),
                "user": null,
                "labels": [],
                "state": "open",
                "assignees": [],
                "comments": 0,
                "created_at": "2026-05-22T00:00:00Z",
                "updated_at": "2026-05-23T00:00:00Z",
                "closed_at": null,
                "draft": false,
                "pull_request": { "url": format!("https://api.github.com/repos/acme/project/pulls/{number}") }
            })
        })
        .collect::<Vec<_>>();
    json!({
        "total_count": count,
        "incomplete_results": false,
        "items": items
    })
}

fn graphql_response(review_decision: &str, merged: bool) -> serde_json::Value {
    graphql_response_for_node("PR_kwDO", review_decision, merged)
}

fn graphql_response_for_node(
    node_id: &str,
    review_decision: &str,
    merged: bool,
) -> serde_json::Value {
    json!({
        "data": {
            "nodes": [{
                "id": node_id,
                "number": 7,
                "title": "Improve stream",
                "state": if merged { "MERGED" } else { "OPEN" },
                "isDraft": false,
                "merged": merged,
                "mergedAt": if merged {
                    serde_json::Value::String("2026-05-24T00:00:00Z".to_owned())
                } else {
                    serde_json::Value::Null
                },
                "reviewDecision": review_decision,
                "reviewRequests": {
                    "totalCount": 1,
                    "nodes": [{
                        "requestedReviewer": {
                            "login": "triage",
                            "avatarUrl": "https://avatars.githubusercontent.com/u/3?v=4"
                        }
                    }]
                },
                "latestReviews": {
                    "nodes": [{
                        "state": "APPROVED",
                        "author": {
                            "login": "reviewer",
                            "avatarUrl": "https://avatars.githubusercontent.com/u/4?v=4"
                        },
                        "submittedAt": "2026-05-24T00:00:00Z"
                    }]
                }
            }]
        }
    })
}
