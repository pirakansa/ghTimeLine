use std::collections::HashMap;

use crate::github::{client, GitHubError};
use crate::models::{HostConfig, ItemType};
use crate::storage::items::StreamItemUpsert;

const PULL_REQUEST_ENRICHMENT_QUERY: &str = r#"
query PullRequestEnrichment($ids: [ID!]!) {
  nodes(ids: $ids) {
    ... on PullRequest {
      id
      number
      title
      state
      isDraft
      merged
      mergedAt
      reviewDecision
      reviewRequests(first: 20) {
        totalCount
      }
      latestReviews(first: 20) {
        nodes {
          state
          author {
            login
          }
          submittedAt
        }
      }
    }
  }
}
"#;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReviewSignal {
    None,
    ReviewRequired,
    ChangesRequested,
    Approved,
    Unknown,
}

impl ReviewSignal {
    pub fn as_db_value(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::ReviewRequired => "review_required",
            Self::ChangesRequested => "changes_requested",
            Self::Approved => "approved",
            Self::Unknown => "unknown",
        }
    }
}

pub fn enrich_pull_requests(
    host: &HostConfig,
    pat: &str,
    items: &mut [StreamItemUpsert],
) -> Result<(), GitHubError> {
    let ids = items
        .iter()
        .filter(|item| item.item_type == ItemType::PullRequest)
        .filter_map(|item| item.node_id.clone())
        .collect::<Vec<_>>();

    if ids.is_empty() {
        return Ok(());
    }

    let enrichment_by_id = fetch_pull_request_enrichment(host, pat, &ids)?;
    for item in items {
        let Some(node_id) = &item.node_id else {
            continue;
        };
        let Some(enrichment) = enrichment_by_id.get(node_id) else {
            item.review_status = Some(ReviewSignal::Unknown.as_db_value().to_owned());
            continue;
        };
        item.is_draft = Some(enrichment.is_draft);
        item.is_merged = Some(enrichment.merged);
        item.merged_at_github = enrichment.merged_at.clone();
        item.review_status = Some(enrichment.review_status.as_db_value().to_owned());
    }

    Ok(())
}

fn fetch_pull_request_enrichment(
    host: &HostConfig,
    pat: &str,
    ids: &[String],
) -> Result<HashMap<String, PullRequestEnrichment>, GitHubError> {
    let request = GraphqlRequest {
        query: PULL_REQUEST_ENRICHMENT_QUERY,
        variables: GraphqlVariables { ids },
    };
    let body = serde_json::to_string(&request).map_err(|error| GitHubError::Parse {
        host: host.name.clone(),
        message: error.to_string(),
    })?;
    let mut response = client::authenticated_post_json(host, pat, &host.graphql_url(), body)?;
    client::ensure_success(host, &response)?;
    let body = client::read_body(host, pat, &mut response)?;
    parse_pull_request_enrichment(host, &body)
}

fn parse_pull_request_enrichment(
    host: &HostConfig,
    body: &str,
) -> Result<HashMap<String, PullRequestEnrichment>, GitHubError> {
    let response =
        serde_json::from_str::<GraphqlResponse>(body).map_err(|error| GitHubError::Parse {
            host: host.name.clone(),
            message: error.to_string(),
        })?;
    if let Some(errors) = response.errors {
        let message = errors
            .into_iter()
            .map(|error| error.message)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(GitHubError::Parse {
            host: host.name.clone(),
            message,
        });
    }

    let mut enrichments = HashMap::new();
    let Some(data) = response.data else {
        return Ok(enrichments);
    };
    for node in data.nodes.into_iter().flatten() {
        let review_status = derive_review_signal(&node);
        enrichments.insert(
            node.id.clone(),
            PullRequestEnrichment {
                is_draft: node.is_draft,
                merged: node.merged,
                merged_at: node.merged_at,
                review_status,
            },
        );
    }
    Ok(enrichments)
}

fn derive_review_signal(node: &PullRequestNode) -> ReviewSignal {
    if node.review_decision.as_deref() == Some("CHANGES_REQUESTED")
        || node.latest_reviews.nodes.iter().any(|review| {
            review.state == "CHANGES_REQUESTED" || review.state == "CHANGES_REQUESTED_EVENT"
        })
    {
        return ReviewSignal::ChangesRequested;
    }

    match node.review_decision.as_deref() {
        Some("APPROVED") => ReviewSignal::Approved,
        Some("REVIEW_REQUIRED") => ReviewSignal::ReviewRequired,
        _ if node.review_requests.total_count > 0 => ReviewSignal::ReviewRequired,
        _ => ReviewSignal::None,
    }
}

#[derive(serde::Serialize)]
struct GraphqlRequest<'a> {
    query: &'static str,
    variables: GraphqlVariables<'a>,
}

#[derive(serde::Serialize)]
struct GraphqlVariables<'a> {
    ids: &'a [String],
}

#[derive(Debug, serde::Deserialize)]
struct GraphqlResponse {
    data: Option<GraphqlData>,
    errors: Option<Vec<GraphqlError>>,
}

#[derive(Debug, serde::Deserialize)]
struct GraphqlError {
    message: String,
}

#[derive(Debug, serde::Deserialize)]
struct GraphqlData {
    nodes: Vec<Option<PullRequestNode>>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PullRequestNode {
    id: String,
    is_draft: bool,
    merged: bool,
    merged_at: Option<String>,
    review_decision: Option<String>,
    review_requests: ReviewRequests,
    latest_reviews: LatestReviews,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReviewRequests {
    total_count: i64,
}

#[derive(Debug, serde::Deserialize)]
struct LatestReviews {
    nodes: Vec<ReviewNode>,
}

#[derive(Debug, serde::Deserialize)]
struct ReviewNode {
    state: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PullRequestEnrichment {
    is_draft: bool,
    merged: bool,
    merged_at: Option<String>,
    review_status: ReviewSignal,
}

#[cfg(test)]
mod tests {
    use crate::models::AppConfig;

    use super::*;

    #[test]
    fn derives_changes_requested_from_review_decision() {
        let node = pull_request_node(Some("CHANGES_REQUESTED"), 0, Vec::new());

        assert_eq!(derive_review_signal(&node), ReviewSignal::ChangesRequested);
    }

    #[test]
    fn derives_review_required_from_review_requests() {
        let node = pull_request_node(None, 1, Vec::new());

        assert_eq!(derive_review_signal(&node), ReviewSignal::ReviewRequired);
    }

    #[test]
    fn parses_graphql_enrichment() {
        let config = AppConfig::default_with_pat("token".to_owned());
        let body = r#"{
          "data": {
            "nodes": [{
              "id": "PR_kwDO",
              "number": 7,
              "title": "Improve stream",
              "state": "OPEN",
              "isDraft": true,
              "merged": false,
              "mergedAt": null,
              "reviewDecision": "REVIEW_REQUIRED",
              "reviewRequests": { "totalCount": 2 },
              "latestReviews": {
                "nodes": [{
                  "state": "COMMENTED",
                  "author": { "login": "octo" },
                  "submittedAt": "2026-05-23T00:00:00Z"
                }]
              }
            }]
          }
        }"#;

        let enrichments =
            parse_pull_request_enrichment(&config.host, body).expect("graphql response");
        let enrichment = enrichments.get("PR_kwDO").expect("enrichment");

        assert!(enrichment.is_draft);
        assert!(!enrichment.merged);
        assert_eq!(enrichment.review_status, ReviewSignal::ReviewRequired);
    }

    fn pull_request_node(
        review_decision: Option<&str>,
        review_request_count: i64,
        review_states: Vec<&str>,
    ) -> PullRequestNode {
        PullRequestNode {
            id: "PR_kwDO".to_owned(),
            is_draft: false,
            merged: false,
            merged_at: None,
            review_decision: review_decision.map(ToOwned::to_owned),
            review_requests: ReviewRequests {
                total_count: review_request_count,
            },
            latest_reviews: LatestReviews {
                nodes: review_states
                    .into_iter()
                    .map(|state| ReviewNode {
                        state: state.to_owned(),
                    })
                    .collect(),
            },
        }
    }
}
