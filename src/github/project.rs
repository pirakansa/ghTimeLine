use chrono::{DateTime, Utc};

use super::project_types::*;
use crate::github::FetchedStreamItem;
use crate::github::{client, GitHubError};
use crate::models::{HostConfig, ItemPerson, ItemType};

const PROJECT_ITEMS_LIMIT: usize = 500;
const PROJECT_ITEMS_PAGE_SIZE: usize = 100;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ProjectLocator {
    NodeId(String),
    Organization { owner: String, number: i64 },
    User { owner: String, number: i64 },
}

pub fn project_preview_url(host: &HostConfig, query: &str) -> Result<String, GitHubError> {
    project_url(host, &parse_project_locator(host, query)?)
}

pub fn search_project_items(
    host: &HostConfig,
    pat: &str,
    query: &str,
) -> Result<Vec<FetchedStreamItem>, GitHubError> {
    let locator = parse_project_locator(host, query)?;
    let mut items = Vec::new();
    let mut after = None;

    while items.len() < PROJECT_ITEMS_LIMIT {
        let page = fetch_project_items_page(
            host,
            pat,
            &locator,
            after.as_deref(),
            (PROJECT_ITEMS_LIMIT - items.len()).min(PROJECT_ITEMS_PAGE_SIZE),
        )?;
        items.extend(page.items);
        if !page.has_next_page {
            break;
        }
        after = page.end_cursor;
        if after.is_none() {
            break;
        }
    }

    Ok(items)
}

fn parse_project_locator(host: &HostConfig, query: &str) -> Result<ProjectLocator, GitHubError> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err(project_query_error(host));
    }
    if let Some(locator) = parse_project_url(trimmed) {
        return Ok(locator);
    }

    let mut node_id = None;
    let mut org = None;
    let mut user = None;
    let mut number = None;

    for token in trimmed.split_whitespace() {
        if let Some(value) = token.strip_prefix("node:") {
            node_id = Some(value.trim().to_owned());
        } else if let Some(value) = token.strip_prefix("org:") {
            org = Some(value.trim().to_owned());
        } else if let Some(value) = token.strip_prefix("user:") {
            user = Some(value.trim().to_owned());
        } else if let Some(value) = token.strip_prefix("number:") {
            number = value.trim().parse::<i64>().ok();
        }
    }

    if node_id.is_none()
        && org.is_none()
        && user.is_none()
        && !trimmed.contains(':')
        && !trimmed.contains(char::is_whitespace)
    {
        node_id = Some(trimmed.to_owned());
    }

    match (node_id, org, user, number) {
        (Some(node_id), None, None, None) if !node_id.is_empty() => {
            Ok(ProjectLocator::NodeId(node_id))
        }
        (None, Some(owner), None, Some(number)) if !owner.is_empty() && number > 0 => {
            Ok(ProjectLocator::Organization { owner, number })
        }
        (None, None, Some(owner), Some(number)) if !owner.is_empty() && number > 0 => {
            Ok(ProjectLocator::User { owner, number })
        }
        _ => Err(project_query_error(host)),
    }
}

fn parse_project_url(value: &str) -> Option<ProjectLocator> {
    let url = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))?;
    let (_, path) = url.split_once('/')?;
    let mut segments = path.split('/').filter(|segment| !segment.is_empty());
    match segments.next()? {
        "orgs" => {
            let owner = segments.next()?.to_owned();
            if segments.next()? != "projects" {
                return None;
            }
            let number = segments.next()?.parse::<i64>().ok()?;
            Some(ProjectLocator::Organization { owner, number })
        }
        "users" => {
            let owner = segments.next()?.to_owned();
            if segments.next()? != "projects" {
                return None;
            }
            let number = segments.next()?.parse::<i64>().ok()?;
            Some(ProjectLocator::User { owner, number })
        }
        _ => None,
    }
}

fn project_query_error(host: &HostConfig) -> GitHubError {
    GitHubError::Parse {
        host: host.name.clone(),
        message:
            "project query must be one of: node:PROJECT_ID, org:OWNER number:N, user:OWNER number:N"
                .to_owned(),
    }
}

fn project_url(host: &HostConfig, locator: &ProjectLocator) -> Result<String, GitHubError> {
    let web_base = match host.kind {
        crate::models::HostKind::GitHub => format!("{}://github.com", host.scheme),
        crate::models::HostKind::Ghes => format!("{}://{}", host.scheme, host.hostname),
    };
    match locator {
        ProjectLocator::Organization { owner, number } => {
            Ok(format!("{web_base}/orgs/{owner}/projects/{number}"))
        }
        ProjectLocator::User { owner, number } => {
            Ok(format!("{web_base}/users/{owner}/projects/{number}"))
        }
        ProjectLocator::NodeId(_) => Err(GitHubError::Parse {
            host: host.name.clone(),
            message: "project node IDs cannot be previewed as a GitHub project URL".to_owned(),
        }),
    }
}

fn fetch_project_items_page(
    host: &HostConfig,
    pat: &str,
    locator: &ProjectLocator,
    after: Option<&str>,
    first: usize,
) -> Result<ProjectItemsPage, GitHubError> {
    let request = ProjectRequest::new(locator, first, after);
    let body = serde_json::to_string(&request).map_err(|error| GitHubError::Parse {
        host: host.name.clone(),
        message: error.to_string(),
    })?;
    let mut response = client::authenticated_post_json(host, pat, &host.graphql_url(), body)?;
    client::ensure_success(host, &response)?;
    let body = client::read_body(host, pat, &mut response)?;
    parse_project_items_response(host, locator, &body)
}

fn parse_project_items_response(
    host: &HostConfig,
    locator: &ProjectLocator,
    body: &str,
) -> Result<ProjectItemsPage, GitHubError> {
    let response =
        serde_json::from_str::<ProjectResponse>(body).map_err(|error| GitHubError::Parse {
            host: host.name.clone(),
            message: error.to_string(),
        })?;
    if let Some(errors) = response.errors {
        return Err(GitHubError::Parse {
            host: host.name.clone(),
            message: errors
                .into_iter()
                .map(|error| error.message)
                .collect::<Vec<_>>()
                .join("; "),
        });
    }

    let Some(project) = response.data.and_then(|data| data.project(locator)) else {
        return Ok(ProjectItemsPage::default());
    };

    let Some(project_items) = project.items else {
        return Ok(ProjectItemsPage::default());
    };
    let ProjectItems { page_info, nodes } = project_items;

    let items = nodes
        .into_iter()
        .flatten()
        .filter(|item| !item.is_archived)
        .filter_map(|item| project_item_to_fetched(host, item).transpose())
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ProjectItemsPage {
        items,
        has_next_page: page_info.has_next_page,
        end_cursor: page_info.end_cursor,
    })
}

fn project_item_to_fetched(
    host: &HostConfig,
    item: ProjectItem,
) -> Result<Option<FetchedStreamItem>, GitHubError> {
    let Some(content) = item.content else {
        return Ok(None);
    };
    let (repository_owner, repository_name) = content
        .repository
        .name_with_owner
        .split_once('/')
        .ok_or_else(|| GitHubError::Parse {
            host: host.name.clone(),
            message: "project item repository name did not include owner and repository".to_owned(),
        })?;
    let author = content.author;
    let item_type = match content.typename.as_str() {
        "PullRequest" => ItemType::PullRequest,
        "Issue" => ItemType::Issue,
        _ => return Ok(None),
    };
    let review_status = matches!(item_type, ItemType::PullRequest).then(|| "unknown".to_owned());
    let updated_at_github = latest_timestamp(&content.updated_at, &item.updated_at);

    Ok(Some(FetchedStreamItem {
        node_id: Some(content.id),
        repository_owner: repository_owner.to_owned(),
        repository_name: repository_name.to_owned(),
        number: content.number,
        item_type,
        title: content.title,
        author_login: author.as_ref().map(|author| author.login.clone()),
        author_avatar_url: author.and_then(|author| author.avatar_url),
        html_url: content.url,
        api_url: None,
        state: normalize_state(&content.state).to_owned(),
        is_draft: content.is_draft,
        is_merged: content.merged,
        review_status,
        comment_count: content.comments.total_count,
        created_at_github: content.created_at,
        updated_at_github,
        closed_at_github: content.closed_at,
        merged_at_github: content.merged_at,
        labels: content
            .labels
            .nodes
            .into_iter()
            .flatten()
            .map(|label| label.name)
            .collect(),
        assignees: content
            .assignees
            .nodes
            .into_iter()
            .flatten()
            .map(|assignee| ItemPerson {
                login: assignee.login,
                avatar_url: assignee.avatar_url,
            })
            .collect(),
        review_requests: Vec::new(),
        reviewers: Vec::new(),
        participants: Vec::new(),
        mentions: Vec::new(),
        graphql_enriched: false,
    }))
}

fn latest_timestamp(left: &str, right: &str) -> String {
    let left_datetime = DateTime::parse_from_rfc3339(left).map(|value| value.with_timezone(&Utc));
    let right_datetime = DateTime::parse_from_rfc3339(right).map(|value| value.with_timezone(&Utc));
    match (left_datetime, right_datetime) {
        (Ok(left), Ok(right)) if right > left => right.to_rfc3339(),
        (Ok(left), Ok(_)) => left.to_rfc3339(),
        _ if right > left => right.to_owned(),
        _ => left.to_owned(),
    }
}

fn normalize_state(state: &str) -> &str {
    match state {
        "CLOSED" | "MERGED" => "closed",
        _ => "open",
    }
}

#[cfg(test)]
mod tests {
    use crate::models::AppConfig;

    use super::*;

    #[test]
    fn parses_org_project_locator() {
        let config = AppConfig::default_with_pat("token".to_owned());

        assert_eq!(
            parse_project_locator(&config.host, "org:acme number:7").expect("locator"),
            ProjectLocator::Organization {
                owner: "acme".to_owned(),
                number: 7
            }
        );
    }

    #[test]
    fn parses_org_project_url_for_locator_and_preview() {
        let config = AppConfig::default_with_pat("token".to_owned());

        assert_eq!(
            parse_project_locator(&config.host, "https://github.com/orgs/aws/projects/244")
                .expect("locator"),
            ProjectLocator::Organization {
                owner: "aws".to_owned(),
                number: 244
            }
        );
        assert_eq!(
            project_preview_url(&config.host, "org:aws number:244").expect("preview url"),
            "https://github.com/orgs/aws/projects/244"
        );
    }

    #[test]
    fn parses_project_items_into_issue_and_pull_request_stream_items() {
        let config = AppConfig::default_with_pat("token".to_owned());
        let body = r#"{
          "data": {
            "organization": {
              "projectV2": {
                "items": {
                  "pageInfo": { "hasNextPage": false, "endCursor": null },
                  "nodes": [{
                    "id": "PVTI_1",
                    "type": "ISSUE",
                    "isArchived": false,
                    "updatedAt": "2026-05-24T00:00:00Z",
                    "content": {
                      "__typename": "Issue",
                      "id": "I_1",
                      "number": 10,
                      "title": "Track project work",
                      "url": "https://github.com/acme/project/issues/10",
                      "state": "OPEN",
                      "createdAt": "2026-05-22T00:00:00Z",
                      "updatedAt": "2026-05-23T00:00:00Z",
                      "closedAt": null,
                      "comments": { "totalCount": 2 },
                      "repository": { "nameWithOwner": "acme/project" },
                      "author": { "login": "octo", "avatarUrl": null },
                      "labels": { "nodes": [{ "name": "project" }] },
                      "assignees": {
                        "nodes": [{
                          "login": "dev",
                          "avatarUrl": "https://avatars.githubusercontent.com/u/2?v=4"
                        }]
                      }
                    }
                  }, {
                    "id": "PVTI_2",
                    "type": "PULL_REQUEST",
                    "isArchived": false,
                    "updatedAt": "2026-05-25T00:00:00Z",
                    "content": {
                      "__typename": "PullRequest",
                      "id": "PR_1",
                      "number": 11,
                      "title": "Project PR",
                      "url": "https://github.com/acme/project/pull/11",
                      "state": "MERGED",
                      "isDraft": false,
                      "merged": true,
                      "mergedAt": "2026-05-25T00:00:00Z",
                      "createdAt": "2026-05-22T00:00:00Z",
                      "updatedAt": "2026-05-23T00:00:00Z",
                      "closedAt": "2026-05-25T00:00:00Z",
                      "comments": { "totalCount": 4 },
                      "repository": { "nameWithOwner": "acme/project" },
                      "author": null,
                      "labels": { "nodes": [] },
                      "assignees": { "nodes": [] }
                    }
                  }, {
                    "id": "PVTI_3",
                    "type": "REDACTED",
                    "isArchived": false,
                    "updatedAt": "2026-05-25T00:00:00Z",
                    "content": null
                  }]
                }
              }
            }
          }
        }"#;

        let page = parse_project_items_response(
            &config.host,
            &ProjectLocator::Organization {
                owner: "acme".to_owned(),
                number: 7,
            },
            body,
        )
        .expect("project items");

        assert_eq!(page.items.len(), 2);
        assert_eq!(page.items[0].item_type, ItemType::Issue);
        assert_eq!(page.items[0].updated_at_github, "2026-05-24T00:00:00+00:00");
        assert_eq!(page.items[0].labels, vec!["project".to_owned()]);
        assert_eq!(page.items[0].assignees[0].login, "dev");
        assert_eq!(page.items[1].item_type, ItemType::PullRequest);
        assert_eq!(page.items[1].state, "closed");
        assert_eq!(page.items[1].is_merged, Some(true));
    }
}
