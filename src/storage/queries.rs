use std::str::FromStr;

use chrono::Utc;
use rusqlite::{params, OptionalExtension};

use crate::models::{LibraryCounts, SavedQuery, SortOrder};

use super::{Result, Storage};

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
        let mut statement = self.connection().prepare(
            "SELECT
                q.id,
                q.name,
                q.query,
                q.sort,
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
            let sort_text: String = row.get(3)?;
            Ok(SavedQuery {
                id: row.get(0)?,
                name: row.get(1)?,
                query: row.get(2)?,
                sort: SortOrder::from_str(&sort_text).unwrap_or(SortOrder::UpdatedDesc),
                enabled: row.get::<_, i64>(4)? == 1,
                position: row.get(5)?,
                unread_count: row.get(6)?,
            })
        })?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn add_saved_query(
        &self,
        host_id: i64,
        name: &str,
        query: &str,
        sort: SortOrder,
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
                host_id, name, query, sort, position, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            params![
                host_id,
                name.trim(),
                query.trim(),
                sort.as_db_value(),
                next_position,
                now
            ],
        )?;
        Ok(self.connection().last_insert_rowid())
    }

    pub fn update_saved_query(
        &self,
        saved_query_id: i64,
        name: &str,
        query: &str,
        sort: SortOrder,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.connection().execute(
            "UPDATE saved_queries
             SET name = ?1, query = ?2, sort = ?3, updated_at = ?4
             WHERE id = ?5",
            params![
                name.trim(),
                query.trim(),
                sort.as_db_value(),
                now,
                saved_query_id
            ],
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
}
