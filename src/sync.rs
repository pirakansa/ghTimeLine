use thiserror::Error;

use crate::github;
use crate::models::{AppConfig, SavedQuery};
use crate::storage::{Storage, StorageError};

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("{0}")]
    GitHub(#[from] github::GitHubError),
    #[error("{0}")]
    Storage(#[from] StorageError),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RefreshStats {
    pub processed_count: usize,
    pub changed_count: usize,
}

pub fn refresh_saved_query(
    config: &AppConfig,
    storage: &Storage,
    host_id: i64,
    saved_query: &SavedQuery,
) -> Result<RefreshStats, SyncError> {
    let mut items = github::rest::search_issues_and_pull_requests(
        &config.host,
        &config.auth.pat,
        host_id,
        &saved_query.query,
        saved_query.sort,
    )?;
    let _ = github::graphql::enrich_pull_requests(&config.host, &config.auth.pat, &mut items);

    let mut stats = RefreshStats {
        processed_count: items.len(),
        changed_count: 0,
    };
    for (rank, item) in items.iter().enumerate() {
        let save = storage.upsert_stream_item(item)?;
        if save.changed {
            stats.changed_count += 1;
        }
        storage.record_saved_query_match(saved_query.id, save.id, Some(rank as i64))?;
    }
    storage.mark_saved_query_sync_success(saved_query.id)?;
    Ok(stats)
}

pub fn refresh_saved_queries(
    config: &AppConfig,
    storage: &Storage,
    host_id: i64,
    saved_queries: &[SavedQuery],
) -> Vec<(i64, Result<RefreshStats, SyncError>)> {
    saved_queries
        .iter()
        .filter(|query| query.enabled)
        .map(|query| {
            let result = refresh_saved_query(config, storage, host_id, query);
            if let Err(error) = &result {
                let _ = storage.mark_saved_query_sync_error(query.id, &short_error(error));
            }
            (query.id, result)
        })
        .collect()
}

fn short_error(error: &SyncError) -> String {
    let message = error.to_string();
    message.chars().take(240).collect()
}
