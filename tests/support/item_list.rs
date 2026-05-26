use ghtl::app::screens::stream::StreamEvent;
use ghtl::models::{ItemPerson, ItemReview, ItemType, StreamItem};

pub struct ItemListHarness {
    pub items: Vec<StreamItem>,
    pub list_state: ghtl::app::components::item_list::ItemListState,
    pub reset_scroll_to_top: bool,
    pub event: Option<StreamEvent>,
}

pub fn sample_stream_item() -> StreamItem {
    StreamItem {
        id: 42,
        repository_owner: "owner".to_owned(),
        repository_name: "repo".to_owned(),
        number: 7,
        item_type: ItemType::PullRequest,
        title: "Improve stream".to_owned(),
        author_login: Some("octo".to_owned()),
        author_avatar_url: Some("https://avatars.githubusercontent.com/u/1?v=4".to_owned()),
        html_url: "https://github.example.test/owner/repo/pull/7".to_owned(),
        state: "open".to_owned(),
        is_draft: Some(false),
        is_merged: Some(false),
        review_status: Some("review_required".to_owned()),
        comment_count: 5,
        created_at_github: "2026-05-22T00:00:00Z".to_owned(),
        updated_at_github: "2026-05-23T00:00:00Z".to_owned(),
        closed_at_github: None,
        merged_at_github: None,
        read_at: None,
        labels: vec!["enhancement".to_owned()],
        assignees: vec![ItemPerson {
            login: "dev".to_owned(),
            avatar_url: Some("https://avatars.githubusercontent.com/u/2?v=4".to_owned()),
        }],
        review_requests: vec![ItemPerson {
            login: "triage".to_owned(),
            avatar_url: Some("https://avatars.githubusercontent.com/u/3?v=4".to_owned()),
        }],
        reviewers: vec![ItemReview {
            login: "reviewer".to_owned(),
            avatar_url: Some("https://avatars.githubusercontent.com/u/4?v=4".to_owned()),
            state: "approved".to_owned(),
        }],
        is_unread: true,
        is_bookmarked: false,
        is_archived: false,
    }
}

pub fn sample_archived_stream_item() -> StreamItem {
    StreamItem {
        is_archived: true,
        ..sample_stream_item()
    }
}
