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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
    System,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    UpdatedDesc,
    UpdatedAsc,
    CreatedDesc,
    CreatedAsc,
    CommentsDesc,
    CommentsAsc,
}

impl SortOrder {
    pub const ALL: [Self; 6] = [
        Self::UpdatedDesc,
        Self::UpdatedAsc,
        Self::CreatedDesc,
        Self::CreatedAsc,
        Self::CommentsDesc,
        Self::CommentsAsc,
    ];

    pub fn as_db_value(self) -> &'static str {
        match self {
            Self::UpdatedDesc => "updated_desc",
            Self::UpdatedAsc => "updated_asc",
            Self::CreatedDesc => "created_desc",
            Self::CreatedAsc => "created_asc",
            Self::CommentsDesc => "comments_desc",
            Self::CommentsAsc => "comments_asc",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::UpdatedDesc => "Updated desc",
            Self::UpdatedAsc => "Updated asc",
            Self::CreatedDesc => "Created desc",
            Self::CreatedAsc => "Created asc",
            Self::CommentsDesc => "Comments desc",
            Self::CommentsAsc => "Comments asc",
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
            "updated_asc" => Ok(Self::UpdatedAsc),
            "created_desc" => Ok(Self::CreatedDesc),
            "created_asc" => Ok(Self::CreatedAsc),
            "comments_desc" => Ok(Self::CommentsDesc),
            "comments_asc" => Ok(Self::CommentsAsc),
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
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthConfig {
    pub pat: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: Theme,
    pub accent_color: String,
    pub default_sort: SortOrder,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefreshConfig {
    pub polling_interval_minutes: u16,
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
            },
            refresh: RefreshConfig {
                polling_interval_minutes: 5,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedQuery {
    pub id: i64,
    pub name: String,
    pub query: String,
    pub sort: SortOrder,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemType {
    Issue,
    PullRequest,
}

impl ItemType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Issue => "Issue",
            Self::PullRequest => "Pull request",
        }
    }
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
    pub html_url: String,
    pub state: String,
    pub is_draft: Option<bool>,
    pub is_merged: Option<bool>,
    pub review_status: Option<String>,
    pub comment_count: i64,
    pub updated_at_github: String,
    pub labels: Vec<String>,
    pub assignees: Vec<String>,
    pub is_unread: bool,
    pub is_bookmarked: bool,
    pub is_archived: bool,
}

impl StreamItem {
    pub fn repository_full_name(&self) -> String {
        format!("{}/{}", self.repository_owner, self.repository_name)
    }
}
