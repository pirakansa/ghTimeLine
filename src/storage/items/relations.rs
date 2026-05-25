use std::collections::HashMap;

use rusqlite::{params, types::Value};

use crate::models::{ItemPerson, ItemReview};
use crate::storage::{Result, Storage};

impl Storage {
    pub(super) fn replace_labels(&self, stream_item_id: i64, labels: &[String]) -> Result<()> {
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

    pub(super) fn replace_assignees(
        &self,
        stream_item_id: i64,
        assignees: &[ItemPerson],
    ) -> Result<()> {
        self.connection().execute(
            "DELETE FROM stream_item_assignees WHERE stream_item_id = ?1",
            params![stream_item_id],
        )?;
        for assignee in assignees {
            self.connection().execute(
                "INSERT INTO stream_item_assignees (stream_item_id, login, avatar_url)
                 VALUES (?1, ?2, ?3)",
                params![stream_item_id, assignee.login, assignee.avatar_url],
            )?;
        }
        Ok(())
    }

    pub(super) fn replace_review_requests(
        &self,
        stream_item_id: i64,
        review_requests: &[ItemPerson],
    ) -> Result<()> {
        self.connection().execute(
            "DELETE FROM stream_item_review_requests WHERE stream_item_id = ?1",
            params![stream_item_id],
        )?;
        for reviewer in review_requests {
            self.connection().execute(
                "INSERT INTO stream_item_review_requests (stream_item_id, login, avatar_url)
                 VALUES (?1, ?2, ?3)",
                params![stream_item_id, reviewer.login, reviewer.avatar_url],
            )?;
        }
        Ok(())
    }

    pub(super) fn replace_reviews(
        &self,
        stream_item_id: i64,
        reviewers: &[ItemReview],
    ) -> Result<()> {
        self.connection().execute(
            "DELETE FROM stream_item_reviews WHERE stream_item_id = ?1",
            params![stream_item_id],
        )?;
        for reviewer in reviewers {
            self.connection().execute(
                "INSERT INTO stream_item_reviews (stream_item_id, login, avatar_url, state)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    stream_item_id,
                    reviewer.login,
                    reviewer.avatar_url,
                    reviewer.state
                ],
            )?;
        }
        Ok(())
    }

    pub(super) fn list_labels_by_item_id(
        &self,
        item_ids: &[i64],
    ) -> Result<HashMap<i64, Vec<String>>> {
        if item_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let placeholders = placeholders(item_ids.len());
        let sql = format!(
            "SELECT stream_item_id, name
             FROM stream_item_labels
             WHERE stream_item_id IN ({placeholders})
             ORDER BY stream_item_id ASC, name COLLATE NOCASE ASC"
        );
        let mut statement = self.connection().prepare(&sql)?;
        let rows = statement.query_map(
            rusqlite::params_from_iter(params_from_ids(item_ids)),
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )?;

        let mut labels = HashMap::<i64, Vec<String>>::new();
        for row in rows {
            let (item_id, label) = row?;
            labels.entry(item_id).or_default().push(label);
        }
        Ok(labels)
    }

    pub(super) fn list_assignees_by_item_id(
        &self,
        item_ids: &[i64],
    ) -> Result<HashMap<i64, Vec<ItemPerson>>> {
        self.list_people_by_item_id(item_ids, "stream_item_assignees")
    }

    pub(super) fn list_review_requests_by_item_id(
        &self,
        item_ids: &[i64],
    ) -> Result<HashMap<i64, Vec<ItemPerson>>> {
        self.list_people_by_item_id(item_ids, "stream_item_review_requests")
    }

    fn list_people_by_item_id(
        &self,
        item_ids: &[i64],
        table: &str,
    ) -> Result<HashMap<i64, Vec<ItemPerson>>> {
        if item_ids.is_empty() {
            return Ok(HashMap::new());
        }

        debug_assert!(
            matches!(
                table,
                "stream_item_assignees" | "stream_item_review_requests"
            ),
            "unexpected people relation table: {table}"
        );
        let placeholders = placeholders(item_ids.len());
        let sql = format!(
            "SELECT stream_item_id, login, avatar_url
             FROM {table}
             WHERE stream_item_id IN ({placeholders})
             ORDER BY stream_item_id ASC, login COLLATE NOCASE ASC"
        );
        let mut statement = self.connection().prepare(&sql)?;
        let rows = statement.query_map(
            rusqlite::params_from_iter(params_from_ids(item_ids)),
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    ItemPerson {
                        login: row.get(1)?,
                        avatar_url: row.get(2)?,
                    },
                ))
            },
        )?;

        let mut people = HashMap::<i64, Vec<ItemPerson>>::new();
        for row in rows {
            let (item_id, person) = row?;
            people.entry(item_id).or_default().push(person);
        }
        Ok(people)
    }

    pub(super) fn list_reviews_by_item_id(
        &self,
        item_ids: &[i64],
    ) -> Result<HashMap<i64, Vec<ItemReview>>> {
        if item_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let placeholders = placeholders(item_ids.len());
        let sql = format!(
            "SELECT stream_item_id, login, avatar_url, state
             FROM stream_item_reviews
             WHERE stream_item_id IN ({placeholders})
             ORDER BY
                 stream_item_id ASC,
                 CASE state
                     WHEN 'changes_requested' THEN 0
                     WHEN 'approved' THEN 1
                     WHEN 'commented' THEN 2
                     ELSE 3
                 END,
                 login COLLATE NOCASE ASC"
        );
        let mut statement = self.connection().prepare(&sql)?;
        let rows = statement.query_map(
            rusqlite::params_from_iter(params_from_ids(item_ids)),
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    ItemReview {
                        login: row.get(1)?,
                        avatar_url: row.get(2)?,
                        state: row.get(3)?,
                    },
                ))
            },
        )?;

        let mut reviews = HashMap::<i64, Vec<ItemReview>>::new();
        for row in rows {
            let (item_id, review) = row?;
            reviews.entry(item_id).or_default().push(review);
        }
        Ok(reviews)
    }
}

fn placeholders(count: usize) -> String {
    vec!["?"; count].join(", ")
}

fn params_from_ids(item_ids: &[i64]) -> Vec<Value> {
    item_ids.iter().copied().map(Value::Integer).collect()
}
