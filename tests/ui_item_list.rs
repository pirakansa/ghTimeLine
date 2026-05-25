#[path = "support/item_list.rs"]
mod support;

use egui_kittest::kittest::Queryable as _;
use egui_kittest::Harness;
use gh_stream_listner::app::components;
use gh_stream_listner::app::screens::stream::{ItemAction, StreamEvent};

use crate::support::{sample_archived_stream_item, sample_stream_item, ItemListHarness};

#[test]
fn item_list_action_buttons_emit_item_events() {
    let mut harness = Harness::new_ui_state(
        |ui, state: &mut ItemListHarness| {
            let mut avatar_cache = components::author_avatar::AvatarCache::default();
            components::item_list::show(
                ui,
                &state.items,
                &mut avatar_cache,
                &mut state.reset_scroll_to_top,
                &mut state.event,
            );
        },
        ItemListHarness {
            items: vec![sample_stream_item()],
            reset_scroll_to_top: false,
            event: None,
        },
    );

    // Hover over the item card to reveal the action overlay
    harness.get_by_label("Improve stream").hover();
    harness.run();

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
            components::item_list::show(
                ui,
                &state.items,
                &mut avatar_cache,
                &mut state.reset_scroll_to_top,
                &mut state.event,
            );
        },
        ItemListHarness {
            items: vec![sample_archived_stream_item()],
            reset_scroll_to_top: false,
            event: None,
        },
    );

    // Hover over the item card to reveal the action overlay
    harness.get_by_label("Improve stream").hover();
    harness.run();

    harness.get_by_label("Unarchive").click();
    harness.run();
    assert!(matches!(
        harness.state().event,
        Some(StreamEvent::ItemAction(ItemAction::Archive(42, false)))
    ));
}

#[test]
fn item_list_item_click_emits_open_event() {
    let mut harness = Harness::new_ui_state(
        |ui, state: &mut ItemListHarness| {
            let mut avatar_cache = components::author_avatar::AvatarCache::default();
            components::item_list::show(
                ui,
                &state.items,
                &mut avatar_cache,
                &mut state.reset_scroll_to_top,
                &mut state.event,
            );
        },
        ItemListHarness {
            items: vec![sample_stream_item()],
            reset_scroll_to_top: false,
            event: None,
        },
    );

    harness.get_by_label("Improve stream").click();
    harness.run();
    assert!(matches!(
        &harness.state().event,
        Some(StreamEvent::ItemAction(ItemAction::Open { id: 42, url }))
            if url == "https://github.example.test/owner/repo/pull/7"
    ));
    assert!(harness.query_by_label("Open").is_none());
}

#[test]
fn item_list_hides_user_names_when_avatars_are_present() {
    let harness = Harness::new_ui_state(
        |ui, state: &mut ItemListHarness| {
            let mut avatar_cache = components::author_avatar::AvatarCache::default();
            components::item_list::show(
                ui,
                &state.items,
                &mut avatar_cache,
                &mut state.reset_scroll_to_top,
                &mut state.event,
            );
        },
        ItemListHarness {
            items: vec![sample_stream_item()],
            reset_scroll_to_top: false,
            event: None,
        },
    );

    assert!(harness.query_by_label("octo").is_none());
    assert!(harness.query_by_label("dev").is_none());
    assert!(harness.query_by_label("triage").is_none());
    assert!(harness.query_by_label("reviewer").is_none());
}

#[test]
fn item_list_shows_labels_as_badges_without_heading() {
    let harness = Harness::new_ui_state(
        |ui, state: &mut ItemListHarness| {
            let mut avatar_cache = components::author_avatar::AvatarCache::default();
            components::item_list::show(
                ui,
                &state.items,
                &mut avatar_cache,
                &mut state.reset_scroll_to_top,
                &mut state.event,
            );
        },
        ItemListHarness {
            items: vec![sample_stream_item()],
            reset_scroll_to_top: false,
            event: None,
        },
    );

    harness.get_by_label("enhancement");
    assert!(harness.query_by_label("Labels: enhancement").is_none());
    assert!(harness.query_by_label("Labels:").is_none());
}

#[test]
fn item_list_keeps_comment_count_and_reviewer_row_visible() {
    let harness = Harness::new_ui_state(
        |ui, state: &mut ItemListHarness| {
            let mut avatar_cache = components::author_avatar::AvatarCache::default();
            components::item_list::show(
                ui,
                &state.items,
                &mut avatar_cache,
                &mut state.reset_scroll_to_top,
                &mut state.event,
            );
        },
        ItemListHarness {
            items: vec![sample_stream_item()],
            reset_scroll_to_top: false,
            event: None,
        },
    );

    harness.get_by_label("5");
    harness.get_by_label("←");
}

#[test]
fn item_list_shows_requested_reviewer_row_without_completed_reviews() {
    let mut item = sample_stream_item();
    item.reviewers.clear();

    let harness = Harness::new_ui_state(
        |ui, state: &mut ItemListHarness| {
            let mut avatar_cache = components::author_avatar::AvatarCache::default();
            components::item_list::show(
                ui,
                &state.items,
                &mut avatar_cache,
                &mut state.reset_scroll_to_top,
                &mut state.event,
            );
        },
        ItemListHarness {
            items: vec![item],
            reset_scroll_to_top: false,
            event: None,
        },
    );

    harness.get_by_label("←");
}

#[test]
fn item_list_scroll_reset_returns_virtualized_list_to_first_item() {
    let items = (0..20)
        .map(|index| {
            let mut item = sample_stream_item();
            item.id = index;
            item.title = format!("Item {index}");
            item
        })
        .collect();
    let mut harness = Harness::new_ui_state(
        |ui, state: &mut ItemListHarness| {
            let mut avatar_cache = components::author_avatar::AvatarCache::default();
            components::item_list::show(
                ui,
                &state.items,
                &mut avatar_cache,
                &mut state.reset_scroll_to_top,
                &mut state.event,
            );
        },
        ItemListHarness {
            items,
            reset_scroll_to_top: false,
            event: None,
        },
    );

    for _ in 0..3 {
        harness.get_by_label("Item 0").scroll_down();
    }
    harness.run();
    assert!(harness.query_by_label("Item 0").is_none());

    harness.state_mut().reset_scroll_to_top = true;
    harness.run();

    harness.get_by_label("Item 0");
    assert!(!harness.state().reset_scroll_to_top);
}
