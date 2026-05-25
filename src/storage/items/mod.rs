mod query;
mod relations;
mod state;
mod upsert;

use chrono::{DateTime, Utc};

use crate::models::{ItemPerson, ItemReview, ItemType};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamItemUpsert {
    pub host_id: i64,
    pub node_id: Option<String>,
    pub repository_owner: String,
    pub repository_name: String,
    pub number: i64,
    pub item_type: ItemType,
    pub title: String,
    pub author_login: Option<String>,
    pub author_avatar_url: Option<String>,
    pub html_url: String,
    pub api_url: Option<String>,
    pub state: String,
    pub is_draft: Option<bool>,
    pub is_merged: Option<bool>,
    pub review_status: Option<String>,
    pub comment_count: i64,
    pub created_at_github: String,
    pub updated_at_github: String,
    pub closed_at_github: Option<String>,
    pub merged_at_github: Option<String>,
    pub labels: Vec<String>,
    pub assignees: Vec<ItemPerson>,
    pub review_requests: Vec<ItemPerson>,
    pub reviewers: Vec<ItemReview>,
    pub graphql_enriched: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StreamItemSave {
    pub id: i64,
    pub changed: bool,
}

pub(super) const STREAM_VIEW_LIMIT: usize = 500;

pub(super) fn item_type_db_value(item_type: &ItemType) -> &'static str {
    match item_type {
        ItemType::Issue => "issue",
        ItemType::PullRequest => "pull_request",
    }
}

pub(super) fn item_type_from_db(value: &str) -> ItemType {
    match value {
        "pull_request" => ItemType::PullRequest,
        _ => ItemType::Issue,
    }
}

pub(super) fn github_updated_at_advanced(previous: &str, current: &str) -> bool {
    let previous_datetime = DateTime::parse_from_rfc3339(previous);
    let current_datetime = DateTime::parse_from_rfc3339(current);

    match (previous_datetime, current_datetime) {
        (Ok(previous), Ok(current)) => current > previous,
        _ => current > previous,
    }
}

pub(super) fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}
