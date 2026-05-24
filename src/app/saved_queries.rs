use crate::app::{AppMode, GhStreamApp};
use crate::models::{LibraryView, Selection, SortOrder};

impl GhStreamApp {
    pub(super) fn add_query(&mut self, name: &str, query: &str, enabled: bool) {
        if name.trim().is_empty() || query.trim().is_empty() {
            Self::replace_status(
                &mut self.status,
                &mut self.status_history,
                "Saved query name and query must not be empty.",
            );
            return;
        }

        if let AppMode::Main(runtime) = &mut self.mode {
            match runtime.storage.add_saved_query(
                runtime.host_id,
                name,
                query,
                runtime.config.ui.default_sort,
            ) {
                Ok(id) => {
                    if !enabled {
                        if let Err(err) = runtime.storage.set_saved_query_enabled(id, false) {
                            Self::replace_status(
                                &mut self.status,
                                &mut self.status_history,
                                format!("Could not disable saved query: {err}"),
                            );
                            return;
                        }
                    }
                    self.stream.selection = Selection::SavedQuery(id);
                    Self::replace_status(
                        &mut self.status,
                        &mut self.status_history,
                        "Saved query created.",
                    );
                }
                Err(err) => Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    format!("Could not create saved query: {err}"),
                ),
            }
        }
        self.reload_queries();
        self.reload_current_view();
    }

    pub(super) fn update_query(
        &mut self,
        id: i64,
        name: &str,
        query: &str,
        sort: SortOrder,
        enabled: bool,
    ) {
        if name.trim().is_empty() || query.trim().is_empty() {
            Self::replace_status(
                &mut self.status,
                &mut self.status_history,
                "Saved query name and query must not be empty.",
            );
            return;
        }

        if let AppMode::Main(runtime) = &mut self.mode {
            match runtime.storage.update_saved_query(id, name, query, sort) {
                Ok(()) => match runtime.storage.set_saved_query_enabled(id, enabled) {
                    Ok(()) => Self::replace_status(
                        &mut self.status,
                        &mut self.status_history,
                        "Saved query updated.",
                    ),
                    Err(err) => Self::replace_status_error(
                        &mut self.status,
                        &mut self.status_history,
                        format!("Could not update saved query: {err}"),
                    ),
                },
                Err(err) => Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    format!("Could not update saved query: {err}"),
                ),
            }
        }
        self.reload_queries();
        self.reload_current_view();
    }

    pub(super) fn delete_query(&mut self, id: i64) {
        if let AppMode::Main(runtime) = &mut self.mode {
            match runtime.storage.delete_saved_query(id) {
                Ok(()) => {
                    if self.stream.selection == Selection::SavedQuery(id) {
                        self.stream.selection = Selection::Library(LibraryView::Inbox);
                    }
                    Self::replace_status(
                        &mut self.status,
                        &mut self.status_history,
                        "Saved query deleted.",
                    );
                }
                Err(err) => Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    format!("Could not delete saved query: {err}"),
                ),
            }
        }
        self.reload_queries();
        self.reload_current_view();
    }

    pub(super) fn mark_saved_query_read(&mut self, id: i64) {
        let mut changed_item_ids = Vec::new();
        let mut did_update_items = false;
        if let AppMode::Main(runtime) = &mut self.mode {
            match runtime.storage.list_unread_item_ids_for_saved_query(id) {
                Ok(ids) => changed_item_ids = ids,
                Err(err) => {
                    Self::replace_status_error(
                        &mut self.status,
                        &mut self.status_history,
                        format!("Could not inspect saved query items: {err}"),
                    );
                    return;
                }
            }
            match runtime.storage.mark_saved_query_read(id) {
                Ok(0) => Self::replace_status(
                    &mut self.status,
                    &mut self.status_history,
                    "No unread items to mark read.",
                ),
                Ok(count) => {
                    Self::replace_status(
                        &mut self.status,
                        &mut self.status_history,
                        format!("Marked {count} items as read."),
                    );
                    did_update_items = true;
                }
                Err(err) => Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    format!("Could not mark saved query read: {err}"),
                ),
            }
        }
        if did_update_items {
            self.reload_queries();
            self.reload_current_view_for_changed_items(&changed_item_ids);
        }
    }
}
