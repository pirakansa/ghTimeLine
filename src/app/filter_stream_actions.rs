use crate::app::{AppMode, GhStreamApp};
use crate::models::{LibraryView, Selection};

impl GhStreamApp {
    pub(super) fn add_filter_stream(
        &mut self,
        saved_query_id: i64,
        name: &str,
        filter_query: &str,
        enabled: bool,
    ) {
        if name.trim().is_empty() || filter_query.trim().is_empty() {
            Self::replace_status(
                &mut self.status,
                &mut self.status_history,
                "Filter stream name and filter must not be empty.",
            );
            return;
        }

        if let AppMode::Main(runtime) = &mut self.mode {
            if let Err(err) = runtime
                .storage
                .validate_local_filter(Some(filter_query.trim()))
            {
                Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    format!("Could not create filter stream: {err}"),
                );
                return;
            }

            match runtime
                .storage
                .add_filter_stream(saved_query_id, name, filter_query, enabled)
            {
                Ok(id) => {
                    self.stream.selection = Selection::FilterStream(id);
                    self.stream.reset_item_list_scroll = true;
                    Self::replace_status(
                        &mut self.status,
                        &mut self.status_history,
                        "Filter stream created.",
                    );
                }
                Err(err) => Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    format!("Could not create filter stream: {err}"),
                ),
            }
        }
        self.reload_queries();
        self.reload_current_view();
    }

    pub(super) fn update_filter_stream(
        &mut self,
        id: i64,
        name: &str,
        filter_query: &str,
        enabled: bool,
    ) {
        if name.trim().is_empty() || filter_query.trim().is_empty() {
            Self::replace_status(
                &mut self.status,
                &mut self.status_history,
                "Filter stream name and filter must not be empty.",
            );
            return;
        }

        if let AppMode::Main(runtime) = &mut self.mode {
            if let Err(err) = runtime
                .storage
                .validate_local_filter(Some(filter_query.trim()))
            {
                Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    format!("Could not update filter stream: {err}"),
                );
                return;
            }

            match runtime
                .storage
                .update_filter_stream(id, name, filter_query, enabled)
            {
                Ok(()) => Self::replace_status(
                    &mut self.status,
                    &mut self.status_history,
                    "Filter stream updated.",
                ),
                Err(err) => Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    format!("Could not update filter stream: {err}"),
                ),
            }
        }
        self.reload_queries();
        self.reload_current_view();
    }

    pub(super) fn delete_filter_stream(&mut self, id: i64) {
        let parent_saved_query_id;
        if let AppMode::Main(runtime) = &mut self.mode {
            parent_saved_query_id = runtime.saved_queries.iter().find_map(|query| {
                query
                    .filter_streams
                    .iter()
                    .find(|stream| stream.id == id)
                    .map(|_| query.id)
            });

            match runtime.storage.delete_filter_stream(id) {
                Ok(()) => {
                    if self.stream.selection == Selection::FilterStream(id) {
                        self.stream.selection = parent_saved_query_id
                            .map(Selection::SavedQuery)
                            .unwrap_or(Selection::Library(LibraryView::Inbox));
                        self.stream.reset_item_list_scroll = true;
                    }
                    Self::replace_status(
                        &mut self.status,
                        &mut self.status_history,
                        "Filter stream deleted.",
                    );
                }
                Err(err) => Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    format!("Could not delete filter stream: {err}"),
                ),
            }
        }
        self.reload_queries();
        self.reload_current_view();
    }
}
