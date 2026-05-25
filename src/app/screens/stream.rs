use eframe::egui;

use super::saved_query_manager;
use super::status_log;
use crate::app::components;
use crate::app::screens::saved_query_manager::SavedQueryManagerState;
use crate::app::screens::status_log::StatusLogState;
use crate::app::StatusEntry;
use crate::models::{
    AppConfig, FontSize, LibraryCounts, LibraryView, SavedQuery, Selection, SortOrder,
    StreamFilter, StreamItem, Theme,
};

pub struct StreamState {
    pub selection: Selection,
    pub filter: Option<StreamFilter>,
    pub(in crate::app) reset_item_list_scroll: bool,
    pub(in crate::app) polling_interval_draft: u32,
    pub(in crate::app) saved_query_manager: SavedQueryManagerState,
    pub(in crate::app) status_log: StatusLogState,
    pub(in crate::app) avatar_cache: components::author_avatar::AvatarCache,
    pub(in crate::app) item_list: components::item_list::ItemListState,
}

impl Default for StreamState {
    fn default() -> Self {
        Self {
            selection: Selection::Library(LibraryView::Inbox),
            filter: None,
            reset_item_list_scroll: false,
            polling_interval_draft: 0,
            saved_query_manager: SavedQueryManagerState::default(),
            status_log: StatusLogState::default(),
            avatar_cache: components::author_avatar::AvatarCache::default(),
            item_list: components::item_list::ItemListState::default(),
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
        enabled: bool,
    },
    DeleteQuery(i64),
    MoveQueryUp(i64),
    MoveQueryDown(i64),
    MarkSavedQueryRead(i64),
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
    Open { id: i64, url: String },
}

pub(in crate::app) fn show(
    ctx: &egui::Context,
    state: &mut StreamState,
    config: &AppConfig,
    library_counts: &LibraryCounts,
    saved_queries: &[SavedQuery],
    items: &[StreamItem],
    status_history: &[StatusEntry],
) -> Option<StreamEvent> {
    if state.polling_interval_draft == 0 {
        state.polling_interval_draft = config.refresh.polling_interval_seconds;
    }
    let mut event = None;

    if state.saved_query_manager.open {
        saved_query_manager::show(ctx, state, saved_queries, &mut event);
        return event;
    }

    if state.status_log.open {
        status_log::show(ctx, state, status_history);
        return event;
    }

    components::menu_bar::show(ctx, state, config, &mut event);
    components::status_bar::show(ctx, state, status_history);
    components::left_pane::show(ctx, state, library_counts, saved_queries, &mut event);

    egui::CentralPanel::default().show(ctx, |ui| {
        components::toolbar::show(ui, state, config, &mut event);
        ui.separator();
        components::item_list::show(
            ui,
            items,
            &mut state.avatar_cache,
            &mut state.item_list,
            &mut state.reset_item_list_scroll,
            &mut event,
        );
    });

    event
}
