use super::FetchedStreamItem;
use crate::storage::items::StreamItemUpsert;

pub(super) fn into_upsert(
    host_id: i64,
    item: FetchedStreamItem,
    graphql_enriched: bool,
) -> StreamItemUpsert {
    StreamItemUpsert {
        host_id,
        node_id: item.node_id,
        repository_owner: item.repository_owner,
        repository_name: item.repository_name,
        number: item.number,
        item_type: item.item_type,
        title: item.title,
        author_login: item.author_login,
        author_avatar_url: item.author_avatar_url,
        html_url: item.html_url,
        api_url: item.api_url,
        state: item.state,
        is_draft: item.is_draft,
        is_merged: item.is_merged,
        review_status: item.review_status,
        comment_count: item.comment_count,
        created_at_github: item.created_at_github,
        updated_at_github: item.updated_at_github,
        closed_at_github: item.closed_at_github,
        merged_at_github: item.merged_at_github,
        labels: item.labels,
        assignees: item.assignees,
        review_requests: item.review_requests,
        reviewers: item.reviewers,
        participants: item.participants,
        mentions: item.mentions,
        graphql_enriched,
    }
}
