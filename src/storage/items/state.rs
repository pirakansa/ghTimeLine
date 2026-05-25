use rusqlite::{params, OptionalExtension};

use super::now_rfc3339;
use crate::storage::{Result, Storage};

impl Storage {
    pub fn record_saved_query_match(
        &self,
        saved_query_id: i64,
        stream_item_id: i64,
        search_rank: Option<i64>,
    ) -> Result<bool> {
        let now = now_rfc3339();
        let existed = self
            .connection()
            .query_row(
                "SELECT 1
                 FROM saved_query_matches
                 WHERE saved_query_id = ?1 AND stream_item_id = ?2",
                params![saved_query_id, stream_item_id],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        self.connection().execute(
            "INSERT INTO saved_query_matches (
                saved_query_id, stream_item_id, first_seen_at, last_seen_at, search_rank
             ) VALUES (?1, ?2, ?3, ?3, ?4)
             ON CONFLICT(saved_query_id, stream_item_id)
             DO UPDATE SET last_seen_at = excluded.last_seen_at,
                           search_rank = excluded.search_rank",
            params![saved_query_id, stream_item_id, now, search_rank],
        )?;
        Ok(!existed)
    }

    pub fn list_unread_item_ids_for_saved_query(&self, saved_query_id: i64) -> Result<Vec<i64>> {
        let mut statement = self.connection().prepare(
            "SELECT stream_item_id
             FROM saved_query_matches
             WHERE saved_query_id = ?1
               AND stream_item_id IN (
                   SELECT stream_item_id
                   FROM item_state
                   WHERE is_unread = 1 AND is_archived = 0
               )",
        )?;
        let rows = statement.query_map([saved_query_id], |row| row.get::<_, i64>(0))?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn set_read_state(&self, stream_item_id: i64, unread: bool) -> Result<()> {
        let now = now_rfc3339();
        let read_at = if unread { None } else { Some(now.as_str()) };
        let unread_at = if unread { Some(now.as_str()) } else { None };
        self.connection().execute(
            "UPDATE item_state
             SET is_unread = ?1, read_at = ?2, unread_at = ?3, updated_at = ?4
             WHERE stream_item_id = ?5",
            params![i64::from(unread), read_at, unread_at, now, stream_item_id],
        )?;
        Ok(())
    }

    pub fn mark_saved_query_read(&self, saved_query_id: i64) -> Result<usize> {
        let now = now_rfc3339();
        let updated = self.connection().execute(
            "UPDATE item_state
             SET is_unread = 0, read_at = ?1, unread_at = NULL, updated_at = ?1
             WHERE is_unread = 1
               AND is_archived = 0
               AND stream_item_id IN (
                   SELECT stream_item_id
                   FROM saved_query_matches
                   WHERE saved_query_id = ?2
               )",
            params![now, saved_query_id],
        )?;
        Ok(updated)
    }

    pub fn set_bookmarked(&self, stream_item_id: i64, bookmarked: bool) -> Result<()> {
        let now = now_rfc3339();
        let bookmarked_at = if bookmarked { Some(now.as_str()) } else { None };
        self.connection().execute(
            "UPDATE item_state
             SET is_bookmarked = ?1, bookmarked_at = ?2, updated_at = ?3
             WHERE stream_item_id = ?4",
            params![i64::from(bookmarked), bookmarked_at, now, stream_item_id],
        )?;
        Ok(())
    }

    pub fn set_archived(&self, stream_item_id: i64, archived: bool) -> Result<()> {
        let now = now_rfc3339();
        let archived_at = if archived { Some(now.as_str()) } else { None };
        self.connection().execute(
            "UPDATE item_state
             SET is_archived = ?1, archived_at = ?2, updated_at = ?3
             WHERE stream_item_id = ?4",
            params![i64::from(archived), archived_at, now, stream_item_id],
        )?;
        Ok(())
    }
}
