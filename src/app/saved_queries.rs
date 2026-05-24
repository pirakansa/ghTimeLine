use crate::app::{AppMode, GhStreamApp};
use crate::models::{LibraryView, Selection, SortOrder};

impl GhStreamApp {
    pub(super) fn add_query(&mut self, name: &str, query: &str, enabled: bool) {
        if name.trim().is_empty() || query.trim().is_empty() {
            self.status = "Saved query name and query must not be empty.".to_owned();
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
                            self.status = format!("Could not disable saved query: {err}");
                            return;
                        }
                    }
                    self.stream.selection = Selection::SavedQuery(id);
                    self.status = "Saved query created.".to_owned();
                }
                Err(err) => self.status = format!("Could not create saved query: {err}"),
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
            self.status = "Saved query name and query must not be empty.".to_owned();
            return;
        }

        if let AppMode::Main(runtime) = &mut self.mode {
            match runtime.storage.update_saved_query(id, name, query, sort) {
                Ok(()) => match runtime.storage.set_saved_query_enabled(id, enabled) {
                    Ok(()) => self.status = "Saved query updated.".to_owned(),
                    Err(err) => self.status = format!("Could not update saved query: {err}"),
                },
                Err(err) => self.status = format!("Could not update saved query: {err}"),
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
                    self.status = "Saved query deleted.".to_owned();
                }
                Err(err) => self.status = format!("Could not delete saved query: {err}"),
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
                    self.status = format!("Could not inspect saved query items: {err}");
                    return;
                }
            }
            match runtime.storage.mark_saved_query_read(id) {
                Ok(0) => self.status = "No unread items to mark read.".to_owned(),
                Ok(count) => {
                    self.status = format!("Marked {count} items as read.");
                    did_update_items = true;
                }
                Err(err) => self.status = format!("Could not mark saved query read: {err}"),
            }
        }
        if did_update_items {
            self.reload_queries();
            self.reload_current_view_for_changed_items(&changed_item_ids);
        }
    }
}
