use eframe::egui;

use super::saved_query_manager;
use crate::app::components;
use crate::app::screens::saved_query_manager::SavedQueryManagerState;
use crate::models::{
    AppConfig, FontSize, LibraryCounts, LibraryView, SavedQuery, Selection, SortOrder,
    StreamFilter, StreamItem, Theme,
};

pub struct StreamState {
    pub selection: Selection,
    pub filter: Option<StreamFilter>,
    pub(in crate::app) polling_interval_draft: u32,
    pub(in crate::app) saved_query_manager: SavedQueryManagerState,
}

impl Default for StreamState {
    fn default() -> Self {
        Self {
            selection: Selection::Library(LibraryView::Inbox),
            filter: None,
            polling_interval_draft: 0,
            saved_query_manager: SavedQueryManagerState::default(),
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
    OpenSetup,
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

    if state.saved_query_manager.open {
        saved_query_manager::show(ctx, state, saved_queries, &mut event);
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
