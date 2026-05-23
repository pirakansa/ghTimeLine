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

pub fn refresh_saved_query(
    config: &AppConfig,
    storage: &Storage,
    host_id: i64,
    saved_query: &SavedQuery,
) -> Result<usize, SyncError> {
    let mut items = github::rest::search_issues_and_pull_requests(
        &config.host,
        &config.auth.pat,
        host_id,
        &saved_query.query,
        saved_query.sort,
    )?;
    let _ = github::graphql::enrich_pull_requests(&config.host, &config.auth.pat, &mut items);

    let count = items.len();
    for (rank, item) in items.iter().enumerate() {
        let stream_item_id = storage.upsert_stream_item(item)?;
        storage.record_saved_query_match(saved_query.id, stream_item_id, Some(rank as i64))?;
    }
    storage.mark_saved_query_sync_success(saved_query.id)?;
    Ok(count)
}

pub fn refresh_saved_queries(
    config: &AppConfig,
    storage: &Storage,
    host_id: i64,
    saved_queries: &[SavedQuery],
) -> Vec<(i64, Result<usize, SyncError>)> {
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
