mod client;
pub mod discussion;
pub mod graphql;
mod graphql_types;
mod legacy;
pub mod project;
mod project_types;
pub mod rest;

use thiserror::Error;

use crate::models::{AppConfig, HostConfig, ItemPerson, ItemReview, ItemType};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FetchedStreamItem {
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
    pub participants: Vec<ItemPerson>,
    pub mentions: Vec<String>,
}

#[derive(Debug, Error)]
pub enum GitHubError {
    #[error("authentication failed for {host}")]
    Authentication { host: String },
    #[error("GitHub API returned HTTP {status} for {host}")]
    Api { host: String, status: u16 },
    #[error("network connection failed for {host}: {message}")]
    Network { host: String, message: String },
    #[error("GitHub API response could not be parsed for {host}: {message}")]
    Parse { host: String, message: String },
    #[error("GitHub API execution is not implemented yet")]
    NotImplemented,
}

pub struct GitHubService {
    host: HostConfig,
}

impl GitHubService {
    pub fn new(host: HostConfig) -> Self {
        Self { host }
    }

    pub fn host(&self) -> &HostConfig {
        &self.host
    }
}

pub fn test_connection(config: &AppConfig) -> Result<(), GitHubError> {
    rest::test_connection(&config.host, &config.auth.pat)
}

#[cfg(test)]
mod compatibility_tests {
    use super::*;
    use crate::storage::items::StreamItemUpsert;

    type SearchFn = fn(&HostConfig, &str, i64, &str) -> Result<Vec<StreamItemUpsert>, GitHubError>;
    type SearchPageFn =
        fn(&HostConfig, &str, i64, &str, u16, u16) -> Result<rest::SearchPage, GitHubError>;

    #[test]
    fn public_item_api_signatures_remain_compatible() {
        let _: SearchFn = rest::search_issues_and_pull_requests;
        let _: SearchPageFn = rest::search_issues_and_pull_requests_page;
        let _: SearchFn = discussion::search_discussions;
        let _: SearchFn = project::search_project_items;
        let _: fn(&HostConfig, &str, &mut [StreamItemUpsert]) -> Result<(), GitHubError> =
            graphql::enrich_items;
    }
}
