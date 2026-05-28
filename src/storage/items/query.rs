use rusqlite::types::Value;

use super::{item_type_from_db, local_filter, STREAM_VIEW_LIMIT};
use crate::models::{LibraryView, Selection, SortOrder, StreamFilter, StreamItem};
use crate::storage::{Result, Storage};

#[derive(Clone, Copy)]
struct ItemListOptions<'a> {
    filter: Option<StreamFilter>,
    local_filter_query: Option<&'a str>,
    sort: SortOrder,
}

impl Storage {
    pub fn list_items_for_saved_query(
        &self,
        saved_query_id: i64,
        filter: Option<StreamFilter>,
        local_filter_query: Option<&str>,
        sort: SortOrder,
    ) -> Result<Vec<StreamItem>> {
        let mut params = vec![Value::Integer(saved_query_id)];
        let options = ItemListOptions {
            filter,
            local_filter_query,
            sort,
        };
        let mut sql = item_select_sql(
            "JOIN saved_query_matches m ON m.stream_item_id = i.id",
            "m.saved_query_id = ? AND s.is_archived = 0",
            "",
            options,
            &mut params,
        )?;
        sql.push_str(&format!(" LIMIT {STREAM_VIEW_LIMIT}"));
        self.query_items(&sql, rusqlite::params_from_iter(params))
    }

    pub fn list_items_for_library(
        &self,
        host_id: i64,
        library: LibraryView,
        filter: Option<StreamFilter>,
        local_filter_query: Option<&str>,
        sort: SortOrder,
    ) -> Result<Vec<StreamItem>> {
        let mut params = vec![Value::Integer(host_id)];
        let options = ItemListOptions {
            filter,
            local_filter_query,
            sort,
        };
        let where_clause = library_where_clause(library);
        let mut sql = item_select_sql("", &where_clause, "", options, &mut params)?;
        sql.push_str(&format!(" LIMIT {STREAM_VIEW_LIMIT}"));
        self.query_items(&sql, rusqlite::params_from_iter(params))
    }

    pub fn list_items_for_selection_by_ids(
        &self,
        host_id: i64,
        selection: &Selection,
        filter: Option<StreamFilter>,
        local_filter_query: Option<&str>,
        sort: SortOrder,
        item_ids: &[i64],
    ) -> Result<Vec<StreamItem>> {
        let options = ItemListOptions {
            filter,
            local_filter_query,
            sort,
        };
        match selection {
            Selection::Library(library) => {
                self.list_items_for_library_by_ids(host_id, *library, options, item_ids)
            }
            Selection::SavedQuery(id) => {
                self.list_items_for_saved_query_by_ids(*id, options, item_ids)
            }
            Selection::FilterStream(id) => {
                self.list_items_for_filter_stream_by_ids(*id, options, item_ids)
            }
        }
    }

    pub fn validate_local_filter(&self, query: Option<&str>) -> Result<()> {
        let _ = local_filter::compile(query)?;
        Ok(())
    }

    pub fn list_items_for_filter_stream(
        &self,
        filter_stream_id: i64,
        filter: Option<StreamFilter>,
        local_filter_query: Option<&str>,
        sort: SortOrder,
    ) -> Result<Vec<StreamItem>> {
        let Some(filter_stream) = self.get_filter_stream(filter_stream_id)? else {
            return Ok(Vec::new());
        };

        self.list_items_for_saved_query(
            filter_stream.saved_query_id,
            filter,
            combine_local_filters(
                Some(filter_stream.filter_query.as_str()),
                local_filter_query,
            )
            .as_deref(),
            sort,
        )
    }

    pub fn count_items_for_saved_query(
        &self,
        saved_query_id: i64,
        filter: Option<StreamFilter>,
        local_filter_query: Option<&str>,
        extra_local_filter_query: Option<&str>,
    ) -> Result<i64> {
        let mut params = vec![Value::Integer(saved_query_id)];
        let combined_local_filter =
            combine_local_filters(local_filter_query, extra_local_filter_query);
        let options = ItemListOptions {
            filter,
            local_filter_query: combined_local_filter.as_deref(),
            sort: SortOrder::UpdatedDesc,
        };
        let sql = item_count_sql(
            "JOIN saved_query_matches m ON m.stream_item_id = i.id",
            "m.saved_query_id = ? AND s.is_archived = 0",
            options,
            &mut params,
        )?;
        self.connection()
            .query_row(&sql, rusqlite::params_from_iter(params), |row| {
                row.get::<_, i64>(0)
            })
            .map_err(Into::into)
    }

    pub fn list_item_ids_for_saved_query(
        &self,
        saved_query_id: i64,
        filter: Option<StreamFilter>,
        local_filter_query: Option<&str>,
    ) -> Result<Vec<i64>> {
        let mut params = vec![Value::Integer(saved_query_id)];
        let sql = item_ids_sql(
            "JOIN saved_query_matches m ON m.stream_item_id = i.id",
            "m.saved_query_id = ? AND s.is_archived = 0",
            ItemListOptions {
                filter,
                local_filter_query,
                sort: SortOrder::UpdatedDesc,
            },
            &mut params,
        )?;
        self.query_item_ids(&sql, params)
    }

    pub fn count_items_for_filter_stream(
        &self,
        filter_stream_id: i64,
        filter: Option<StreamFilter>,
    ) -> Result<i64> {
        let Some(filter_stream) = self.get_filter_stream(filter_stream_id)? else {
            return Ok(0);
        };
        self.count_items_for_saved_query(
            filter_stream.saved_query_id,
            filter,
            Some(&filter_stream.filter_query),
            None,
        )
    }

    pub fn list_item_ids_for_filter_stream(
        &self,
        filter_stream_id: i64,
        filter: Option<StreamFilter>,
    ) -> Result<Vec<i64>> {
        let Some(filter_stream) = self.get_filter_stream(filter_stream_id)? else {
            return Ok(Vec::new());
        };
        self.list_item_ids_for_saved_query(
            filter_stream.saved_query_id,
            filter,
            Some(&filter_stream.filter_query),
        )
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
                author_avatar_url: row.get(7)?,
                html_url: row.get(8)?,
                state: row.get(9)?,
                is_draft: row.get::<_, Option<i64>>(10)?.map(|value| value == 1),
                is_merged: row.get::<_, Option<i64>>(11)?.map(|value| value == 1),
                review_status: row.get(12)?,
                comment_count: row.get(13)?,
                created_at_github: row.get(14)?,
                updated_at_github: row.get(15)?,
                closed_at_github: row.get(16)?,
                merged_at_github: row.get(17)?,
                read_at: row.get(18)?,
                labels: Vec::new(),
                assignees: Vec::new(),
                review_requests: Vec::new(),
                reviewers: Vec::new(),
                is_unread: row.get::<_, i64>(19)? == 1,
                is_bookmarked: row.get::<_, i64>(20)? == 1,
                is_archived: row.get::<_, i64>(21)? == 1,
            })
        })?;
        let mut items = rows
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(crate::storage::StorageError::from)?;
        drop(statement);

        self.hydrate_item_relations(&mut items)?;

        Ok(items)
    }

    fn query_item_ids(&self, sql: &str, params: Vec<Value>) -> Result<Vec<i64>> {
        let mut statement = self.connection().prepare(sql)?;
        let rows = statement.query_map(rusqlite::params_from_iter(params), |row| {
            row.get::<_, i64>(0)
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    fn hydrate_item_relations(&self, items: &mut [StreamItem]) -> Result<()> {
        let item_ids = items.iter().map(|item| item.id).collect::<Vec<_>>();
        let mut labels = self.list_labels_by_item_id(&item_ids)?;
        let mut assignees = self.list_assignees_by_item_id(&item_ids)?;
        let mut review_requests = self.list_review_requests_by_item_id(&item_ids)?;
        let mut reviews = self.list_reviews_by_item_id(&item_ids)?;

        for item in items {
            item.labels = labels.remove(&item.id).unwrap_or_default();
            item.assignees = assignees.remove(&item.id).unwrap_or_default();
            item.review_requests = review_requests.remove(&item.id).unwrap_or_default();
            item.reviewers = reviews.remove(&item.id).unwrap_or_default();
        }

        Ok(())
    }

    fn list_items_for_saved_query_by_ids(
        &self,
        saved_query_id: i64,
        options: ItemListOptions<'_>,
        item_ids: &[i64],
    ) -> Result<Vec<StreamItem>> {
        self.query_items_by_ids(
            "JOIN saved_query_matches m ON m.stream_item_id = i.id",
            "m.saved_query_id = ? AND s.is_archived = 0",
            vec![Value::Integer(saved_query_id)],
            item_ids,
            options,
        )
    }

    fn list_items_for_library_by_ids(
        &self,
        host_id: i64,
        library: LibraryView,
        options: ItemListOptions<'_>,
        item_ids: &[i64],
    ) -> Result<Vec<StreamItem>> {
        let where_clause = library_where_clause(library);
        self.query_items_by_ids(
            "",
            &where_clause,
            vec![Value::Integer(host_id)],
            item_ids,
            options,
        )
    }

    fn list_items_for_filter_stream_by_ids(
        &self,
        filter_stream_id: i64,
        options: ItemListOptions<'_>,
        item_ids: &[i64],
    ) -> Result<Vec<StreamItem>> {
        let Some(filter_stream) = self.get_filter_stream(filter_stream_id)? else {
            return Ok(Vec::new());
        };
        let combined_local_filter = combine_local_filters(
            Some(filter_stream.filter_query.as_str()),
            options.local_filter_query,
        );
        self.list_items_for_saved_query_by_ids(
            filter_stream.saved_query_id,
            ItemListOptions {
                filter: options.filter,
                local_filter_query: combined_local_filter.as_deref(),
                sort: options.sort,
            },
            item_ids,
        )
    }

    fn query_items_by_ids(
        &self,
        extra_join: &str,
        base_where: &str,
        mut base_params: Vec<Value>,
        item_ids: &[i64],
        options: ItemListOptions<'_>,
    ) -> Result<Vec<StreamItem>> {
        if item_ids.is_empty() {
            return Ok(Vec::new());
        }

        let placeholders = vec!["?"; item_ids.len()].join(", ");
        let extra_where = format!(" AND i.id IN ({placeholders})");
        let sql = item_select_sql(
            extra_join,
            base_where,
            &extra_where,
            options,
            &mut base_params,
        )?;
        base_params.extend(item_ids.iter().copied().map(Value::Integer));
        self.query_items(&sql, rusqlite::params_from_iter(base_params))
    }
}

fn library_where_clause(library: LibraryView) -> String {
    let library_clause = match library {
        LibraryView::Inbox => "s.is_archived = 0",
        LibraryView::Bookmark => "s.is_bookmarked = 1 AND s.is_archived = 0",
        LibraryView::Archived => "s.is_archived = 1",
    };
    format!(
        "i.host_id = ?1 AND EXISTS (
            SELECT 1 FROM saved_query_matches m
            JOIN saved_queries q ON q.id = m.saved_query_id
            WHERE m.stream_item_id = i.id AND q.enabled = 1
         ) AND {library_clause}"
    )
}

fn item_select_sql(
    extra_join: &str,
    base_where: &str,
    extra_where: &str,
    options: ItemListOptions<'_>,
    params: &mut Vec<Value>,
) -> Result<String> {
    let where_sql = item_where_sql(base_where, extra_where, options, params)?;
    let order = match options.sort {
        SortOrder::UpdatedDesc => "i.updated_at_github DESC",
        SortOrder::CreatedDesc => "i.created_at_github DESC",
        SortOrder::ReadDesc => "s.read_at IS NULL ASC, s.read_at DESC, i.updated_at_github DESC",
        SortOrder::ClosedDesc => {
            "i.closed_at_github IS NULL ASC, i.closed_at_github DESC, i.updated_at_github DESC"
        }
        SortOrder::MergedDesc => {
            "i.merged_at_github IS NULL ASC, i.merged_at_github DESC, i.updated_at_github DESC"
        }
    };
    Ok(format!(
        "SELECT
            i.id,
            i.repository_owner,
            i.repository_name,
            i.number,
            i.item_type,
            i.title,
            i.author_login,
            i.author_avatar_url,
            i.html_url,
            i.state,
            i.is_draft,
            i.is_merged,
            i.review_status,
            i.comment_count,
            i.created_at_github,
            i.updated_at_github,
            i.closed_at_github,
            i.merged_at_github,
            s.read_at,
            s.is_unread,
            s.is_bookmarked,
            s.is_archived
         FROM stream_items i
         JOIN item_state s ON s.stream_item_id = i.id
         {extra_join}
         WHERE {where_sql}
         GROUP BY i.id
         ORDER BY {order}"
    ))
}

fn item_count_sql(
    extra_join: &str,
    base_where: &str,
    options: ItemListOptions<'_>,
    params: &mut Vec<Value>,
) -> Result<String> {
    let where_sql = item_where_sql(base_where, "", options, params)?;
    Ok(format!(
        "SELECT COUNT(DISTINCT i.id)
         FROM stream_items i
         JOIN item_state s ON s.stream_item_id = i.id
         {extra_join}
         WHERE {where_sql}"
    ))
}

fn item_ids_sql(
    extra_join: &str,
    base_where: &str,
    options: ItemListOptions<'_>,
    params: &mut Vec<Value>,
) -> Result<String> {
    let where_sql = item_where_sql(base_where, "", options, params)?;
    Ok(format!(
        "SELECT DISTINCT i.id
         FROM stream_items i
         JOIN item_state s ON s.stream_item_id = i.id
         {extra_join}
         WHERE {where_sql}
         ORDER BY i.id ASC"
    ))
}

fn item_where_sql(
    base_where: &str,
    extra_where: &str,
    options: ItemListOptions<'_>,
    params: &mut Vec<Value>,
) -> Result<String> {
    let filter_clause = match options.filter {
        Some(StreamFilter::Open) => {
            " AND (i.item_type = 'issue' AND i.state = 'open'
               OR i.item_type = 'pull_request' AND i.state = 'open' AND COALESCE(i.is_merged, 0) = 0
               OR i.item_type = 'discussion' AND i.state = 'open')"
        }
        Some(StreamFilter::Unread) => " AND s.is_unread = 1",
        Some(StreamFilter::Bookmarked) => " AND s.is_bookmarked = 1",
        None => "",
    };
    let local_filter_clause = local_filter::compile(options.local_filter_query)?
        .map(|compiled| {
            params.extend(compiled.params);
            format!(" AND ({})", compiled.clause)
        })
        .unwrap_or_default();
    Ok(format!(
        "{base_where}{extra_where}{filter_clause}{local_filter_clause}"
    ))
}

fn combine_local_filters(primary: Option<&str>, secondary: Option<&str>) -> Option<String> {
    let primary = primary.map(str::trim).filter(|value| !value.is_empty());
    let secondary = secondary.map(str::trim).filter(|value| !value.is_empty());
    match (primary, secondary) {
        (Some(primary), Some(secondary)) => Some(format!("{primary} {secondary}")),
        (Some(primary), None) => Some(primary.to_owned()),
        (None, Some(secondary)) => Some(secondary.to_owned()),
        (None, None) => None,
    }
}
