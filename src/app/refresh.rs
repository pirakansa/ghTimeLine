use std::time::{Duration, Instant};

use crate::app::{AppMode, GhStreamApp};
use crate::models::{SavedQuery, Selection};
use crate::sync;

impl GhStreamApp {
    pub(super) fn refresh_now(&mut self) {
        self.refresh_selected_scope("Manual refresh");
    }

    pub(super) fn maybe_poll(&mut self) {
        let AppMode::Main(runtime) = &self.mode else {
            return;
        };
        let interval =
            Duration::from_secs(u64::from(runtime.config.refresh.polling_interval_minutes) * 60);
        if self.last_poll_at.is_none() {
            self.last_poll_at = Some(Instant::now());
            return;
        }
        if self
            .last_poll_at
            .is_none_or(|last_poll_at| last_poll_at.elapsed() >= interval)
        {
            self.last_poll_at = Some(Instant::now());
            self.refresh_all_queries("Polling refresh");
        }
    }

    fn refresh_selected_scope(&mut self, label: &str) {
        let queries = match &self.mode {
            AppMode::Main(runtime) => match self.stream.selection {
                Selection::SavedQuery(id) => runtime
                    .saved_queries
                    .iter()
                    .find(|query| query.id == id)
                    .cloned()
                    .into_iter()
                    .collect::<Vec<_>>(),
                Selection::Library(_) => runtime.saved_queries.clone(),
            },
            AppMode::Setup => Vec::new(),
        };
        self.refresh_queries(label, &queries);
    }

    fn refresh_all_queries(&mut self, label: &str) {
        let queries = match &self.mode {
            AppMode::Main(runtime) => runtime.saved_queries.clone(),
            AppMode::Setup => Vec::new(),
        };
        self.refresh_queries(label, &queries);
    }

    fn refresh_queries(&mut self, label: &str, queries: &[SavedQuery]) {
        if queries.is_empty() {
            self.status = format!("{label}: no saved queries to refresh.");
            return;
        }

        if let AppMode::Main(runtime) = &mut self.mode {
            let results = sync::refresh_saved_queries(
                &runtime.config,
                &runtime.storage,
                runtime.host_id,
                queries,
            );
            let refreshed_count = results
                .iter()
                .filter_map(|(_, result)| result.as_ref().ok())
                .sum::<usize>();
            let failed_count = results.iter().filter(|(_, result)| result.is_err()).count();
            self.status = refresh_status(label, refreshed_count, failed_count);
        }
        self.reload_queries();
        self.reload_current_view();
    }
}

fn refresh_status(label: &str, refreshed_count: usize, failed_count: usize) -> String {
    if failed_count == 0 {
        format!("{label}: refreshed {refreshed_count} items.")
    } else {
        format!(
            "{label}: refreshed {refreshed_count} items; {failed_count} query refreshes failed."
        )
    }
}
