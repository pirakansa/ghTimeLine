use crate::app::{AppMode, GhStreamApp};
use crate::github;
use crate::models::{LibraryView, Selection, StreamSource};

impl GhStreamApp {
    pub(super) fn preview_query(&mut self, query: &str, source: StreamSource) {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            Self::replace_status(
                &mut self.status,
                &mut self.status_history,
                "Saved query must not be empty.",
            );
            return;
        }

        if let AppMode::Main(runtime) = &mut self.mode {
            let preview_url = match source {
                StreamSource::ProjectV2 => {
                    match github::project::project_preview_url(&runtime.config.host, trimmed) {
                        Ok(url) => url,
                        Err(err) => {
                            Self::replace_status_error(
                                &mut self.status,
                                &mut self.status_history,
                                format!("Could not preview project: {err}"),
                            );
                            return;
                        }
                    }
                }
                _ => runtime.config.host.search_url_for(source, trimmed),
            };
            match open::that(preview_url) {
                Ok(()) => Self::replace_status(
                    &mut self.status,
                    &mut self.status_history,
                    "Opened query preview in external browser.",
                ),
                Err(err) => Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    format!("Could not open browser: {err}"),
                ),
            }
        }
    }

    pub(super) fn add_query(
        &mut self,
        name: &str,
        query: &str,
        source: StreamSource,
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
            match runtime.storage.add_saved_query_configured(
                runtime.host_id,
                name,
                query,
                source,
                enabled,
            ) {
                Ok(id) => {
                    self.stream.selection = Selection::SavedQuery(id);
                    self.stream.reset_item_list_scroll = true;
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
        source: StreamSource,
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
            match runtime
                .storage
                .update_saved_query_configured(id, name, query, source, enabled)
            {
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
            }
        }
        self.reload_queries();
        self.reload_current_view();
    }

    pub(super) fn delete_query(&mut self, id: i64) {
        if let AppMode::Main(runtime) = &mut self.mode {
            match runtime.storage.delete_saved_query(id) {
                Ok(()) => {
                    if self.stream.selection == Selection::SavedQuery(id)
                        || matches!(
                            self.stream.selection,
                            Selection::FilterStream(filter_stream_id)
                                if runtime
                                    .saved_queries
                                    .iter()
                                    .find(|query| query.id == id)
                                    .is_some_and(|query| query
                                        .filter_streams
                                        .iter()
                                        .any(|stream| stream.id == filter_stream_id))
                        )
                    {
                        self.stream.selection = Selection::Library(LibraryView::Inbox);
                        self.stream.reset_item_list_scroll = true;
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

    pub(super) fn move_query_up(&mut self, id: i64) {
        if let AppMode::Main(runtime) = &mut self.mode {
            match runtime.storage.move_saved_query_up(id) {
                Ok(true) => Self::replace_status(
                    &mut self.status,
                    &mut self.status_history,
                    "Saved query moved up.",
                ),
                Ok(false) => Self::replace_status(
                    &mut self.status,
                    &mut self.status_history,
                    "Saved query is already at the top.",
                ),
                Err(err) => Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    format!("Could not move saved query: {err}"),
                ),
            }
        }
        self.reload_queries();
    }

    pub(super) fn move_query_down(&mut self, id: i64) {
        if let AppMode::Main(runtime) = &mut self.mode {
            match runtime.storage.move_saved_query_down(id) {
                Ok(true) => Self::replace_status(
                    &mut self.status,
                    &mut self.status_history,
                    "Saved query moved down.",
                ),
                Ok(false) => Self::replace_status(
                    &mut self.status,
                    &mut self.status_history,
                    "Saved query is already at the bottom.",
                ),
                Err(err) => Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    format!("Could not move saved query: {err}"),
                ),
            }
        }
        self.reload_queries();
    }
}
