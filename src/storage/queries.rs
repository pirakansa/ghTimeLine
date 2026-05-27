use chrono::Utc;
use rusqlite::{params, OptionalExtension};

use crate::models::{FilterStream, LibraryCounts, SavedQuery, StreamFilter, StreamSource};
use crate::saved_query_io::{ImportedFilterStream, ImportedSavedQuery};

use super::{Result, Storage};

#[derive(Clone, Debug)]
struct FilterStreamRow {
    id: i64,
    saved_query_id: i64,
    name: String,
    filter_query: String,
    enabled: bool,
    position: i64,
}

impl Storage {
    pub fn list_library_counts(&self, host_id: i64) -> Result<LibraryCounts> {
        Ok(self.connection().query_row(
            "SELECT
                COUNT(DISTINCT CASE
                    WHEN s.is_unread = 1 AND s.is_archived = 0 THEN i.id
                END) AS inbox_unread_count,
                COUNT(DISTINCT CASE
                    WHEN s.is_unread = 1 AND s.is_bookmarked = 1 AND s.is_archived = 0 THEN i.id
                END) AS bookmark_unread_count,
                COUNT(DISTINCT CASE
                    WHEN s.is_unread = 1 AND s.is_archived = 1 THEN i.id
                END) AS archived_unread_count
             FROM stream_items i
             JOIN item_state s ON s.stream_item_id = i.id
             JOIN saved_query_matches m ON m.stream_item_id = i.id
             JOIN saved_queries q ON q.id = m.saved_query_id
             WHERE i.host_id = ?1 AND q.enabled = 1",
            params![host_id],
            |row| {
                Ok(LibraryCounts {
                    inbox_unread_count: row.get(0)?,
                    bookmark_unread_count: row.get(1)?,
                    archived_unread_count: row.get(2)?,
                })
            },
        )?)
    }

    pub fn list_saved_queries(&self, host_id: i64) -> Result<Vec<SavedQuery>> {
        let filter_stream_rows = self.list_filter_stream_rows(host_id)?;

        let mut statement = self.connection().prepare(
            "SELECT
                q.id,
                q.name,
                q.query,
                q.resource_type,
                q.enabled,
                q.position,
                COUNT(DISTINCT CASE
                    WHEN s.is_unread = 1 AND s.is_archived = 0 THEN i.id
                END) AS unread_count
             FROM saved_queries q
             LEFT JOIN saved_query_matches m ON m.saved_query_id = q.id
             LEFT JOIN stream_items i ON i.id = m.stream_item_id
             LEFT JOIN item_state s ON s.stream_item_id = i.id
             WHERE q.host_id = ?1
             GROUP BY q.id
             ORDER BY q.enabled DESC, q.position ASC, q.name ASC",
        )?;

        let rows = statement.query_map(params![host_id], |row| {
            Ok(SavedQuery {
                id: row.get(0)?,
                name: row.get(1)?,
                query: row.get(2)?,
                source: StreamSource::from_db_value(&row.get::<_, String>(3)?),
                enabled: row.get::<_, i64>(4)? == 1,
                position: row.get(5)?,
                unread_count: row.get(6)?,
                filter_streams: Vec::new(),
            })
        })?;

        let mut saved_queries = rows
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(crate::storage::StorageError::from)?;

        for saved_query in &mut saved_queries {
            saved_query.filter_streams = filter_stream_rows
                .iter()
                .filter(|row| row.saved_query_id == saved_query.id)
                .map(|row| {
                    Ok(FilterStream {
                        id: row.id,
                        saved_query_id: row.saved_query_id,
                        name: row.name.clone(),
                        filter_query: row.filter_query.clone(),
                        enabled: row.enabled,
                        position: row.position,
                        unread_count: self.count_items_for_saved_query(
                            saved_query.id,
                            Some(StreamFilter::Unread),
                            Some(&row.filter_query),
                            None,
                        )?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
        }

        Ok(saved_queries)
    }

    pub fn get_filter_stream(&self, filter_stream_id: i64) -> Result<Option<FilterStream>> {
        let row = self
            .connection()
            .query_row(
                "SELECT id, saved_query_id, name, filter_query, enabled, position
                 FROM filter_streams
                 WHERE id = ?1",
                params![filter_stream_id],
                |row| {
                    Ok(FilterStreamRow {
                        id: row.get(0)?,
                        saved_query_id: row.get(1)?,
                        name: row.get(2)?,
                        filter_query: row.get(3)?,
                        enabled: row.get::<_, i64>(4)? == 1,
                        position: row.get(5)?,
                    })
                },
            )
            .optional()?;

        row.map(|row| {
            Ok(FilterStream {
                id: row.id,
                saved_query_id: row.saved_query_id,
                name: row.name,
                filter_query: row.filter_query.clone(),
                enabled: row.enabled,
                position: row.position,
                unread_count: self.count_items_for_saved_query(
                    row.saved_query_id,
                    Some(StreamFilter::Unread),
                    Some(&row.filter_query),
                    None,
                )?,
            })
        })
        .transpose()
    }

    pub fn add_saved_query(&self, host_id: i64, name: &str, query: &str) -> Result<i64> {
        self.add_saved_query_for_source(host_id, name, query, StreamSource::IssueOrPullRequest)
    }

    pub fn add_saved_query_for_source(
        &self,
        host_id: i64,
        name: &str,
        query: &str,
        source: StreamSource,
    ) -> Result<i64> {
        let now = Utc::now().to_rfc3339();
        let next_position = self
            .connection()
            .query_row(
                "SELECT COALESCE(MAX(position), -1) + 1 FROM saved_queries WHERE host_id = ?1",
                params![host_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .unwrap_or(0);

        self.connection().execute(
            "INSERT INTO saved_queries (
                host_id, name, query, resource_type, position, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![
                host_id,
                name.trim(),
                query.trim(),
                source.as_db_value(),
                next_position,
                now
            ],
        )?;
        Ok(self.connection().last_insert_rowid())
    }

    pub fn update_saved_query(&self, saved_query_id: i64, name: &str, query: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.connection().execute(
            "UPDATE saved_queries
             SET name = ?1, query = ?2, updated_at = ?3
             WHERE id = ?4",
            params![name.trim(), query.trim(), now, saved_query_id],
        )?;
        Ok(())
    }

    pub fn update_saved_query_for_source(
        &self,
        saved_query_id: i64,
        name: &str,
        query: &str,
        source: StreamSource,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.connection().execute(
            "UPDATE saved_queries
             SET name = ?1, query = ?2, resource_type = ?3, updated_at = ?4
             WHERE id = ?5",
            params![
                name.trim(),
                query.trim(),
                source.as_db_value(),
                now,
                saved_query_id
            ],
        )?;
        Ok(())
    }

    pub fn add_filter_stream(
        &self,
        saved_query_id: i64,
        name: &str,
        filter_query: &str,
        enabled: bool,
    ) -> Result<i64> {
        let now = Utc::now().to_rfc3339();
        let next_position = self
            .connection()
            .query_row(
                "SELECT COALESCE(MAX(position), -1) + 1
                 FROM filter_streams
                 WHERE saved_query_id = ?1",
                params![saved_query_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .unwrap_or(0);

        self.connection().execute(
            "INSERT INTO filter_streams (
                saved_query_id, name, filter_query, enabled, position, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![
                saved_query_id,
                name.trim(),
                filter_query.trim(),
                i64::from(enabled),
                next_position,
                now
            ],
        )?;
        Ok(self.connection().last_insert_rowid())
    }

    pub fn update_filter_stream(
        &self,
        filter_stream_id: i64,
        name: &str,
        filter_query: &str,
        enabled: bool,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.connection().execute(
            "UPDATE filter_streams
             SET name = ?1, filter_query = ?2, enabled = ?3, updated_at = ?4
             WHERE id = ?5",
            params![
                name.trim(),
                filter_query.trim(),
                i64::from(enabled),
                now,
                filter_stream_id
            ],
        )?;
        Ok(())
    }

    pub fn delete_filter_stream(&self, filter_stream_id: i64) -> Result<()> {
        self.connection().execute(
            "DELETE FROM filter_streams WHERE id = ?1",
            params![filter_stream_id],
        )?;
        Ok(())
    }

    pub fn set_saved_query_enabled(&self, saved_query_id: i64, enabled: bool) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.connection().execute(
            "UPDATE saved_queries
             SET enabled = ?1, updated_at = ?2
             WHERE id = ?3",
            params![if enabled { 1 } else { 0 }, now, saved_query_id],
        )?;
        Ok(())
    }

    pub fn delete_saved_query(&self, saved_query_id: i64) -> Result<()> {
        self.connection().execute(
            "DELETE FROM saved_queries WHERE id = ?1",
            params![saved_query_id],
        )?;
        Ok(())
    }

    pub fn replace_saved_queries(
        &self,
        host_id: i64,
        queries: &[ImportedSavedQuery],
    ) -> Result<Vec<i64>> {
        self.with_immediate_transaction(|storage| {
            storage.connection().execute(
                "DELETE FROM saved_queries WHERE host_id = ?1",
                params![host_id],
            )?;

            let now = Utc::now().to_rfc3339();
            let mut inserted_ids = Vec::with_capacity(queries.len());
            for query in queries {
                storage.connection().execute(
                    "INSERT INTO saved_queries (
                        host_id, name, query, resource_type, enabled, position, created_at, updated_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                    params![
                        host_id,
                        query.name,
                        query.query,
                        query.source.as_db_value(),
                        if query.enabled { 1 } else { 0 },
                        query.position,
                        now
                    ],
                )?;
                let saved_query_id = storage.connection().last_insert_rowid();
                inserted_ids.push(saved_query_id);
                insert_imported_filter_streams(
                    storage,
                    saved_query_id,
                    &query.filter_streams,
                    &now,
                )?;
            }

            Ok(inserted_ids)
        })
    }

    pub fn move_saved_query_up(&self, saved_query_id: i64) -> Result<bool> {
        self.move_saved_query(saved_query_id, true)
    }

    pub fn move_saved_query_down(&self, saved_query_id: i64) -> Result<bool> {
        self.move_saved_query(saved_query_id, false)
    }

    fn move_saved_query(&self, saved_query_id: i64, move_up: bool) -> Result<bool> {
        self.with_immediate_transaction(|storage| {
            let (host_id, enabled, position): (i64, i64, i64) = storage.connection().query_row(
                "SELECT host_id, enabled, position
                 FROM saved_queries
                 WHERE id = ?1",
                params![saved_query_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;

            let target = if move_up {
                storage
                    .connection()
                    .query_row(
                        "SELECT id, position
                         FROM saved_queries
                         WHERE host_id = ?1 AND enabled = ?2 AND position < ?3
                         ORDER BY position DESC
                         LIMIT 1",
                        params![host_id, enabled, position],
                        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                    )
                    .optional()?
            } else {
                storage
                    .connection()
                    .query_row(
                        "SELECT id, position
                         FROM saved_queries
                         WHERE host_id = ?1 AND enabled = ?2 AND position > ?3
                         ORDER BY position ASC
                         LIMIT 1",
                        params![host_id, enabled, position],
                        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                    )
                    .optional()?
            };

            let Some((target_id, target_position)) = target else {
                return Ok(false);
            };

            let now = Utc::now().to_rfc3339();
            storage.connection().execute(
                "UPDATE saved_queries
                 SET position = ?1, updated_at = ?2
                 WHERE id = ?3",
                params![target_position, now, saved_query_id],
            )?;
            storage.connection().execute(
                "UPDATE saved_queries
                 SET position = ?1, updated_at = ?2
                 WHERE id = ?3",
                params![position, now, target_id],
            )?;

            Ok(true)
        })
    }

    pub fn mark_saved_query_sync_success(&self, saved_query_id: i64) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.connection().execute(
            "UPDATE saved_queries
             SET last_successful_sync_at = ?1, last_sync_error = NULL, updated_at = ?1
             WHERE id = ?2",
            params![now, saved_query_id],
        )?;
        Ok(())
    }

    pub fn mark_saved_query_sync_error(&self, saved_query_id: i64, message: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.connection().execute(
            "UPDATE saved_queries
             SET last_sync_error = ?1, updated_at = ?2
             WHERE id = ?3",
            params![message, now, saved_query_id],
        )?;
        Ok(())
    }

    fn list_filter_stream_rows(&self, host_id: i64) -> Result<Vec<FilterStreamRow>> {
        let mut statement = self.connection().prepare(
            "SELECT
                f.id,
                f.saved_query_id,
                f.name,
                f.filter_query,
                f.enabled,
                f.position
             FROM filter_streams f
             JOIN saved_queries q ON q.id = f.saved_query_id
             WHERE q.host_id = ?1
             ORDER BY f.enabled DESC, f.position ASC, f.name ASC",
        )?;

        let rows = statement.query_map(params![host_id], |row| {
            Ok(FilterStreamRow {
                id: row.get(0)?,
                saved_query_id: row.get(1)?,
                name: row.get(2)?,
                filter_query: row.get(3)?,
                enabled: row.get::<_, i64>(4)? == 1,
                position: row.get(5)?,
            })
        })?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }
}

fn insert_imported_filter_streams(
    storage: &Storage,
    saved_query_id: i64,
    filter_streams: &[ImportedFilterStream],
    now: &str,
) -> Result<()> {
    for filter_stream in filter_streams {
        storage.connection().execute(
            "INSERT INTO filter_streams (
                saved_query_id, name, filter_query, enabled, position, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![
                saved_query_id,
                filter_stream.name,
                filter_stream.filter_query,
                if filter_stream.enabled { 1 } else { 0 },
                filter_stream.position,
                now
            ],
        )?;
    }

    Ok(())
}
