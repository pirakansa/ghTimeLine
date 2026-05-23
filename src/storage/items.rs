use chrono::Utc;
use rusqlite::params;

use crate::models::{ItemType, LibraryView, SortOrder, StreamFilter, StreamItem};

use super::{Result, Storage};

#[derive(Clone, Debug)]
pub struct StreamItemUpsert {
    pub host_id: i64,
    pub node_id: Option<String>,
    pub repository_owner: String,
    pub repository_name: String,
    pub number: i64,
    pub item_type: ItemType,
    pub title: String,
    pub author_login: Option<String>,
    pub html_url: String,
    pub api_url: Option<String>,
    pub state: String,
    pub is_draft: Option<bool>,
    pub is_merged: Option<bool>,
    pub review_status: Option<String>,
    pub comment_count: i64,
    pub created_at_github: String,
    pub updated_at_github: String,
    pub closed_at_github: Option<String>,
    pub merged_at_github: Option<String>,
    pub labels: Vec<String>,
    pub assignees: Vec<String>,
}

impl Storage {
    pub fn upsert_stream_item(&self, item: &StreamItemUpsert) -> Result<i64> {
        let now = Utc::now().to_rfc3339();
        self.connection().execute(
            "INSERT INTO stream_items (
                host_id, node_id, repository_owner, repository_name, number, item_type,
                title, author_login, html_url, api_url, state, is_draft, is_merged,
                review_status, comment_count, created_at_github, updated_at_github,
                closed_at_github, merged_at_github, last_seen_at, created_at, updated_at
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                ?16, ?17, ?18, ?19, ?20, ?20, ?20
             )
             ON CONFLICT(host_id, repository_owner, repository_name, number, item_type)
             DO UPDATE SET
                node_id = excluded.node_id,
                title = excluded.title,
                author_login = excluded.author_login,
                html_url = excluded.html_url,
                api_url = excluded.api_url,
                state = excluded.state,
                is_draft = excluded.is_draft,
                is_merged = excluded.is_merged,
                review_status = excluded.review_status,
                comment_count = excluded.comment_count,
                created_at_github = excluded.created_at_github,
                updated_at_github = excluded.updated_at_github,
                closed_at_github = excluded.closed_at_github,
                merged_at_github = excluded.merged_at_github,
                last_seen_at = excluded.last_seen_at,
                updated_at = excluded.updated_at",
            params![
                item.host_id,
                item.node_id,
                item.repository_owner,
                item.repository_name,
                item.number,
                item_type_db_value(&item.item_type),
                item.title,
                item.author_login,
                item.html_url,
                item.api_url,
                item.state,
                item.is_draft.map(i64::from),
                item.is_merged.map(i64::from),
                item.review_status,
                item.comment_count,
                item.created_at_github,
                item.updated_at_github,
                item.closed_at_github,
                item.merged_at_github,
                now
            ],
        )?;

        let id = self.connection().query_row(
            "SELECT id FROM stream_items
             WHERE host_id = ?1
               AND repository_owner = ?2
               AND repository_name = ?3
               AND number = ?4
               AND item_type = ?5",
            params![
                item.host_id,
                item.repository_owner,
                item.repository_name,
                item.number,
                item_type_db_value(&item.item_type)
            ],
            |row| row.get::<_, i64>(0),
        )?;

        self.connection().execute(
            "INSERT OR IGNORE INTO item_state (stream_item_id, updated_at)
             VALUES (?1, ?2)",
            params![id, now],
        )?;

        self.replace_labels(id, &item.labels)?;
        self.replace_assignees(id, &item.assignees)?;

        Ok(id)
    }

    pub fn record_saved_query_match(
        &self,
        saved_query_id: i64,
        stream_item_id: i64,
        search_rank: Option<i64>,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.connection().execute(
            "INSERT INTO saved_query_matches (
                saved_query_id, stream_item_id, first_seen_at, last_seen_at, search_rank
             ) VALUES (?1, ?2, ?3, ?3, ?4)
             ON CONFLICT(saved_query_id, stream_item_id)
             DO UPDATE SET last_seen_at = excluded.last_seen_at,
                           search_rank = excluded.search_rank",
            params![saved_query_id, stream_item_id, now, search_rank],
        )?;
        Ok(())
    }

    pub fn list_items_for_saved_query(
        &self,
        saved_query_id: i64,
        filter: Option<StreamFilter>,
        sort: SortOrder,
    ) -> Result<Vec<StreamItem>> {
        let mut sql = item_select_sql(
            "JOIN saved_query_matches m ON m.stream_item_id = i.id",
            "m.saved_query_id = ?1 AND s.is_archived = 0",
            filter,
            sort,
        );
        sql.push_str(" LIMIT 500");
        self.query_items(&sql, params![saved_query_id])
    }

    pub fn list_items_for_library(
        &self,
        host_id: i64,
        library: LibraryView,
        filter: Option<StreamFilter>,
        sort: SortOrder,
    ) -> Result<Vec<StreamItem>> {
        let library_clause = match library {
            LibraryView::Inbox => "s.is_archived = 0",
            LibraryView::Bookmark => "s.is_bookmarked = 1 AND s.is_archived = 0",
            LibraryView::Archived => "s.is_archived = 1",
        };
        let where_clause = format!(
            "i.host_id = ?1 AND EXISTS (
                SELECT 1 FROM saved_query_matches m
                JOIN saved_queries q ON q.id = m.saved_query_id
                WHERE m.stream_item_id = i.id AND q.enabled = 1
             ) AND {library_clause}"
        );
        let sql = item_select_sql("", &where_clause, filter, sort);
        self.query_items(&sql, params![host_id])
    }

    pub fn set_read_state(&self, stream_item_id: i64, unread: bool) -> Result<()> {
        let now = Utc::now().to_rfc3339();
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

    pub fn set_bookmarked(&self, stream_item_id: i64, bookmarked: bool) -> Result<()> {
        let now = Utc::now().to_rfc3339();
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
        let now = Utc::now().to_rfc3339();
        let archived_at = if archived { Some(now.as_str()) } else { None };
        self.connection().execute(
            "UPDATE item_state
             SET is_archived = ?1, archived_at = ?2, updated_at = ?3
             WHERE stream_item_id = ?4",
            params![i64::from(archived), archived_at, now, stream_item_id],
        )?;
        Ok(())
    }

    fn replace_labels(&self, stream_item_id: i64, labels: &[String]) -> Result<()> {
        self.connection().execute(
            "DELETE FROM stream_item_labels WHERE stream_item_id = ?1",
            params![stream_item_id],
        )?;
        for label in labels {
            self.connection().execute(
                "INSERT INTO stream_item_labels (stream_item_id, name) VALUES (?1, ?2)",
                params![stream_item_id, label],
            )?;
        }
        Ok(())
    }

    fn replace_assignees(&self, stream_item_id: i64, assignees: &[String]) -> Result<()> {
        self.connection().execute(
            "DELETE FROM stream_item_assignees WHERE stream_item_id = ?1",
            params![stream_item_id],
        )?;
        for assignee in assignees {
            self.connection().execute(
                "INSERT INTO stream_item_assignees (stream_item_id, login) VALUES (?1, ?2)",
                params![stream_item_id, assignee],
            )?;
        }
        Ok(())
    }

    fn query_items<P>(&self, sql: &str, params: P) -> Result<Vec<StreamItem>>
    where
        P: rusqlite::Params,
    {
        let mut statement = self.connection().prepare(sql)?;
        let rows = statement.query_map(params, |row| {
            let id = row.get::<_, i64>(0)?;
            Ok(StreamItem {
                id,
                repository_owner: row.get(1)?,
                repository_name: row.get(2)?,
                number: row.get(3)?,
                item_type: item_type_from_db(&row.get::<_, String>(4)?),
                title: row.get(5)?,
                author_login: row.get(6)?,
                html_url: row.get(7)?,
                state: row.get(8)?,
                is_draft: row.get::<_, Option<i64>>(9)?.map(|value| value == 1),
                is_merged: row.get::<_, Option<i64>>(10)?.map(|value| value == 1),
                review_status: row.get(11)?,
                comment_count: row.get(12)?,
                updated_at_github: row.get(13)?,
                labels: split_list(&row.get::<_, Option<String>>(14)?.unwrap_or_default()),
                assignees: split_list(&row.get::<_, Option<String>>(15)?.unwrap_or_default()),
                is_unread: row.get::<_, i64>(16)? == 1,
                is_bookmarked: row.get::<_, i64>(17)? == 1,
                is_archived: row.get::<_, i64>(18)? == 1,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }
}

fn item_select_sql(
    extra_join: &str,
    base_where: &str,
    filter: Option<StreamFilter>,
    sort: SortOrder,
) -> String {
    let filter_clause = match filter {
        Some(StreamFilter::Open) => {
            " AND (i.item_type = 'issue' AND i.state = 'open'
               OR i.item_type = 'pull_request' AND i.state = 'open' AND COALESCE(i.is_merged, 0) = 0)"
        }
        Some(StreamFilter::Unread) => " AND s.is_unread = 1",
        Some(StreamFilter::Bookmarked) => " AND s.is_bookmarked = 1",
        None => "",
    };
    let order = match sort {
        SortOrder::UpdatedDesc => "i.updated_at_github DESC",
        SortOrder::UpdatedAsc => "i.updated_at_github ASC",
        SortOrder::CreatedDesc => "i.created_at_github DESC",
        SortOrder::CreatedAsc => "i.created_at_github ASC",
        SortOrder::CommentsDesc => "i.comment_count DESC, i.updated_at_github DESC",
        SortOrder::CommentsAsc => "i.comment_count ASC, i.updated_at_github DESC",
    };
    format!(
        "SELECT
            i.id,
            i.repository_owner,
            i.repository_name,
            i.number,
            i.item_type,
            i.title,
            i.author_login,
            i.html_url,
            i.state,
            i.is_draft,
            i.is_merged,
            i.review_status,
            i.comment_count,
            i.updated_at_github,
            (SELECT GROUP_CONCAT(name, ',') FROM stream_item_labels WHERE stream_item_id = i.id),
            (SELECT GROUP_CONCAT(login, ',') FROM stream_item_assignees WHERE stream_item_id = i.id),
            s.is_unread,
            s.is_bookmarked,
            s.is_archived
         FROM stream_items i
         JOIN item_state s ON s.stream_item_id = i.id
         {extra_join}
         WHERE {base_where}{filter_clause}
         GROUP BY i.id
         ORDER BY {order}"
    )
}

fn item_type_db_value(item_type: &ItemType) -> &'static str {
    match item_type {
        ItemType::Issue => "issue",
        ItemType::PullRequest => "pull_request",
    }
}

fn item_type_from_db(value: &str) -> ItemType {
    match value {
        "pull_request" => ItemType::PullRequest,
        _ => ItemType::Issue,
    }
}

fn split_list(value: &str) -> Vec<String> {
    if value.is_empty() {
        Vec::new()
    } else {
        value.split(',').map(ToOwned::to_owned).collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::models::{AppConfig, HostKind};
    use crate::storage::Storage;

    use super::*;

    #[test]
    fn item_state_survives_metadata_upsert() {
        let storage = Storage::in_memory().expect("storage");
        let mut config = AppConfig::default_with_pat("token".to_owned());
        config.host.kind = HostKind::Ghes;
        config.host.hostname = "ghe.example.test".to_owned();
        config.host.rest_api_base_path = "/api/v3/".to_owned();
        let host_id = storage.ensure_host(&config.host).expect("host");
        let query_id = storage
            .add_saved_query(host_id, "Mine", "assignee:@me", SortOrder::UpdatedDesc)
            .expect("query");

        let mut item = sample_item(host_id);
        let item_id = storage.upsert_stream_item(&item).expect("item");
        storage
            .record_saved_query_match(query_id, item_id, Some(0))
            .expect("match");
        storage.set_read_state(item_id, false).expect("read");
        storage.set_bookmarked(item_id, true).expect("bookmark");

        item.title = "Updated title".to_owned();
        storage.upsert_stream_item(&item).expect("updated item");

        let items = storage
            .list_items_for_saved_query(query_id, None, SortOrder::UpdatedDesc)
            .expect("items");

        assert_eq!(items[0].title, "Updated title");
        assert!(!items[0].is_unread);
        assert!(items[0].is_bookmarked);
    }

    #[test]
    fn archived_unread_items_are_excluded_from_query_badges() {
        let storage = Storage::in_memory().expect("storage");
        let config = AppConfig::default_with_pat("token".to_owned());
        let host_id = storage.ensure_host(&config.host).expect("host");
        let query_id = storage
            .add_saved_query(host_id, "Inbox", "is:open", SortOrder::UpdatedDesc)
            .expect("query");
        let item_id = storage
            .upsert_stream_item(&sample_item(host_id))
            .expect("item");
        storage
            .record_saved_query_match(query_id, item_id, None)
            .expect("match");
        storage.set_archived(item_id, true).expect("archive");

        let queries = storage.list_saved_queries(host_id).expect("queries");
        let archived_items = storage
            .list_items_for_library(
                host_id,
                crate::models::LibraryView::Archived,
                Some(StreamFilter::Unread),
                SortOrder::UpdatedDesc,
            )
            .expect("archived");

        assert_eq!(queries[0].unread_count, 0);
        assert_eq!(archived_items.len(), 1);
        assert!(archived_items[0].is_unread);
    }

    fn sample_item(host_id: i64) -> StreamItemUpsert {
        StreamItemUpsert {
            host_id,
            node_id: Some("node".to_owned()),
            repository_owner: "owner".to_owned(),
            repository_name: "repo".to_owned(),
            number: 42,
            item_type: ItemType::PullRequest,
            title: "Title".to_owned(),
            author_login: Some("author".to_owned()),
            html_url: "https://github.example.test/owner/repo/pull/42".to_owned(),
            api_url: None,
            state: "open".to_owned(),
            is_draft: Some(false),
            is_merged: Some(false),
            review_status: Some("review_required".to_owned()),
            comment_count: 3,
            created_at_github: "2026-05-22T00:00:00+00:00".to_owned(),
            updated_at_github: "2026-05-23T00:00:00+00:00".to_owned(),
            closed_at_github: None,
            merged_at_github: None,
            labels: vec!["bug".to_owned()],
            assignees: vec!["dev".to_owned()],
        }
    }
}
