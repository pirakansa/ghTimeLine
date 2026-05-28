use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HostKind {
    GitHub,
    Ghes,
}

impl HostKind {
    pub fn graphql_path(&self) -> &'static str {
        match self {
            Self::GitHub => "/graphql",
            Self::Ghes => "/api/graphql",
        }
    }
}

impl fmt::Display for HostKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GitHub => f.write_str("github"),
            Self::Ghes => f.write_str("ghes"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scheme {
    Http,
    Https,
}

impl fmt::Display for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Http => f.write_str("http"),
            Self::Https => f.write_str("https"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Light,
    Dark,
    System,
}

impl Theme {
    pub fn label(self) -> &'static str {
        match self {
            Theme::Light => "Light",
            Theme::Dark => "Dark",
            Theme::System => "System",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    #[serde(alias = "updated_asc", alias = "comments_desc", alias = "comments_asc")]
    UpdatedDesc,
    #[serde(alias = "created_asc")]
    CreatedDesc,
    ReadDesc,
    ClosedDesc,
    MergedDesc,
}

impl SortOrder {
    pub const ALL: [Self; 5] = [
        Self::UpdatedDesc,
        Self::CreatedDesc,
        Self::ReadDesc,
        Self::ClosedDesc,
        Self::MergedDesc,
    ];

    pub fn as_db_value(self) -> &'static str {
        match self {
            Self::UpdatedDesc => "updated_desc",
            Self::CreatedDesc => "created_desc",
            Self::ReadDesc => "read_desc",
            Self::ClosedDesc => "closed_desc",
            Self::MergedDesc => "merged_desc",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::UpdatedDesc => "Updated at",
            Self::CreatedDesc => "Created at",
            Self::ReadDesc => "Read at",
            Self::ClosedDesc => "Closed at",
            Self::MergedDesc => "Merged at",
        }
    }
}

impl fmt::Display for SortOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_db_value())
    }
}

impl FromStr for SortOrder {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "updated_desc" => Ok(Self::UpdatedDesc),
            "updated_asc" => Ok(Self::UpdatedDesc),
            "created_desc" => Ok(Self::CreatedDesc),
            "created_asc" => Ok(Self::CreatedDesc),
            "read_desc" => Ok(Self::ReadDesc),
            "closed_desc" => Ok(Self::ClosedDesc),
            "merged_desc" => Ok(Self::MergedDesc),
            "comments_desc" => Ok(Self::UpdatedDesc),
            "comments_asc" => Ok(Self::UpdatedDesc),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamFilter {
    Open,
    Unread,
    Bookmarked,
}

impl StreamFilter {
    pub const ALL: [Self; 3] = [Self::Open, Self::Unread, Self::Bookmarked];

    pub fn label(self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::Unread => "Unread",
            Self::Bookmarked => "Bookmarked",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryView {
    Inbox,
    Bookmark,
    Archived,
}

impl LibraryView {
    pub const ALL: [Self; 3] = [Self::Inbox, Self::Bookmark, Self::Archived];

    pub fn label(self) -> &'static str {
        match self {
            Self::Inbox => "Inbox",
            Self::Bookmark => "Bookmark",
            Self::Archived => "Archived",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selection {
    Library(LibraryView),
    SavedQuery(i64),
    FilterStream(i64),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostConfig {
    pub name: String,
    pub scheme: Scheme,
    pub hostname: String,
    pub rest_api_base_path: String,
    pub kind: HostKind,
}

impl HostConfig {
    pub fn github_default() -> Self {
        Self {
            name: "GitHub.com".to_owned(),
            scheme: Scheme::Https,
            hostname: "api.github.com".to_owned(),
            rest_api_base_path: "/".to_owned(),
            kind: HostKind::GitHub,
        }
    }

    pub fn fingerprint(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.kind, self.scheme, self.hostname, self.rest_api_base_path
        )
    }

    pub fn rest_api_base_url(&self) -> String {
        format!(
            "{}://{}{}",
            self.scheme, self.hostname, self.rest_api_base_path
        )
    }

    pub fn graphql_url(&self) -> String {
        match self.kind {
            HostKind::GitHub => format!("{}://api.github.com/graphql", self.scheme),
            HostKind::Ghes => format!(
                "{}://{}{}",
                self.scheme,
                self.hostname,
                self.kind.graphql_path()
            ),
        }
    }

    pub fn search_url(&self, query: &str) -> String {
        self.search_url_for(StreamSource::IssueOrPullRequest, query)
    }

    pub fn search_url_for(&self, source: StreamSource, query: &str) -> String {
        let web_base = match self.kind {
            HostKind::GitHub => format!("{}://github.com", self.scheme),
            HostKind::Ghes => format!("{}://{}", self.scheme, self.hostname),
        };
        format!(
            "{web_base}/search?q={}&type={}",
            urlencoding::encode(query.trim()),
            match source {
                StreamSource::IssueOrPullRequest => "issues",
                StreamSource::Discussion => "discussions",
            }
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthConfig {
    pub pat: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FontSize {
    XSmall,
    Small,
    Default,
    Large,
    XLarge,
}

impl FontSize {
    pub fn label(self) -> &'static str {
        match self {
            FontSize::XSmall => "X-Small",
            FontSize::Small => "Small",
            FontSize::Default => "Default",
            FontSize::Large => "Large",
            FontSize::XLarge => "X-Large",
        }
    }

    pub fn scale(self) -> f32 {
        match self {
            FontSize::XSmall => 0.80,
            FontSize::Small => 0.90,
            FontSize::Default => 1.0,
            FontSize::Large => 1.1,
            FontSize::XLarge => 1.2,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: Theme,
    pub accent_color: String,
    pub default_sort: SortOrder,
    pub font_size: FontSize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefreshConfig {
    pub polling_interval_seconds: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub host: HostConfig,
    pub auth: AuthConfig,
    pub ui: UiConfig,
    pub refresh: RefreshConfig,
}

impl AppConfig {
    pub fn default_with_pat(pat: String) -> Self {
        Self {
            host: HostConfig::github_default(),
            auth: AuthConfig { pat },
            ui: UiConfig {
                theme: Theme::System,
                accent_color: "#4F8CC9".to_owned(),
                default_sort: SortOrder::UpdatedDesc,
                font_size: FontSize::Default,
            },
            refresh: RefreshConfig {
                polling_interval_seconds: 180,
            },
        }
    }
}

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
}

impl StreamSource {
    pub const ALL: [Self; 2] = [Self::IssueOrPullRequest, Self::Discussion];

    pub fn label(self) -> &'static str {
        match self {
            Self::IssueOrPullRequest => "Issues and pull requests",
            Self::Discussion => "Discussions",
        }
    }

    pub fn as_db_value(self) -> &'static str {
        match self {
            Self::IssueOrPullRequest => "issue_or_pull_request",
            Self::Discussion => "discussion",
        }
    }

    pub fn from_db_value(value: &str) -> Self {
        match value {
            "discussion" => Self::Discussion,
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LibraryCounts {
    pub inbox_unread_count: i64,
    pub bookmark_unread_count: i64,
    pub archived_unread_count: i64,
}

impl LibraryCounts {
    pub fn unread_count(&self, library: LibraryView) -> i64 {
        match library {
            LibraryView::Inbox => self.inbox_unread_count,
            LibraryView::Bookmark => self.bookmark_unread_count,
            LibraryView::Archived => self.archived_unread_count,
        }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum ItemType {
    Issue,
    PullRequest,
    Discussion,
}

impl ItemType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Issue => "Issue",
            Self::PullRequest => "Pull request",
            Self::Discussion => "Discussion",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ItemPerson {
    pub login: String,
    pub avatar_url: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ItemReview {
    pub login: String,
    pub avatar_url: Option<String>,
    pub state: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamItem {
    pub id: i64,
    pub repository_owner: String,
    pub repository_name: String,
    pub number: i64,
    pub item_type: ItemType,
    pub title: String,
    pub author_login: Option<String>,
    pub author_avatar_url: Option<String>,
    pub html_url: String,
    pub state: String,
    pub is_draft: Option<bool>,
    pub is_merged: Option<bool>,
    pub review_status: Option<String>,
    pub comment_count: i64,
    pub created_at_github: String,
    pub updated_at_github: String,
    pub closed_at_github: Option<String>,
    pub merged_at_github: Option<String>,
    pub read_at: Option<String>,
    pub labels: Vec<String>,
    pub assignees: Vec<ItemPerson>,
    pub review_requests: Vec<ItemPerson>,
    pub reviewers: Vec<ItemReview>,
    pub is_unread: bool,
    pub is_bookmarked: bool,
    pub is_archived: bool,
}

impl StreamItem {
    pub fn repository_full_name(&self) -> String {
        format!("{}/{}", self.repository_owner, self.repository_name)
    }
}

#[cfg(test)]
mod tests {
    use super::{HostConfig, HostKind, Scheme, StreamSource};

    #[test]
    fn github_search_url_uses_public_web_host() {
        let host = HostConfig::github_default();

        assert_eq!(
            host.search_url("is:pr review-requested:@me"),
            "https://github.com/search?q=is%3Apr%20review-requested%3A%40me&type=issues"
        );
    }

    #[test]
    fn ghes_search_url_uses_configured_host() {
        let host = HostConfig {
            name: "GHES".to_owned(),
            scheme: Scheme::Https,
            hostname: "ghe.example.test".to_owned(),
            rest_api_base_path: "/api/v3/".to_owned(),
            kind: HostKind::Ghes,
        };

        assert_eq!(
            host.search_url(" repo:acme/api is:issue "),
            "https://ghe.example.test/search?q=repo%3Aacme%2Fapi%20is%3Aissue&type=issues"
        );
    }

    #[test]
    fn discussion_preview_url_selects_discussion_search() {
        let host = HostConfig::github_default();

        assert_eq!(
            host.search_url_for(StreamSource::Discussion, "repo:acme/project feedback"),
            "https://github.com/search?q=repo%3Aacme%2Fproject%20feedback&type=discussions"
        );
    }
}
