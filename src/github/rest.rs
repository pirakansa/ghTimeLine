use crate::github::{client, GitHubError};
use crate::models::{HostConfig, ItemPerson, ItemType, SortOrder};
use crate::storage::items::StreamItemUpsert;

const SEARCH_PER_PAGE: u16 = 50;

pub fn search_sort_query(sort: SortOrder) -> (&'static str, &'static str) {
    match sort {
        SortOrder::UpdatedDesc => ("updated", "desc"),
        SortOrder::UpdatedAsc => ("updated", "asc"),
        SortOrder::CreatedDesc => ("created", "desc"),
        SortOrder::CreatedAsc => ("created", "asc"),
        SortOrder::CommentsDesc => ("comments", "desc"),
        SortOrder::CommentsAsc => ("comments", "asc"),
    }
}

pub fn test_connection(host: &HostConfig, pat: &str) -> Result<(), GitHubError> {
    let endpoint = api_url(host, "user");
    let response = client::authenticated_get(host, pat, &endpoint)?;
    client::ensure_success(host, &response)
}

pub fn search_issues_and_pull_requests(
    host: &HostConfig,
    pat: &str,
    host_id: i64,
    query: &str,
    sort: SortOrder,
) -> Result<Vec<StreamItemUpsert>, GitHubError> {
    let (sort_key, order) = search_sort_query(sort);
    let endpoint = format!(
        "{}?q={}&sort={sort_key}&order={order}&per_page={SEARCH_PER_PAGE}",
        api_url(host, "search/issues"),
        urlencoding::encode(query)
    );
    let mut response = client::authenticated_get(host, pat, &endpoint)?;
    client::ensure_success(host, &response)?;
    let body = client::read_body(host, pat, &mut response)?;
    parse_search_response(host, host_id, &body)
}

pub fn api_url(host: &HostConfig, path: &str) -> String {
    let base = host.rest_api_base_url();
    let path = path.trim_start_matches('/');
    format!("{base}{path}")
}

fn parse_search_response(
    host: &HostConfig,
    host_id: i64,
    body: &str,
) -> Result<Vec<StreamItemUpsert>, GitHubError> {
    let response =
        serde_json::from_str::<SearchResponse>(body).map_err(|error| GitHubError::Parse {
            host: host.name.clone(),
            message: error.to_string(),
        })?;

    response
        .items
        .into_iter()
        .map(|item| search_item_to_upsert(host, host_id, item))
        .collect()
}

fn search_item_to_upsert(
    host: &HostConfig,
    host_id: i64,
    item: SearchItem,
) -> Result<StreamItemUpsert, GitHubError> {
    let (repository_owner, repository_name) = parse_repository_url(&item.repository_url)
        .ok_or_else(|| GitHubError::Parse {
            host: host.name.clone(),
            message: "search result repository_url did not include owner and repository".to_owned(),
        })?;
    let item_type = if item.pull_request.is_some() {
        ItemType::PullRequest
    } else {
        ItemType::Issue
    };
    let review_status = matches!(item_type, ItemType::PullRequest).then(|| "unknown".to_owned());

    Ok(StreamItemUpsert {
        host_id,
        node_id: item.node_id,
        repository_owner,
        repository_name,
        number: item.number,
        item_type,
        title: item.title,
        author_login: item.user.as_ref().map(|user| user.login.clone()),
        author_avatar_url: item.user.and_then(|user| user.avatar_url),
        html_url: item.html_url,
        api_url: Some(item.url),
        state: item.state,
        is_draft: item.draft,
        is_merged: None,
        review_status,
        comment_count: item.comments,
        created_at_github: item.created_at,
        updated_at_github: item.updated_at,
        closed_at_github: item.closed_at,
        merged_at_github: None,
        labels: item.labels.into_iter().map(|label| label.name).collect(),
        assignees: item
            .assignees
            .into_iter()
            .map(|assignee| ItemPerson {
                login: assignee.login,
                avatar_url: assignee.avatar_url,
            })
            .collect(),
        review_requests: Vec::new(),
        reviewers: Vec::new(),
        graphql_enriched: false,
    })
}

fn parse_repository_url(repository_url: &str) -> Option<(String, String)> {
    let (_, suffix) = repository_url.rsplit_once("/repos/")?;
    let mut parts = suffix.split('/');
    let owner = parts.next()?.to_owned();
    let name = parts.next()?.to_owned();
    if owner.is_empty() || name.is_empty() {
        None
    } else {
        Some((owner, name))
    }
}

#[derive(Debug, serde::Deserialize)]
struct SearchResponse {
    items: Vec<SearchItem>,
}

#[derive(Debug, serde::Deserialize)]
struct SearchItem {
    url: String,
    repository_url: String,
    html_url: String,
    node_id: Option<String>,
    number: i64,
    title: String,
    user: Option<SearchUser>,
    labels: Vec<SearchLabel>,
    state: String,
    assignees: Vec<SearchUser>,
    comments: i64,
    created_at: String,
    updated_at: String,
    closed_at: Option<String>,
    draft: Option<bool>,
    pull_request: Option<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize)]
struct SearchUser {
    login: String,
    avatar_url: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct SearchLabel {
    name: String,
}

#[cfg(test)]
mod tests {
    use crate::models::{AppConfig, HostKind};

    use super::*;

    #[test]
    fn api_url_uses_normalized_rest_base_path() {
        let mut config = AppConfig::default_with_pat("token".to_owned());
        config.host.kind = HostKind::Ghes;
        config.host.hostname = "ghe.example.test".to_owned();
        config.host.rest_api_base_path = "/api/v3/".to_owned();

        assert_eq!(
            api_url(&config.host, "/user"),
            "https://ghe.example.test/api/v3/user"
        );
    }

    #[test]
    fn parses_search_response_into_stream_items() {
        let config = AppConfig::default_with_pat("token".to_owned());
        let body = r#"{
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
                "locked": false,
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
        }"#;

        let items = parse_search_response(&config.host, 10, body).expect("search response");

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].repository_owner, "acme");
        assert_eq!(items[0].repository_name, "project");
        assert_eq!(items[0].item_type, ItemType::PullRequest);
        assert_eq!(items[0].review_status.as_deref(), Some("unknown"));
        assert_eq!(
            items[0].author_avatar_url.as_deref(),
            Some("https://avatars.githubusercontent.com/u/1?v=4")
        );
        assert_eq!(items[0].labels, vec!["enhancement"]);
        assert_eq!(
            items[0].assignees,
            vec![ItemPerson {
                login: "dev".to_owned(),
                avatar_url: Some("https://avatars.githubusercontent.com/u/2?v=4".to_owned())
            }]
        );
    }
}
