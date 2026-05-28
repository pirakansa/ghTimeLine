use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::StreamSource;

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
                StreamSource::ProjectV2 => "issues",
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

#[cfg(test)]
mod tests {
    use super::{HostConfig, HostKind, Scheme};
    use crate::models::StreamSource;

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
