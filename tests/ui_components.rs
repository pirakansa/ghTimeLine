use egui_kittest::kittest::Queryable as _;
use egui_kittest::Harness;
use gh_stream_listner::app::components;
use gh_stream_listner::app::screens::{
    saved_query_manager,
    stream::{ItemAction, StreamEvent, StreamState},
};
use gh_stream_listner::models::{
    AppConfig, ItemPerson, ItemReview, ItemType, LibraryCounts, SavedQuery, Selection, SortOrder,
    StreamItem,
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
fn preferences_menu_emits_open_setup_event() {
    let mut harness = Harness::new_state(
        |ctx, state: &mut ToolbarHarness| {
            components::menu_bar::show(ctx, &mut state.stream, &state.config, &mut state.event);
        },
        ToolbarHarness {
            stream: StreamState::default(),
            config: AppConfig::default_with_pat("ghp_test".to_owned()),
            event: None,
        },
    );

    harness.get_by_label("Preferences").click();
    harness.run();
    harness.get_by_label("Host settings").click();
    harness.run();

    assert!(matches!(
        harness.state().event,
        Some(StreamEvent::OpenSetup)
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
fn left_pane_hides_disabled_saved_queries() {
    let harness = Harness::new_state(
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
            library_counts: LibraryCounts::default(),
            saved_queries: vec![SavedQuery {
                id: 7,
                name: "Disabled reviews".to_owned(),
                query: "is:pr review-requested:@me".to_owned(),
                sort: SortOrder::UpdatedDesc,
                enabled: false,
                position: 0,
                unread_count: 3,
            }],
            event: None,
        },
    );

    harness.get_by_label("Saved queries");
    assert!(harness.query_by_label("Disabled reviews").is_none());
    assert!(harness
        .query_by_label("Disabled reviews (disabled)")
        .is_none());
}

#[test]
fn saved_query_manager_saves_enabled_state_with_changes() {
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
    saved_query_manager::open(&mut stream, &saved_queries);

    let mut harness = Harness::new_state(
        |ctx, state: &mut StreamHarness| {
            saved_query_manager::show(
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
    assert!(harness.state().event.is_none());

    harness.get_by_label("Save changes").click();
    harness.run();

    match &harness.state().event {
        Some(StreamEvent::UpdateQuery {
            id,
            name,
            query,
            sort,
            enabled,
        }) => {
            assert_eq!(*id, 7);
            assert_eq!(name, "Reviews");
            assert_eq!(query, "is:pr review-requested:@me");
            assert_eq!(*sort, SortOrder::UpdatedDesc);
            assert!(!enabled);
        }
        Some(_) => panic!("unexpected stream event"),
        None => panic!("expected query update event"),
    }
}

#[test]
fn item_list_action_buttons_emit_item_events() {
    let mut harness = Harness::new_ui_state(
        |ui, state: &mut ItemListHarness| {
            let mut avatar_cache = components::author_avatar::AvatarCache::default();
            components::item_list::show(ui, &state.items, &mut avatar_cache, &mut state.event);
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
            let mut avatar_cache = components::author_avatar::AvatarCache::default();
            components::item_list::show(ui, &state.items, &mut avatar_cache, &mut state.event);
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

#[test]
fn item_list_hides_user_names_when_avatars_are_present() {
    let harness = Harness::new_ui_state(
        |ui, state: &mut ItemListHarness| {
            let mut avatar_cache = components::author_avatar::AvatarCache::default();
            components::item_list::show(ui, &state.items, &mut avatar_cache, &mut state.event);
        },
        ItemListHarness {
            items: vec![sample_stream_item()],
            event: None,
        },
    );

    assert!(harness.query_by_label("octo").is_none());
    assert!(harness.query_by_label("dev").is_none());
    assert!(harness.query_by_label("triage").is_none());
    assert!(harness.query_by_label("reviewer").is_none());
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
        author_avatar_url: Some("https://avatars.githubusercontent.com/u/1?v=4".to_owned()),
        html_url: "https://github.example.test/owner/repo/pull/7".to_owned(),
        state: "open".to_owned(),
        is_draft: Some(false),
        is_merged: Some(false),
        review_status: Some("review_required".to_owned()),
        comment_count: 5,
        updated_at_github: "2026-05-23T00:00:00Z".to_owned(),
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

fn sample_archived_stream_item() -> StreamItem {
    StreamItem {
        is_archived: true,
        ..sample_stream_item()
    }
}
