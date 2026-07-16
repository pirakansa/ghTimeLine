use std::collections::HashMap;

use chrono::{DateTime, Duration, SecondsFormat, Utc};
use thiserror::Error;

use crate::github;
use crate::models::{AppConfig, SavedQuery, StreamSource};
use crate::storage::items::{StreamItemSave, StreamItemUpsert};
use crate::storage::{Storage, StorageError};

const ISSUE_SEARCH_PAGE: u16 = 1;
const DELTA_SEARCH_OVERLAP_SECONDS: i64 = 60;

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
    item_type: crate::models::ItemType,
}

impl From<&StreamItemUpsert> for StreamItemKey {
    fn from(item: &StreamItemUpsert) -> Self {
        Self {
            host_id: item.host_id,
            repository_owner: item.repository_owner.clone(),
            repository_name: item.repository_name.clone(),
            number: item.number,
            item_type: item.item_type.clone(),
        }
    }
}

#[derive(Clone)]
struct CachedSave {
    item: StreamItemUpsert,
    save: StreamItemSave,
}

enum PendingRefresh {
    Fetched {
        query: SavedQuery,
        items: Vec<github::FetchedStreamItem>,
    },
    Failed {
        query_id: i64,
        error: SyncError,
    },
}

fn into_upsert(
    host_id: i64,
    item: github::FetchedStreamItem,
    graphql_enriched: bool,
) -> StreamItemUpsert {
    StreamItemUpsert {
        host_id,
        node_id: item.node_id,
        repository_owner: item.repository_owner,
        repository_name: item.repository_name,
        number: item.number,
        item_type: item.item_type,
        title: item.title,
        author_login: item.author_login,
        author_avatar_url: item.author_avatar_url,
        html_url: item.html_url,
        api_url: item.api_url,
        state: item.state,
        is_draft: item.is_draft,
        is_merged: item.is_merged,
        review_status: item.review_status,
        comment_count: item.comment_count,
        created_at_github: item.created_at_github,
        updated_at_github: item.updated_at_github,
        closed_at_github: item.closed_at_github,
        merged_at_github: item.merged_at_github,
        labels: item.labels,
        assignees: item.assignees,
        review_requests: item.review_requests,
        reviewers: item.reviewers,
        participants: item.participants,
        mentions: item.mentions,
        graphql_enriched,
    }
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
    let mut items = fetch_saved_query_items(config, storage, saved_query)?;
    let enrichment = if matches!(
        saved_query.source,
        StreamSource::IssueOrPullRequest | StreamSource::ProjectV2
    ) {
        Some(github::graphql::enrich_fetched_items(
            &config.host,
            &config.auth.pat,
            &mut items,
        ))
    } else {
        None
    };
    let items = items
        .into_iter()
        .map(|item| {
            let graphql_enriched = relations_are_complete(
                saved_query.source,
                &item,
                enrichment.as_ref().map(|report| &report.enriched_node_ids),
            );
            into_upsert(host_id, item, graphql_enriched)
        })
        .collect::<Vec<_>>();
    persist_saved_query_items(storage, saved_query, &items, item_cache)
}

fn fetch_saved_query_items(
    config: &AppConfig,
    storage: &Storage,
    saved_query: &SavedQuery,
) -> Result<Vec<github::FetchedStreamItem>, SyncError> {
    match saved_query.source {
        StreamSource::IssueOrPullRequest => {
            let last_successful_sync_at =
                storage.saved_query_last_successful_sync_at(saved_query.id)?;
            let query = issue_search_query(&saved_query.query, last_successful_sync_at.as_deref());
            github::rest::fetch_issues_and_pull_requests_page(
                &config.host,
                &config.auth.pat,
                &query,
                ISSUE_SEARCH_PAGE,
                github::rest::SEARCH_PER_PAGE,
            )
            .map(|page| page.items)
        }
        StreamSource::Discussion => github::discussion::fetch_discussions(
            &config.host,
            &config.auth.pat,
            &saved_query.query,
        ),
        StreamSource::ProjectV2 => {
            github::project::fetch_project_items(&config.host, &config.auth.pat, &saved_query.query)
        }
    }
    .map_err(SyncError::from)
}

fn relations_are_complete(
    source: StreamSource,
    item: &github::FetchedStreamItem,
    enriched_node_ids: Option<&std::collections::HashSet<String>>,
) -> bool {
    source == StreamSource::Discussion
        || item.node_id.as_ref().is_some_and(|node_id| {
            enriched_node_ids.is_some_and(|enriched| enriched.contains(node_id))
        })
}

fn issue_search_query(base_query: &str, last_successful_sync_at: Option<&str>) -> String {
    let Some(last_successful_sync_at) = last_successful_sync_at else {
        return base_query.to_owned();
    };
    let updated_since =
        sync_window_start(last_successful_sync_at).unwrap_or(last_successful_sync_at.to_owned());
    format!("{base_query} updated:>={updated_since}")
}

fn sync_window_start(last_successful_sync_at: &str) -> Option<String> {
    let parsed = DateTime::parse_from_rfc3339(last_successful_sync_at).ok()?;
    Some(
        (parsed.with_timezone(&Utc) - Duration::seconds(DELTA_SEARCH_OVERLAP_SECONDS))
            .to_rfc3339_opts(SecondsFormat::Secs, true),
    )
}

fn persist_saved_query_items(
    storage: &Storage,
    saved_query: &SavedQuery,
    items: &[StreamItemUpsert],
    item_cache: &mut HashMap<StreamItemKey, CachedSave>,
) -> Result<RefreshStats, SyncError> {
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
    let mut pending = saved_queries
        .iter()
        .filter(|query| query.enabled)
        .map(
            |query| match fetch_saved_query_items(config, storage, query) {
                Ok(items) => PendingRefresh::Fetched {
                    query: query.clone(),
                    items,
                },
                Err(error) => PendingRefresh::Failed {
                    query_id: query.id,
                    error,
                },
            },
        )
        .collect::<Vec<_>>();

    let successful_items = pending
        .iter_mut()
        .filter_map(|refresh| match refresh {
            PendingRefresh::Fetched { query, items }
                if matches!(
                    query.source,
                    StreamSource::IssueOrPullRequest | StreamSource::ProjectV2
                ) =>
            {
                Some(items)
            }
            PendingRefresh::Failed { .. } => None,
            PendingRefresh::Fetched { .. } => None,
        })
        .flat_map(|items| items.iter_mut());
    let enrichment =
        github::graphql::enrich_fetched_item_iter(&config.host, &config.auth.pat, successful_items);

    let mut item_cache = HashMap::new();
    pending
        .into_iter()
        .map(|refresh| {
            let (query_id, result) = match refresh {
                PendingRefresh::Fetched { query, items } => {
                    let items = items
                        .into_iter()
                        .map(|item| {
                            let graphql_enriched = relations_are_complete(
                                query.source,
                                &item,
                                Some(&enrichment.enriched_node_ids),
                            );
                            into_upsert(host_id, item, graphql_enriched)
                        })
                        .collect::<Vec<_>>();
                    (
                        query.id,
                        persist_saved_query_items(storage, &query, &items, &mut item_cache),
                    )
                }
                PendingRefresh::Failed { query_id, error } => (query_id, Err(error)),
            };
            if let Err(error) = &result {
                let _ = storage.mark_saved_query_sync_error(query_id, &short_error(error));
            }
            (query_id, result)
        })
        .collect()
}

fn short_error(error: &SyncError) -> String {
    let message = error.to_string();
    message.chars().take(240).collect()
}
