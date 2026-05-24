#[path = "support/toolbar.rs"]
mod support;

use egui_kittest::kittest::Queryable as _;
use egui_kittest::Harness;
use gh_stream_listner::app::components;
use gh_stream_listner::app::screens::stream::StreamEvent;

use crate::support::{sample_toolbar_harness, ToolbarHarness};

#[test]
fn toolbar_buttons_emit_refresh_and_filter_events() {
    let mut harness = Harness::new_ui_state(
        |ui, state: &mut ToolbarHarness| {
            components::toolbar::show(ui, &mut state.stream, &state.config, &mut state.event);
        },
        sample_toolbar_harness(),
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
        sample_toolbar_harness(),
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
