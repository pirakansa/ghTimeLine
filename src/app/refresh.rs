use std::sync::mpsc;
use std::time::{Duration, Instant};

use eframe::egui;

use crate::app::{AppMode, GhStreamApp};
use crate::models::{SavedQuery, Selection};
use crate::sync;

pub(super) struct RefreshOutcome {
    pub label: String,
    pub processed_count: usize,
    pub changed_count: usize,
    pub failed_count: usize,
    pub changed_item_ids: Vec<i64>,
}

impl GhStreamApp {
    pub(super) fn refresh_now(&mut self, ctx: egui::Context) {
        self.refresh_selected_scope("Manual refresh", ctx);
    }

    pub(super) fn maybe_poll(&mut self, ctx: &egui::Context) {
        let AppMode::Main(runtime) = &self.mode else {
            return;
        };
        let interval =
            Duration::from_secs(u64::from(runtime.config.refresh.polling_interval_seconds));
        if self.last_poll_at.is_none() {
            self.last_poll_at = Some(Instant::now());
            return;
        }
        if self
            .last_poll_at
            .is_none_or(|last_poll_at| last_poll_at.elapsed() >= interval)
        {
            self.last_poll_at = Some(Instant::now());
            self.refresh_all_queries("Polling refresh", ctx.clone());
        }
    }

    /// Check if a background refresh has completed and apply the result.
    pub(super) fn poll_refresh_result(&mut self) {
        let outcome = match &self.refresh_rx {
            Some(rx) => match rx.try_recv() {
                Ok(outcome) => outcome,
                Err(_) => return,
            },
            None => return,
        };
        self.refresh_rx = None;
        Self::replace_status(
            &mut self.status,
            &mut self.status_history,
            refresh_status(
                &outcome.label,
                outcome.processed_count,
                outcome.changed_count,
                outcome.failed_count,
            ),
        );
        if !outcome.changed_item_ids.is_empty() || outcome.failed_count > 0 {
            self.reload_queries();
        }
        if !outcome.changed_item_ids.is_empty() {
            self.reload_current_view_for_changed_items(&outcome.changed_item_ids);
        }
    }

    fn refresh_selected_scope(&mut self, label: &str, ctx: egui::Context) {
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
            AppMode::Setup { .. } => Vec::new(),
        };
        self.refresh_queries(label, &queries, ctx);
    }

    fn refresh_all_queries(&mut self, label: &str, ctx: egui::Context) {
        let queries = match &self.mode {
            AppMode::Main(runtime) => runtime.saved_queries.clone(),
            AppMode::Setup { .. } => Vec::new(),
        };
        self.refresh_queries(label, &queries, ctx);
    }

    fn refresh_queries(&mut self, label: &str, queries: &[SavedQuery], ctx: egui::Context) {
        if queries.is_empty() {
            Self::replace_status(
                &mut self.status,
                &mut self.status_history,
                format!("{label}: no saved queries to refresh."),
            );
            return;
        }

        // Skip if a refresh is already in progress.
        if self.refresh_rx.is_some() {
            return;
        }

        let AppMode::Main(runtime) = &self.mode else {
            return;
        };

        let config = runtime.config.clone();
        let database_path = self.database_path.clone();
        let host_id = runtime.host_id;
        let queries = queries.to_vec();
        let label = label.to_owned();

        Self::replace_status(
            &mut self.status,
            &mut self.status_history,
            format!("{label}: refreshing..."),
        );

        let (tx, rx) = mpsc::channel();
        self.refresh_rx = Some(rx);

        std::thread::spawn(move || {
            let outcome = match crate::storage::Storage::open(&database_path) {
                Err(_) => RefreshOutcome {
                    failed_count: queries.len(),
                    label,
                    processed_count: 0,
                    changed_count: 0,
                    changed_item_ids: Vec::new(),
                },
                Ok(storage) => {
                    let results = sync::refresh_saved_queries(&config, &storage, host_id, &queries);
                    let mut changed_item_ids = results
                        .iter()
                        .filter_map(|(_, r)| r.as_ref().ok())
                        .flat_map(|stats| stats.changed_item_ids.iter().copied())
                        .collect::<Vec<_>>();
                    changed_item_ids.sort_unstable();
                    changed_item_ids.dedup();
                    RefreshOutcome {
                        processed_count: results
                            .iter()
                            .filter_map(|(_, r)| r.as_ref().ok())
                            .map(|stats| stats.processed_count)
                            .sum(),
                        changed_count: results
                            .iter()
                            .filter_map(|(_, r)| r.as_ref().ok())
                            .map(|stats| stats.changed_count)
                            .sum(),
                        failed_count: results.iter().filter(|(_, r)| r.is_err()).count(),
                        label,
                        changed_item_ids,
                    }
                }
            };
            let _ = tx.send(outcome);
            ctx.request_repaint();
        });
    }
}

fn refresh_status(
    label: &str,
    processed_count: usize,
    changed_count: usize,
    failed_count: usize,
) -> String {
    if failed_count == 0 {
        format!("{label}: processed {processed_count} items; {changed_count} changed.")
    } else {
        format!(
            "{label}: processed {processed_count} items; {changed_count} changed; {failed_count} query refreshes failed."
        )
    }
}

#[cfg(test)]
mod tests {
    use super::refresh_status;

    #[test]
    fn refresh_status_reports_processed_and_changed_counts() {
        assert_eq!(
            refresh_status("Polling refresh", 51, 1, 0),
            "Polling refresh: processed 51 items; 1 changed."
        );
    }

    #[test]
    fn refresh_status_reports_failures_with_processed_and_changed_counts() {
        assert_eq!(
            refresh_status("Polling refresh", 51, 1, 2),
            "Polling refresh: processed 51 items; 1 changed; 2 query refreshes failed."
        );
    }
}
