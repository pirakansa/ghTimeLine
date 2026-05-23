use crate::app::{AppMode, GhStreamApp, Runtime};
use crate::models::{LibraryCounts, SavedQuery, Selection, SortOrder, StreamFilter};
use crate::storage::Storage;

impl GhStreamApp {
    pub(super) fn reload_queries(&mut self) {
        if let AppMode::Main(runtime) = &mut self.mode {
            match load_sidebar_data(&runtime.storage, runtime.host_id) {
                Ok((library_counts, saved_queries)) => {
                    runtime.library_counts = library_counts;
                    runtime.saved_queries = saved_queries;
                }
                Err(err) => self.status = format!("Could not load saved queries: {err}"),
            }
        }
    }

    pub(super) fn reload_current_view(&mut self) {
        if let AppMode::Main(runtime) = &mut self.mode {
            let sort = current_sort(runtime, &self.stream.selection);
            let result = match self.stream.selection.clone() {
                Selection::Library(library) => runtime.storage.list_items_for_library(
                    runtime.host_id,
                    library,
                    self.stream.filter,
                    sort,
                ),
                Selection::SavedQuery(id) => {
                    runtime
                        .storage
                        .list_items_for_saved_query(id, self.stream.filter, sort)
                }
            };

            match result {
                Ok(items) => runtime.items = items,
                Err(err) => self.status = format!("Could not load stream items: {err}"),
            }
        }
    }

    pub(super) fn set_filter(&mut self, filter: Option<StreamFilter>) {
        self.stream.filter = filter;
        self.reload_current_view();
    }

    pub(super) fn select(&mut self, selection: Selection) {
        self.stream.selection = selection;
        self.reload_current_view();
    }
}

pub(super) fn load_sidebar_data(
    storage: &Storage,
    host_id: i64,
) -> crate::storage::Result<(LibraryCounts, Vec<SavedQuery>)> {
    Ok((
        storage.list_library_counts(host_id)?,
        storage.list_saved_queries(host_id)?,
    ))
}

fn current_sort(runtime: &Runtime, selection: &Selection) -> SortOrder {
    match selection {
        Selection::SavedQuery(id) => runtime
            .saved_queries
            .iter()
            .find(|query| query.id == *id)
            .map(|query| query.sort)
            .unwrap_or(runtime.config.ui.default_sort),
        Selection::Library(_) => runtime.config.ui.default_sort,
    }
}
