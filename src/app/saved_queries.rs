use std::path::Path;

use crate::app::{AppMode, GhStreamApp};
use crate::models::{LibraryView, Selection};
use crate::saved_query_io;

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
            match runtime
                .storage
                .add_saved_query(runtime.host_id, name, query)
            {
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

    pub(super) fn update_query(&mut self, id: i64, name: &str, query: &str, enabled: bool) {
        if name.trim().is_empty() || query.trim().is_empty() {
            Self::replace_status(
                &mut self.status,
                &mut self.status_history,
                "Saved query name and query must not be empty.",
            );
            return;
        }

        if let AppMode::Main(runtime) = &mut self.mode {
            match runtime.storage.update_saved_query(id, name, query) {
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

    pub(super) fn export_queries(&mut self, path: &str) {
        let path = path.trim();
        if path.is_empty() {
            Self::replace_status(
                &mut self.status,
                &mut self.status_history,
                "Saved query export path must not be empty.",
            );
            return;
        }

        if let AppMode::Main(runtime) = &mut self.mode {
            match saved_query_io::write_saved_queries(
                Path::new(path),
                &runtime.config.host,
                &runtime.saved_queries,
            ) {
                Ok(()) => Self::replace_status(
                    &mut self.status,
                    &mut self.status_history,
                    format!("Saved queries exported to {path}."),
                ),
                Err(err) => Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    format!("Could not export saved queries: {err}"),
                ),
            }
        }
    }

    pub(super) fn import_queries(&mut self, path: &str) {
        let path = path.trim();
        if path.is_empty() {
            Self::replace_status(
                &mut self.status,
                &mut self.status_history,
                "Saved query import path must not be empty.",
            );
            return;
        }

        if let AppMode::Main(runtime) = &mut self.mode {
            let imported = match saved_query_io::read_saved_queries(Path::new(path)) {
                Ok(imported) => imported,
                Err(err) => {
                    Self::replace_status_error(
                        &mut self.status,
                        &mut self.status_history,
                        format!("Could not import saved queries: {err}"),
                    );
                    return;
                }
            };

            if imported.host.fingerprint() != runtime.config.host.fingerprint() {
                Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    "Could not import saved queries: saved query file host does not match the current host.",
                );
                return;
            }

            let imported_names = imported
                .queries
                .iter()
                .map(|query| query.name.clone())
                .collect::<Vec<_>>();
            match runtime
                .storage
                .replace_saved_queries(runtime.host_id, &imported.queries)
            {
                Ok(inserted_ids) => {
                    self.reload_queries();
                    if let Some(first_id) = inserted_ids.first().copied() {
                        self.stream.selection = Selection::SavedQuery(first_id);
                        self.stream.reset_item_list_scroll = true;
                    } else {
                        self.stream.selection = Selection::Library(LibraryView::Inbox);
                        self.stream.reset_item_list_scroll = true;
                    }
                    if let AppMode::Main(runtime) = &self.mode {
                        crate::app::screens::saved_query_manager::open(
                            &mut self.stream,
                            &runtime.saved_queries,
                        );
                    }
                    self.reload_current_view();
                    let count = imported_names.len();
                    Self::replace_status(
                        &mut self.status,
                        &mut self.status_history,
                        format!(
                            "Imported {count} saved quer{} from {path}. Refresh to rebuild matches.",
                            if count == 1 { "y" } else { "ies" }
                        ),
                    );
                }
                Err(err) => Self::replace_status_error(
                    &mut self.status,
                    &mut self.status_history,
                    format!("Could not import saved queries: {err}"),
                ),
            }
        }
    }
}
