#[path = "support/left_pane.rs"]
mod support;

use egui_kittest::kittest::Queryable as _;
use egui_kittest::Harness;
use ghtl::app::components;
use ghtl::app::screens::{
    saved_query_manager,
    stream::{StreamEvent, StreamState},
};
use ghtl::models::{LibraryCounts, LibraryView, SavedQuery, Selection};

use crate::support::{sample_saved_query, LeftPaneHarness, StreamHarness};

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
            saved_queries: vec![sample_saved_query()],
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
                enabled: false,
                name: "Disabled reviews".to_owned(),
                ..sample_saved_query()
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
fn saved_query_context_menu_marks_query_read() {
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
            library_counts: LibraryCounts::default(),
            saved_queries: vec![sample_saved_query()],
            event: None,
        },
    );

    harness.get_by_label("Reviews").click_secondary();
    harness.run();
    harness.get_by_label("Mark all as read").click();
    harness.run();

    assert!(matches!(
        harness.state().event,
        Some(StreamEvent::MarkSavedQueryRead(7))
    ));
}

#[test]
fn library_context_menu_marks_library_read() {
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
            saved_queries: vec![sample_saved_query()],
            event: None,
        },
    );

    harness.get_by_label("Archived").click_secondary();
    harness.run();
    harness.get_by_label("Mark all as read").click();
    harness.run();

    assert!(matches!(
        harness.state().event,
        Some(StreamEvent::MarkLibraryRead(LibraryView::Archived))
    ));
}

#[test]
fn saved_query_manager_saves_enabled_state_with_changes() {
    let saved_queries = vec![sample_saved_query()];
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
    assert!(harness.query_by_label("Display sort").is_none());

    harness.get_by_label("Save changes").click();
    harness.run();

    match &harness.state().event {
        Some(StreamEvent::UpdateQuery {
            id,
            name,
            query,
            enabled,
        }) => {
            assert_eq!(*id, 7);
            assert_eq!(name, "Reviews");
            assert_eq!(query, "is:pr review-requested:@me");
            assert!(!enabled);
        }
        Some(_) => panic!("unexpected stream event"),
        None => panic!("expected query update event"),
    }
}

#[test]
fn saved_query_manager_new_button_is_next_to_queries_heading() {
    let saved_queries = vec![sample_saved_query()];
    let mut stream = StreamState::default();
    stream.selection = Selection::SavedQuery(7);
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

    harness.get_by_label("Queries");
    assert!(harness.query_by_label("New").is_none());

    harness.get_by_label("+").click();
    harness.run();

    harness.get_by_label("New query");
    assert!(harness.state().event.is_none());
}

#[test]
fn saved_query_manager_move_down_button_emits_reorder_event() {
    let saved_queries = vec![
        sample_saved_query(),
        SavedQuery {
            id: 8,
            name: "Inbox".to_owned(),
            query: "is:open".to_owned(),
            enabled: true,
            position: 1,
            unread_count: 1,
        },
    ];
    let mut stream = StreamState::default();
    stream.selection = Selection::SavedQuery(7);
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

    harness.get_by_label("▼").click();
    harness.run();

    assert!(matches!(
        harness.state().event,
        Some(StreamEvent::MoveQueryDown(7))
    ));
}

#[test]
fn saved_query_manager_import_export_buttons_emit_events_with_path() {
    let saved_queries = vec![sample_saved_query()];
    let mut stream = StreamState::default();
    stream.selection = Selection::SavedQuery(7);
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

    harness.get_by_label("YAML file");
    harness.get_by_label("Export").click();
    harness.run();

    assert!(matches!(
        &harness.state().event,
        Some(StreamEvent::ExportQueries(actual)) if !actual.is_empty()
    ));

    harness.state_mut().event = None;
    harness.get_by_label("Import").click();
    harness.run();

    assert!(matches!(
        &harness.state().event,
        Some(StreamEvent::ImportQueries(actual)) if !actual.is_empty()
    ));
}
