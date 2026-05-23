use std::sync::mpsc;
use std::time::{Duration, Instant};

use eframe::egui;

use crate::app::{AppMode, GhStreamApp};
use crate::models::{SavedQuery, Selection};
use crate::sync;

pub(super) struct RefreshOutcome {
    pub label: String,
    pub refreshed_count: usize,
    pub failed_count: usize,
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
        self.status = refresh_status(
            &outcome.label,
            outcome.refreshed_count,
            outcome.failed_count,
        );
        self.reload_queries();
        self.reload_current_view();
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
            self.status = format!("{label}: no saved queries to refresh.");
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

        self.status = format!("{label}: refreshing...");

        let (tx, rx) = mpsc::channel();
        self.refresh_rx = Some(rx);

        std::thread::spawn(move || {
            let outcome = match crate::storage::Storage::open(&database_path) {
                Err(_) => RefreshOutcome {
                    failed_count: queries.len(),
                    label,
                    refreshed_count: 0,
                },
                Ok(storage) => {
                    let results = sync::refresh_saved_queries(&config, &storage, host_id, &queries);
                    RefreshOutcome {
                        refreshed_count: results
                            .iter()
                            .filter_map(|(_, r)| r.as_ref().ok())
                            .sum::<usize>(),
                        failed_count: results.iter().filter(|(_, r)| r.is_err()).count(),
                        label,
                    }
                }
            };
            let _ = tx.send(outcome);
            ctx.request_repaint();
        });
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
