use egui_kittest::kittest::Queryable as _;
use egui_kittest::Harness;
use gh_stream_listner::app::components;
use gh_stream_listner::app::stream::{ItemAction, StreamEvent, StreamState};
use gh_stream_listner::models::{
    AppConfig, ItemType, LibraryCounts, SavedQuery, Selection, SortOrder, StreamItem,
};

#[test]
fn toolbar_buttons_emit_refresh_and_filter_events() {
    let mut harness = Harness::new_ui_state(
        |ui, state: &mut ToolbarHarness| {
            components::toolbar::show(ui, &mut state.stream, &state.config, &mut state.event);
        },
        ToolbarHarness {
            stream: StreamState::default(),
            config: AppConfig::default_with_pat("ghp_test".to_owned()),
            event: None,
        },
    );

    harness.get_by_label("Refresh").click();
    harness.run();
    assert!(matches!(
        harness.state().event,
        Some(StreamEvent::RefreshNow)
    ));

    harness.state_mut().event = None;
    harness.get_by_label("Unread").click();
    harness.run();
    assert!(matches!(
        harness.state().event,
        Some(StreamEvent::SetFilter(Some(
            gh_stream_listner::models::StreamFilter::Unread
        )))
    ));
}

#[test]
fn left_pane_saved_query_click_emits_selection_event() {
    let mut harness = Harness::new_state(
        |ctx, state: &mut LeftPaneHarness| {
            components::left_pane::show(
                ctx,
                &mut state.stream,
                &state.library_counts,
                &state.saved_queries,
                &mut state.event,
            );
        },
        LeftPaneHarness {
            stream: StreamState::default(),
            library_counts: LibraryCounts {
                inbox_unread_count: 5,
                bookmark_unread_count: 2,
                archived_unread_count: 1,
            },
            saved_queries: vec![SavedQuery {
                id: 7,
                name: "Reviews".to_owned(),
                query: "is:pr review-requested:@me".to_owned(),
                sort: SortOrder::UpdatedDesc,
                enabled: true,
                position: 0,
                unread_count: 3,
            }],
            event: None,
        },
    );

    harness.get_by_label("Inbox");
    harness.get_by_label("Bookmark");
    harness.get_by_label("Archived");
    harness.get_by_label("Reviews").click();
    harness.run();

    assert!(matches!(
        harness.state().event,
        Some(StreamEvent::Select(Selection::SavedQuery(7)))
    ));
}

#[test]
fn saved_query_manager_emits_enabled_toggle_event() {
    let saved_queries = vec![SavedQuery {
        id: 7,
        name: "Reviews".to_owned(),
        query: "is:pr review-requested:@me".to_owned(),
        sort: SortOrder::UpdatedDesc,
        enabled: true,
        position: 0,
        unread_count: 3,
    }];
    let mut stream = StreamState::default();
    components::left_pane::open_saved_query_manager(&mut stream, &saved_queries);

    let mut harness = Harness::new_state(
        |ctx, state: &mut StreamHarness| {
            components::left_pane::show_saved_query_manager(
                ctx,
                &mut state.stream,
                &state.saved_queries,
                &mut state.event,
            );
        },
        StreamHarness {
            stream,
            saved_queries,
            event: None,
        },
    );

    harness.get_by_label("Enabled").click();
    harness.run();

    match &harness.state().event {
        Some(StreamEvent::SetQueryEnabled { id, enabled }) => {
            assert_eq!((*id, *enabled), (7, false));
        }
        Some(_) => panic!("unexpected stream event"),
        None => panic!("expected enabled toggle event"),
    }
}

#[test]
fn item_list_action_buttons_emit_item_events() {
    let mut harness = Harness::new_ui_state(
        |ui, state: &mut ItemListHarness| {
            components::item_list::show(ui, &state.items, &mut state.event);
        },
        ItemListHarness {
            items: vec![sample_stream_item()],
            event: None,
        },
    );

    harness.get_by_label("Mark read").click();
    harness.run();
    assert!(matches!(
        harness.state().event,
        Some(StreamEvent::ItemAction(ItemAction::MarkRead(42)))
    ));

    harness.state_mut().event = None;
    harness.get_by_label("Bookmark").click();
    harness.run();
    assert!(matches!(
        harness.state().event,
        Some(StreamEvent::ItemAction(ItemAction::Bookmark(42, true)))
    ));

    harness.state_mut().event = None;
    harness.get_by_label("Archive").click();
    harness.run();
    assert!(matches!(
        harness.state().event,
        Some(StreamEvent::ItemAction(ItemAction::Archive(42, true)))
    ));

    harness = Harness::new_ui_state(
        |ui, state: &mut ItemListHarness| {
            components::item_list::show(ui, &state.items, &mut state.event);
        },
        ItemListHarness {
            items: vec![sample_archived_stream_item()],
            event: None,
        },
    );

    harness.get_by_label("Unarchive").click();
    harness.run();
    assert!(matches!(
        harness.state().event,
        Some(StreamEvent::ItemAction(ItemAction::Archive(42, false)))
    ));
}

struct ToolbarHarness {
    stream: StreamState,
    config: AppConfig,
    event: Option<StreamEvent>,
}

struct LeftPaneHarness {
    stream: StreamState,
    library_counts: LibraryCounts,
    saved_queries: Vec<SavedQuery>,
    event: Option<StreamEvent>,
}

struct StreamHarness {
    stream: StreamState,
    saved_queries: Vec<SavedQuery>,
    event: Option<StreamEvent>,
}

struct ItemListHarness {
    items: Vec<StreamItem>,
    event: Option<StreamEvent>,
}

fn sample_stream_item() -> StreamItem {
    StreamItem {
        id: 42,
        repository_owner: "owner".to_owned(),
        repository_name: "repo".to_owned(),
        number: 7,
        item_type: ItemType::PullRequest,
        title: "Improve stream".to_owned(),
        author_login: Some("octo".to_owned()),
        html_url: "https://github.example.test/owner/repo/pull/7".to_owned(),
        state: "open".to_owned(),
        is_draft: Some(false),
        is_merged: Some(false),
        review_status: Some("review_required".to_owned()),
        comment_count: 5,
        updated_at_github: "2026-05-23T00:00:00Z".to_owned(),
        labels: vec!["enhancement".to_owned()],
        assignees: vec!["dev".to_owned()],
        is_unread: true,
        is_bookmarked: false,
        is_archived: false,
    }
}

fn sample_archived_stream_item() -> StreamItem {
    StreamItem {
        is_archived: true,
        ..sample_stream_item()
    }
}
