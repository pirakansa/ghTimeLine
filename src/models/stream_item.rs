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
