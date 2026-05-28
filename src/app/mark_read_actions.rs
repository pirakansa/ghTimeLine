use crate::app::{AppMode, GhStreamApp};
use crate::models::LibraryView;

impl GhStreamApp {
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

    pub(super) fn mark_filter_stream_read(&mut self, id: i64) {
        let mut changed_item_ids = Vec::new();
        let mut did_update_items = false;
        if let AppMode::Main(runtime) = &mut self.mode {
            match runtime.storage.list_unread_item_ids_for_filter_stream(id) {
                Ok(ids) => changed_item_ids = ids,
                Err(err) => {
                    Self::replace_status_error(
                        &mut self.status,
                        &mut self.status_history,
                        format!("Could not inspect filter stream items: {err}"),
                    );
                    return;
                }
            }
            match runtime.storage.mark_filter_stream_read(id) {
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
                    format!("Could not mark filter stream read: {err}"),
                ),
            }
        }
        if did_update_items {
            self.reload_queries();
            self.reload_current_view_for_changed_items(&changed_item_ids);
        }
    }

    pub(super) fn mark_library_read(&mut self, library: LibraryView) {
        let mut changed_item_ids = Vec::new();
        let mut did_update_items = false;
        if let AppMode::Main(runtime) = &mut self.mode {
            match runtime
                .storage
                .list_unread_item_ids_for_library(runtime.host_id, library)
            {
                Ok(ids) => changed_item_ids = ids,
                Err(err) => {
                    Self::replace_status_error(
                        &mut self.status,
                        &mut self.status_history,
                        format!("Could not inspect library items: {err}"),
                    );
                    return;
                }
            }
            match runtime.storage.mark_library_read(runtime.host_id, library) {
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
                    format!("Could not mark library read: {err}"),
                ),
            }
        }
        if did_update_items {
            self.reload_queries();
            self.reload_current_view_for_changed_items(&changed_item_ids);
        }
    }
}
