use eframe::egui;

use super::components;
use crate::models::{
    AppConfig, LibraryView, SavedQuery, Selection, SortOrder, StreamFilter, StreamItem,
};

pub struct StreamState {
    pub selection: Selection,
    pub filter: Option<StreamFilter>,
    pub(super) new_query_name: String,
    pub(super) new_query_text: String,
    pub(super) polling_interval_draft: u16,
}

impl Default for StreamState {
    fn default() -> Self {
        Self {
            selection: Selection::Library(LibraryView::Inbox),
            filter: None,
            new_query_name: String::new(),
            new_query_text: String::new(),
            polling_interval_draft: 5,
        }
    }
}

pub enum StreamEvent {
    Select(Selection),
    SetFilter(Option<StreamFilter>),
    AddQuery { name: String, query: String },
    DeleteSelectedQuery,
    RefreshNow,
    SetDefaultSort(SortOrder),
    SetPollingInterval(u16),
    ItemAction(ItemAction),
}

pub enum ItemAction {
    MarkRead(i64),
    MarkUnread(i64),
    Bookmark(i64, bool),
    Archive(i64),
    Open(String),
}

pub fn show(
    ctx: &egui::Context,
    state: &mut StreamState,
    config: &AppConfig,
    saved_queries: &[SavedQuery],
    items: &[StreamItem],
    status: &str,
) -> Option<StreamEvent> {
    state.polling_interval_draft = config.refresh.polling_interval_minutes;
    let mut event = None;

    components::left_pane::show(ctx, state, saved_queries, &mut event);
    components::status_bar::show(ctx, config, status);

    egui::CentralPanel::default().show(ctx, |ui| {
        components::toolbar::show(ui, state, config, &mut event);
        ui.separator();
        components::item_list::show(ui, items, &mut event);
    });

    event
}
