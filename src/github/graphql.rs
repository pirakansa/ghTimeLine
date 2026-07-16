use std::collections::{HashMap, HashSet};

use super::graphql_types::*;
use crate::github::FetchedStreamItem;
use crate::github::{client, GitHubError};
use crate::models::{HostConfig, ItemPerson, ItemReview, ItemType};

const ITEM_ENRICHMENT_QUERY: &str = r#"
query ItemEnrichment($ids: [ID!]!) {
  nodes(ids: $ids) {
    ... on Issue {
      id
      body
      participants(first: 20) {
        nodes {
          login
          avatarUrl
        }
      }
      comments(first: 20) {
        nodes {
          author {
            ... on User {
              login
              avatarUrl
            }
          }
          body
        }
      }
    }
    ... on PullRequest {
      id
      body
      isDraft
      merged
      mergedAt
      reviewDecision
      reviewRequests(first: 20) {
        totalCount
        nodes {
          requestedReviewer {
            ... on User {
              login
              avatarUrl
            }
          }
        }
      }
      latestReviews(first: 20) {
        nodes {
          state
          body
          author {
            ... on User {
              login
              avatarUrl
            }
          }
        }
      }
      participants(first: 20) {
        nodes {
          login
          avatarUrl
        }
      }
      comments(first: 20) {
        nodes {
          author {
            ... on User {
              login
              avatarUrl
            }
          }
          body
        }
      }
    }
  }
}
"#;
const ITEM_ENRICHMENT_BATCH_SIZE: usize = 50;

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

pub fn enrich_items(
    host: &HostConfig,
    pat: &str,
    items: &mut [FetchedStreamItem],
) -> Result<(), GitHubError> {
    enrich_item_iter(host, pat, items.iter_mut())
}

pub(crate) fn enrich_item_iter<'a>(
    host: &HostConfig,
    pat: &str,
    items: impl IntoIterator<Item = &'a mut FetchedStreamItem>,
) -> Result<(), GitHubError> {
    let mut items = items.into_iter().collect::<Vec<_>>();
    let mut seen_ids = HashSet::new();
    let ids = items
        .iter()
        .filter_map(|item| item.node_id.clone())
        .filter(|node_id| seen_ids.insert(node_id.clone()))
        .collect::<Vec<_>>();

    if ids.is_empty() {
        return Ok(());
    }

    let mut enrichment_by_id = HashMap::new();
    let mut first_error = None;
    for batch in ids.chunks(ITEM_ENRICHMENT_BATCH_SIZE) {
        match fetch_item_enrichment(host, pat, batch) {
            Ok(batch_enrichment) => enrichment_by_id.extend(batch_enrichment),
            Err(error) if first_error.is_none() => first_error = Some(error),
            Err(_) => {}
        }
    }

    for item in &mut items {
        let Some(node_id) = &item.node_id else {
            continue;
        };
        let Some(enrichment) = enrichment_by_id.get(node_id) else {
            if item.item_type == ItemType::PullRequest {
                item.review_status = Some(ReviewSignal::Unknown.as_db_value().to_owned());
            }
            continue;
        };

        if item.item_type == ItemType::PullRequest {
            item.is_draft = enrichment.is_draft;
            item.is_merged = enrichment.merged;
            item.merged_at_github = enrichment.merged_at.clone();
            item.review_status = enrichment
                .review_status
                .as_ref()
                .map(|signal| signal.as_db_value().to_owned());
            item.review_requests = enrichment.review_requests.clone();
            item.reviewers = enrichment.reviewers.clone();
        }
        item.participants = enrichment.participants.clone();
        item.mentions = enrichment.mentions.clone();
        item.graphql_enriched = true;
    }

    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn fetch_item_enrichment(
    host: &HostConfig,
    pat: &str,
    ids: &[String],
) -> Result<HashMap<String, ItemEnrichment>, GitHubError> {
    let request = GraphqlRequest {
        query: ITEM_ENRICHMENT_QUERY,
        variables: GraphqlVariables { ids },
    };
    let body = serde_json::to_string(&request).map_err(|error| GitHubError::Parse {
        host: host.name.clone(),
        message: error.to_string(),
    })?;
    let mut response = client::authenticated_post_json(host, pat, &host.graphql_url(), body)?;
    client::ensure_success(host, &response)?;
    let body = client::read_body(host, pat, &mut response)?;
    parse_item_enrichment(host, &body)
}

fn parse_item_enrichment(
    host: &HostConfig,
    body: &str,
) -> Result<HashMap<String, ItemEnrichment>, GitHubError> {
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
        let review_status = node
            .review_status_fields_present()
            .then(|| derive_review_signal(&node));
        let review_requests = review_requests(&node);
        let reviewers = reviewers(&node);
        let participants = participants(&node, &reviewers);
        let mentions = mentions(&node);
        enrichments.insert(
            node.id.clone(),
            ItemEnrichment {
                is_draft: node.is_draft,
                merged: node.merged,
                merged_at: node.merged_at,
                review_status,
                review_requests,
                reviewers,
                participants,
                mentions,
            },
        );
    }
    Ok(enrichments)
}

fn derive_review_signal(node: &EnrichedNode) -> ReviewSignal {
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

fn review_requests(node: &EnrichedNode) -> Vec<ItemPerson> {
    dedupe_people(node.review_requests.nodes.iter().filter_map(|node| {
        let reviewer = node.requested_reviewer.as_ref()?;
        let login = reviewer.login.as_ref()?;
        Some(ItemPerson {
            login: login.clone(),
            avatar_url: reviewer.avatar_url.clone(),
        })
    }))
}

fn reviewers(node: &EnrichedNode) -> Vec<ItemReview> {
    let mut reviewers = node
        .latest_reviews
        .nodes
        .iter()
        .filter_map(|review| {
            let author = review.author.as_ref()?;
            let login = author.login.as_ref()?;
            normalize_review_state(&review.state).map(|state| ItemReview {
                login: login.clone(),
                avatar_url: author.avatar_url.clone(),
                state: state.to_owned(),
            })
        })
        .collect::<Vec<_>>();
    reviewers.sort_by(|left, right| {
        left.login
            .to_ascii_lowercase()
            .cmp(&right.login.to_ascii_lowercase())
    });
    reviewers
}

fn participants(node: &EnrichedNode, reviewers: &[ItemReview]) -> Vec<ItemPerson> {
    dedupe_people(
        node.participants
            .nodes
            .iter()
            .filter_map(user_to_person)
            .chain(
                node.comments
                    .nodes
                    .iter()
                    .filter_map(|comment| comment.author.as_ref().and_then(user_to_person)),
            )
            .chain(reviewers.iter().map(|review| ItemPerson {
                login: review.login.clone(),
                avatar_url: review.avatar_url.clone(),
            })),
    )
}

fn mentions(node: &EnrichedNode) -> Vec<String> {
    let mut mentions = HashSet::new();
    collect_mentions_from_text(&node.body, &mut mentions);
    for comment in &node.comments.nodes {
        collect_mentions_from_text(&comment.body, &mut mentions);
    }
    for review in &node.latest_reviews.nodes {
        collect_mentions_from_text(&review.body, &mut mentions);
    }
    let mut mentions = mentions.into_iter().collect::<Vec<_>>();
    mentions.sort();
    mentions
}

fn collect_mentions_from_text(text: &str, mentions: &mut HashSet<String>) {
    let chars = text.chars().collect::<Vec<_>>();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] != '@' {
            index += 1;
            continue;
        }

        if index > 0 && is_login_char(chars[index - 1]) {
            index += 1;
            continue;
        }

        let start = index + 1;
        if start >= chars.len() || !chars[start].is_ascii_alphanumeric() {
            index += 1;
            continue;
        }

        let mut end = start + 1;
        while end < chars.len() && is_login_char(chars[end]) {
            end += 1;
        }

        if chars[end - 1] == '-' {
            index = end;
            continue;
        }

        let login = chars[start..end].iter().collect::<String>();
        if !login.is_empty() {
            mentions.insert(login.to_ascii_lowercase());
        }
        index = end;
    }
}

fn is_login_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '-'
}

fn dedupe_people(people: impl IntoIterator<Item = ItemPerson>) -> Vec<ItemPerson> {
    let mut by_login = HashMap::<String, ItemPerson>::new();
    for person in people {
        let key = person.login.to_ascii_lowercase();
        by_login
            .entry(key)
            .and_modify(|existing| {
                if existing.avatar_url.is_none() {
                    existing.avatar_url = person.avatar_url.clone();
                }
            })
            .or_insert(person);
    }

    let mut people = by_login.into_values().collect::<Vec<_>>();
    people.sort_by(|left, right| {
        left.login
            .to_ascii_lowercase()
            .cmp(&right.login.to_ascii_lowercase())
    });
    people
}

fn user_to_person(user: &UserRef) -> Option<ItemPerson> {
    user.login.as_ref().map(|login| ItemPerson {
        login: login.clone(),
        avatar_url: user.avatar_url.clone(),
    })
}

fn normalize_review_state(state: &str) -> Option<&'static str> {
    match state {
        "APPROVED" => Some("approved"),
        "CHANGES_REQUESTED" | "CHANGES_REQUESTED_EVENT" => Some("changes_requested"),
        "COMMENTED" => Some("commented"),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ItemEnrichment {
    is_draft: Option<bool>,
    merged: Option<bool>,
    merged_at: Option<String>,
    review_status: Option<ReviewSignal>,
    review_requests: Vec<ItemPerson>,
    reviewers: Vec<ItemReview>,
    participants: Vec<ItemPerson>,
    mentions: Vec<String>,
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
              "isDraft": true,
              "merged": false,
              "mergedAt": null,
              "body": "Ping @octo and @release-team.",
              "reviewDecision": "REVIEW_REQUIRED",
              "reviewRequests": {
                "totalCount": 2,
                "nodes": [{
                  "requestedReviewer": {
                    "login": "octo",
                    "avatarUrl": "https://avatars.githubusercontent.com/u/1?v=4"
                  }
                }]
              },
              "latestReviews": {
                "nodes": [{
                  "state": "COMMENTED",
                  "body": "Looks good to me @review-ally",
                  "author": {
                    "login": "reviewer",
                    "avatarUrl": "https://avatars.githubusercontent.com/u/2?v=4"
                  }
                }]
              },
              "participants": {
                "nodes": [{
                  "login": "participant",
                  "avatarUrl": "https://avatars.githubusercontent.com/u/3?v=4"
                }]
              },
              "comments": {
                "nodes": [{
                  "body": "Following up with @comment-ally",
                  "author": {
                    "login": "commenter",
                    "avatarUrl": "https://avatars.githubusercontent.com/u/4?v=4"
                  }
                }]
              }
            }]
          }
        }"#;

        let enrichments = parse_item_enrichment(&config.host, body).expect("graphql response");
        let enrichment = enrichments.get("PR_kwDO").expect("enrichment");

        assert_eq!(enrichment.is_draft, Some(true));
        assert_eq!(enrichment.merged, Some(false));
        assert_eq!(enrichment.review_status, Some(ReviewSignal::ReviewRequired));
        assert_eq!(
            enrichment.review_requests,
            vec![ItemPerson {
                login: "octo".to_owned(),
                avatar_url: Some("https://avatars.githubusercontent.com/u/1?v=4".to_owned())
            }]
        );
        assert_eq!(
            enrichment.reviewers,
            vec![ItemReview {
                login: "reviewer".to_owned(),
                avatar_url: Some("https://avatars.githubusercontent.com/u/2?v=4".to_owned()),
                state: "commented".to_owned()
            }]
        );
        assert_eq!(
            enrichment.participants,
            vec![
                ItemPerson {
                    login: "commenter".to_owned(),
                    avatar_url: Some("https://avatars.githubusercontent.com/u/4?v=4".to_owned())
                },
                ItemPerson {
                    login: "participant".to_owned(),
                    avatar_url: Some("https://avatars.githubusercontent.com/u/3?v=4".to_owned())
                },
                ItemPerson {
                    login: "reviewer".to_owned(),
                    avatar_url: Some("https://avatars.githubusercontent.com/u/2?v=4".to_owned())
                }
            ]
        );
        assert_eq!(
            enrichment.mentions,
            vec![
                "comment-ally".to_owned(),
                "octo".to_owned(),
                "release-team".to_owned(),
                "review-ally".to_owned()
            ]
        );
    }

    #[test]
    fn extracts_mentions_from_plain_text() {
        let mut mentions = HashSet::new();

        collect_mentions_from_text(
            "Thanks @octo-team, cc @dev-1. Skip email@example.com and trailing @dash-.",
            &mut mentions,
        );

        let mut mentions = mentions.into_iter().collect::<Vec<_>>();
        mentions.sort();
        assert_eq!(mentions, vec!["dev-1".to_owned(), "octo-team".to_owned()]);
    }

    fn pull_request_node(
        review_decision: Option<&str>,
        review_request_count: i64,
        review_states: Vec<&str>,
    ) -> EnrichedNode {
        EnrichedNode {
            id: "PR_kwDO".to_owned(),
            body: String::new(),
            is_draft: Some(false),
            merged: Some(false),
            merged_at: None,
            review_decision: review_decision.map(ToOwned::to_owned),
            review_requests: ReviewRequests {
                total_count: review_request_count,
                nodes: Vec::new(),
            },
            latest_reviews: LatestReviews {
                nodes: review_states
                    .into_iter()
                    .map(|state| ReviewNode {
                        state: state.to_owned(),
                        body: String::new(),
                        author: None,
                    })
                    .collect(),
            },
            participants: Participants::default(),
            comments: Comments::default(),
        }
    }
}
