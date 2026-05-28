use std::path::Path;

use crate::app::{AppMode, GhStreamApp};
use crate::models::{LibraryView, Selection};
use crate::saved_query_io;

impl GhStreamApp {
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
