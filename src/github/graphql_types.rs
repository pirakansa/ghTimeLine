#[derive(serde::Serialize)]
pub(super) struct GraphqlRequest<'a> {
    pub(super) query: &'static str,
    pub(super) variables: GraphqlVariables<'a>,
}

#[derive(serde::Serialize)]
pub(super) struct GraphqlVariables<'a> {
    pub(super) ids: &'a [String],
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct GraphqlResponse {
    pub(super) data: Option<GraphqlData>,
    pub(super) errors: Option<Vec<GraphqlError>>,
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct GraphqlError {
    pub(super) message: String,
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct GraphqlData {
    pub(super) nodes: Vec<Option<EnrichedNode>>,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct EnrichedNode {
    pub(super) id: String,
    #[serde(default)]
    pub(super) body: String,
    #[serde(default)]
    pub(super) is_draft: Option<bool>,
    #[serde(default)]
    pub(super) merged: Option<bool>,
    pub(super) merged_at: Option<String>,
    pub(super) review_decision: Option<String>,
    #[serde(default)]
    pub(super) review_requests: ReviewRequests,
    #[serde(default)]
    pub(super) latest_reviews: LatestReviews,
    #[serde(default)]
    pub(super) participants: Participants,
    #[serde(default)]
    pub(super) comments: Comments,
}

impl EnrichedNode {
    pub(super) fn review_status_fields_present(&self) -> bool {
        self.is_draft.is_some() || self.merged.is_some() || self.review_decision.is_some()
    }
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ReviewRequests {
    #[serde(default)]
    pub(super) total_count: i64,
    #[serde(default)]
    pub(super) nodes: Vec<ReviewRequestNode>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub(super) struct LatestReviews {
    #[serde(default)]
    pub(super) nodes: Vec<ReviewNode>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub(super) struct Participants {
    #[serde(default)]
    pub(super) nodes: Vec<UserRef>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub(super) struct Comments {
    #[serde(default)]
    pub(super) nodes: Vec<CommentNode>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ReviewRequestNode {
    pub(super) requested_reviewer: Option<UserRef>,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct UserRef {
    pub(super) login: Option<String>,
    pub(super) avatar_url: Option<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ReviewNode {
    pub(super) state: String,
    #[serde(default)]
    pub(super) body: String,
    pub(super) author: Option<UserRef>,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CommentNode {
    pub(super) author: Option<UserRef>,
    #[serde(default)]
    pub(super) body: String,
}
