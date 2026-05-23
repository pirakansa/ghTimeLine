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

    let count = sync::refresh_saved_query(&config, &storage, host_id, &saved_query)
        .expect("refresh should succeed");
    let items = storage
        .list_items_for_saved_query(query_id, None, SortOrder::UpdatedDesc)
        .expect("items");

    assert_eq!(count, 1);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].repository_full_name(), "acme/project");
    assert_eq!(items[0].review_status.as_deref(), Some("approved"));
    assert_eq!(items[0].is_merged, Some(true));
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
        },
        refresh: gh_stream_listner::models::RefreshConfig {
            polling_interval_minutes: 5,
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
            "user": { "login": "octo" },
            "labels": [{ "name": "enhancement" }],
            "state": "open",
            "assignees": [{ "login": "dev" }],
            "comments": 5,
            "created_at": "2026-05-22T00:00:00Z",
            "updated_at": "2026-05-23T00:00:00Z",
            "closed_at": null,
            "draft": false,
            "pull_request": { "url": "https://api.github.com/repos/acme/project/pulls/7" }
        }]
    })
}

fn graphql_response(review_decision: &str, merged: bool) -> serde_json::Value {
    json!({
        "data": {
            "nodes": [{
                "id": "PR_kwDO",
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
                "reviewRequests": { "totalCount": 0 },
                "latestReviews": { "nodes": [] }
            }]
        }
    })
}
