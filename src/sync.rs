use std::collections::HashMap;

use thiserror::Error;

use crate::github;
use crate::models::{AppConfig, SavedQuery};
use crate::storage::items::{StreamItemSave, StreamItemUpsert};
use crate::storage::{Storage, StorageError};

#[derive(Debug, Error)]
pub enum SyncError {
    #[error("{0}")]
    GitHub(#[from] github::GitHubError),
    #[error("{0}")]
    Storage(#[from] StorageError),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RefreshStats {
    pub processed_count: usize,
    pub changed_count: usize,
    pub changed_item_ids: Vec<i64>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct StreamItemKey {
    host_id: i64,
    repository_owner: String,
    repository_name: String,
    number: i64,
    is_pull_request: bool,
}

impl From<&StreamItemUpsert> for StreamItemKey {
    fn from(item: &StreamItemUpsert) -> Self {
        Self {
            host_id: item.host_id,
            repository_owner: item.repository_owner.clone(),
            repository_name: item.repository_name.clone(),
            number: item.number,
            is_pull_request: item.item_type == crate::models::ItemType::PullRequest,
        }
    }
}

#[derive(Clone)]
struct CachedSave {
    item: StreamItemUpsert,
    save: StreamItemSave,
}

pub fn refresh_saved_query(
    config: &AppConfig,
    storage: &Storage,
    host_id: i64,
    saved_query: &SavedQuery,
) -> Result<RefreshStats, SyncError> {
    refresh_saved_query_with_cache(config, storage, host_id, saved_query, &mut HashMap::new())
}

fn refresh_saved_query_with_cache(
    config: &AppConfig,
    storage: &Storage,
    host_id: i64,
    saved_query: &SavedQuery,
    item_cache: &mut HashMap<StreamItemKey, CachedSave>,
) -> Result<RefreshStats, SyncError> {
    let mut items = github::rest::search_issues_and_pull_requests(
        &config.host,
        &config.auth.pat,
        host_id,
        &saved_query.query,
        saved_query.sort,
    )?;
    let _ = github::graphql::enrich_pull_requests(&config.host, &config.auth.pat, &mut items);

    let mut pending_cache = HashMap::<StreamItemKey, CachedSave>::new();
    let stats = storage
        .with_immediate_transaction(|storage| {
            let mut stats = RefreshStats {
                processed_count: items.len(),
                changed_count: 0,
                changed_item_ids: Vec::new(),
            };
            for (rank, item) in items.iter().enumerate() {
                let key = StreamItemKey::from(item);
                let cached_save = pending_cache
                    .get(&key)
                    .or_else(|| item_cache.get(&key))
                    .filter(|cached| cached.item == *item)
                    .map(|cached| cached.save);
                let save = match cached_save {
                    Some(save) => save,
                    None => {
                        let save = storage.upsert_stream_item(item)?;
                        pending_cache.insert(
                            key,
                            CachedSave {
                                item: item.clone(),
                                save,
                            },
                        );
                        save
                    }
                };
                let has_new_match =
                    storage.record_saved_query_match(saved_query.id, save.id, Some(rank as i64))?;
                if save.changed || has_new_match {
                    stats.changed_count += 1;
                    stats.changed_item_ids.push(save.id);
                }
            }
            storage.mark_saved_query_sync_success(saved_query.id)?;
            Ok(stats)
        })
        .map_err(SyncError::from)?;
    item_cache.extend(pending_cache);
    Ok(stats)
}

pub fn refresh_saved_queries(
    config: &AppConfig,
    storage: &Storage,
    host_id: i64,
    saved_queries: &[SavedQuery],
) -> Vec<(i64, Result<RefreshStats, SyncError>)> {
    let mut item_cache = HashMap::new();
    saved_queries
        .iter()
        .filter(|query| query.enabled)
        .map(|query| {
            let result =
                refresh_saved_query_with_cache(config, storage, host_id, query, &mut item_cache);
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
