use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedQuery {
    pub id: i64,
    pub name: String,
    pub query: String,
    pub source: StreamSource,
    pub enabled: bool,
    pub position: i64,
    pub unread_count: i64,
    pub filter_streams: Vec<FilterStream>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamSource {
    #[default]
    IssueOrPullRequest,
    Discussion,
    ProjectV2,
}

impl StreamSource {
    pub const ALL: [Self; 3] = [Self::IssueOrPullRequest, Self::Discussion, Self::ProjectV2];

    pub fn label(self) -> &'static str {
        match self {
            Self::IssueOrPullRequest => "Issues and pull requests",
            Self::Discussion => "Discussions",
            Self::ProjectV2 => "Project items",
        }
    }

    pub fn as_db_value(self) -> &'static str {
        match self {
            Self::IssueOrPullRequest => "issue_or_pull_request",
            Self::Discussion => "discussion",
            Self::ProjectV2 => "project_v2",
        }
    }

    pub fn from_db_value(value: &str) -> Self {
        match value {
            "discussion" => Self::Discussion,
            "project_v2" => Self::ProjectV2,
            _ => Self::IssueOrPullRequest,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilterStream {
    pub id: i64,
    pub saved_query_id: i64,
    pub name: String,
    pub filter_query: String,
    pub enabled: bool,
    pub position: i64,
    pub unread_count: i64,
}
