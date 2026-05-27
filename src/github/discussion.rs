use crate::github::{client, GitHubError};
use crate::models::{HostConfig, ItemType};
use crate::storage::items::StreamItemUpsert;

const SEARCH_LIMIT: usize = 50;
const DISCUSSION_SEARCH_QUERY: &str = r#"
query DiscussionSearch($query: String!, $first: Int!) {
  search(query: $query, type: DISCUSSION, first: $first) {
    nodes {
      ... on Discussion {
        id
        number
        title
        url
        createdAt
        updatedAt
        repository {
          nameWithOwner
        }
        author {
          login
          avatarUrl
        }
        comments {
          totalCount
        }
      }
    }
  }
}
"#;

pub fn search_discussions(
    host: &HostConfig,
    pat: &str,
    host_id: i64,
    query: &str,
) -> Result<Vec<StreamItemUpsert>, GitHubError> {
    let query = format!("{} sort:updated-desc", query.trim());
    let request = GraphqlRequest {
        query: DISCUSSION_SEARCH_QUERY,
        variables: GraphqlVariables {
            query: &query,
            first: SEARCH_LIMIT,
        },
    };
    let body = serde_json::to_string(&request).map_err(|error| GitHubError::Parse {
        host: host.name.clone(),
        message: error.to_string(),
    })?;
    let mut response = client::authenticated_post_json(host, pat, &host.graphql_url(), body)?;
    client::ensure_success(host, &response)?;
    let body = client::read_body(host, pat, &mut response)?;
    parse_search_response(host, host_id, &body)
}

fn parse_search_response(
    host: &HostConfig,
    host_id: i64,
    body: &str,
) -> Result<Vec<StreamItemUpsert>, GitHubError> {
    let response =
        serde_json::from_str::<GraphqlResponse>(body).map_err(|error| GitHubError::Parse {
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

    let Some(data) = response.data else {
        return Ok(Vec::new());
    };
    data.search
        .nodes
        .into_iter()
        .flatten()
        .map(|discussion| discussion_to_upsert(host, host_id, discussion))
        .collect()
}

fn discussion_to_upsert(
    host: &HostConfig,
    host_id: i64,
    discussion: DiscussionNode,
) -> Result<StreamItemUpsert, GitHubError> {
    let (repository_owner, repository_name) = discussion
        .repository
        .name_with_owner
        .split_once('/')
        .ok_or_else(|| GitHubError::Parse {
            host: host.name.clone(),
            message: "discussion repository name did not include owner and repository".to_owned(),
        })?;
    Ok(StreamItemUpsert {
        host_id,
        node_id: Some(discussion.id),
        repository_owner: repository_owner.to_owned(),
        repository_name: repository_name.to_owned(),
        number: discussion.number,
        item_type: ItemType::Discussion,
        title: discussion.title,
        author_login: discussion
            .author
            .as_ref()
            .map(|author| author.login.clone()),
        author_avatar_url: discussion.author.and_then(|author| author.avatar_url),
        html_url: discussion.url,
        api_url: None,
        state: "open".to_owned(),
        is_draft: None,
        is_merged: None,
        review_status: None,
        comment_count: discussion.comments.total_count,
        created_at_github: discussion.created_at,
        updated_at_github: discussion.updated_at,
        closed_at_github: None,
        merged_at_github: None,
        labels: Vec::new(),
        assignees: Vec::new(),
        review_requests: Vec::new(),
        reviewers: Vec::new(),
        participants: Vec::new(),
        mentions: Vec::new(),
        graphql_enriched: true,
    })
}

#[derive(serde::Serialize)]
struct GraphqlRequest<'a> {
    query: &'static str,
    variables: GraphqlVariables<'a>,
}

#[derive(serde::Serialize)]
struct GraphqlVariables<'a> {
    query: &'a str,
    first: usize,
}

#[derive(serde::Deserialize)]
struct GraphqlResponse {
    data: Option<GraphqlData>,
    errors: Option<Vec<GraphqlError>>,
}

#[derive(serde::Deserialize)]
struct GraphqlError {
    message: String,
}

#[derive(serde::Deserialize)]
struct GraphqlData {
    search: DiscussionSearch,
}

#[derive(serde::Deserialize)]
struct DiscussionSearch {
    nodes: Vec<Option<DiscussionNode>>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiscussionNode {
    id: String,
    number: i64,
    title: String,
    url: String,
    created_at: String,
    updated_at: String,
    repository: DiscussionRepository,
    author: Option<DiscussionAuthor>,
    comments: DiscussionComments,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiscussionRepository {
    name_with_owner: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiscussionAuthor {
    login: String,
    avatar_url: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiscussionComments {
    total_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AppConfig;

    #[test]
    fn parses_discussion_search_into_stream_items() {
        let config = AppConfig::default_with_pat("token".to_owned());
        let body = r#"{
          "data": {
            "search": {
              "nodes": [{
                "id": "D_kwDO",
                "number": 12,
                "title": "Release feedback",
                "url": "https://github.com/acme/project/discussions/12",
                "createdAt": "2026-05-22T00:00:00Z",
                "updatedAt": "2026-05-23T00:00:00Z",
                "repository": { "nameWithOwner": "acme/project" },
                "author": { "login": "octo", "avatarUrl": null },
                "comments": { "totalCount": 3 }
              }]
            }
          }
        }"#;

        let items = parse_search_response(&config.host, 10, body).expect("discussion result");

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].item_type, ItemType::Discussion);
        assert_eq!(items[0].repository_owner, "acme");
        assert_eq!(items[0].comment_count, 3);
    }
}
