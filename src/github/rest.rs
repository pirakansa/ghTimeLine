use crate::github::GitHubError;
use crate::models::{HostConfig, ItemType, SortOrder};
use crate::storage::items::StreamItemUpsert;

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));
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
    let host_name = host.name.clone();
    let authorization = format!("Bearer {pat}");
    let response = ureq::get(&endpoint)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", USER_AGENT)
        .header("Authorization", &authorization)
        .call();

    match response {
        Ok(response) if response.status().is_success() => Ok(()),
        Ok(response) if response.status().as_u16() == 401 || response.status().as_u16() == 403 => {
            Err(GitHubError::Authentication { host: host_name })
        }
        Ok(response) => Err(GitHubError::Api {
            host: host_name,
            status: response.status().as_u16(),
        }),
        Err(error) => Err(GitHubError::Network {
            host: host_name,
            message: sanitize_error_message(&error.to_string(), pat),
        }),
    }
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
    let mut response =
        authenticated_get(&endpoint, pat).map_err(|error| request_error(host, error, pat))?;
    let status = response.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err(GitHubError::Authentication {
            host: host.name.clone(),
        });
    }
    if !status.is_success() {
        return Err(GitHubError::Api {
            host: host.name.clone(),
            status: status.as_u16(),
        });
    }

    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|error| GitHubError::Network {
            host: host.name.clone(),
            message: sanitize_error_message(&error.to_string(), pat),
        })?;
    parse_search_response(host, host_id, &body)
}

pub fn api_url(host: &HostConfig, path: &str) -> String {
    let base = host.rest_api_base_url();
    let path = path.trim_start_matches('/');
    format!("{base}{path}")
}

fn sanitize_error_message(message: &str, pat: &str) -> String {
    if pat.is_empty() {
        message.to_owned()
    } else {
        message.replace(pat, "<redacted>")
    }
}

fn authenticated_get(
    endpoint: &str,
    pat: &str,
) -> Result<ureq::http::Response<ureq::Body>, ureq::Error> {
    let authorization = format!("Bearer {pat}");
    ureq::get(endpoint)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", USER_AGENT)
        .header("Authorization", &authorization)
        .call()
}

fn request_error(host: &HostConfig, error: ureq::Error, pat: &str) -> GitHubError {
    GitHubError::Network {
        host: host.name.clone(),
        message: sanitize_error_message(&error.to_string(), pat),
    }
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
        author_login: item.user.map(|user| user.login),
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
            .map(|assignee| assignee.login)
            .collect(),
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
    fn sanitized_errors_do_not_include_pat() {
        assert_eq!(
            sanitize_error_message("failed with ghp_secret", "ghp_secret"),
            "failed with <redacted>"
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
                "user": { "login": "octo" },
                "labels": [{ "name": "enhancement" }],
                "state": "open",
                "locked": false,
                "assignees": [{ "login": "dev" }],
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
        assert_eq!(items[0].labels, vec!["enhancement"]);
        assert_eq!(items[0].assignees, vec!["dev"]);
    }
}
