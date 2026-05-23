use eframe::egui;

use super::components;
use crate::models::{
    AppConfig, FontSize, LibraryCounts, LibraryView, SavedQuery, Selection, SortOrder,
    StreamFilter, StreamItem, Theme,
};

pub struct StreamState {
    pub selection: Selection,
    pub filter: Option<StreamFilter>,
    pub(super) edit_query_id: Option<i64>,
    pub(super) edit_query_name: String,
    pub(super) edit_query_text: String,
    pub(super) edit_query_sort: SortOrder,
    pub(super) edit_query_enabled: bool,
    pub(super) saved_query_manager_open: bool,
    pub(super) polling_interval_draft: u32,
}

impl Default for StreamState {
    fn default() -> Self {
        Self {
            selection: Selection::Library(LibraryView::Inbox),
            filter: None,
            edit_query_id: None,
            edit_query_name: String::new(),
            edit_query_text: String::new(),
            edit_query_sort: SortOrder::UpdatedDesc,
            edit_query_enabled: true,
            saved_query_manager_open: false,
            polling_interval_draft: 0,
        }
    }
}

pub enum StreamEvent {
    Select(Selection),
    SetFilter(Option<StreamFilter>),
    AddQuery {
        name: String,
        query: String,
        enabled: bool,
    },
    UpdateQuery {
        id: i64,
        name: String,
        query: String,
        sort: SortOrder,
    },
    SetQueryEnabled {
        id: i64,
        enabled: bool,
    },
    DeleteQuery(i64),
    RefreshNow,
    SetDefaultSort(SortOrder),
    SetPollingInterval(u32),
    SetTheme(Theme),
    SetFontSize(FontSize),
    ItemAction(ItemAction),
}

pub enum ItemAction {
    MarkRead(i64),
    MarkUnread(i64),
    Bookmark(i64, bool),
    Archive(i64, bool),
    Open(String),
}

pub fn show(
    ctx: &egui::Context,
    state: &mut StreamState,
    config: &AppConfig,
    library_counts: &LibraryCounts,
    saved_queries: &[SavedQuery],
    items: &[StreamItem],
    status: &str,
) -> Option<StreamEvent> {
    if state.polling_interval_draft == 0 {
        state.polling_interval_draft = config.refresh.polling_interval_seconds;
    }
    let mut event = None;

    if state.saved_query_manager_open {
        components::saved_query_manager::show(ctx, state, saved_queries, &mut event);
        return event;
    }

    components::menu_bar::show(ctx, state, config, &mut event);
    components::left_pane::show(ctx, state, library_counts, saved_queries, &mut event);
    components::status_bar::show(ctx, config, status);

    egui::CentralPanel::default().show(ctx, |ui| {
        components::toolbar::show(ui, state, config, &mut event);
        ui.separator();
        components::item_list::show(ui, items, &mut event);
    });

    event
}
