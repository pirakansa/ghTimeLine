use rusqlite::Connection;

use super::Result;

pub const SCHEMA_VERSION: i64 = 6;

pub fn migrate(connection: &Connection) -> Result<()> {
    let version =
        connection.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))?;
    if version == 0 {
        connection.execute_batch(V3_SCHEMA)?;
        connection.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    } else {
        if !column_exists(connection, "stream_items", "author_avatar_url")? {
            connection
                .execute_batch("ALTER TABLE stream_items ADD COLUMN author_avatar_url TEXT;")?;
        }
        if !column_exists(connection, "stream_item_assignees", "avatar_url")? {
            connection
                .execute_batch("ALTER TABLE stream_item_assignees ADD COLUMN avatar_url TEXT;")?;
        }
        connection.execute_batch(V3_INCREMENTAL_SCHEMA)?;
        if version < 6 {
            migrate_stream_sources(connection)?;
        }
        connection.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    }
    Ok(())
}

fn migrate_stream_sources(connection: &Connection) -> Result<()> {
    connection.pragma_update(None, "foreign_keys", "OFF")?;
    connection.pragma_update(None, "legacy_alter_table", "ON")?;
    let migration_result = connection.execute_batch(V6_STREAM_SOURCE_MIGRATION);
    connection.pragma_update(None, "legacy_alter_table", "OFF")?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    migration_result?;
    Ok(())
}

fn column_exists(connection: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        if row.get::<_, String>(1)? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

const V3_SCHEMA: &str = r#"
CREATE TABLE hosts (
    id INTEGER PRIMARY KEY,
    fingerprint TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('github', 'ghes')),
    scheme TEXT NOT NULL CHECK (scheme IN ('https', 'http')),
    hostname TEXT NOT NULL,
    rest_api_base_path TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE saved_queries (
    id INTEGER PRIMARY KEY,
    host_id INTEGER NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    query TEXT NOT NULL,
    resource_type TEXT NOT NULL DEFAULT 'issue_or_pull_request'
        CHECK (resource_type IN ('issue_or_pull_request', 'discussion')),
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_successful_sync_at TEXT,
    last_sync_error TEXT,
    UNIQUE (host_id, name)
);

CREATE TABLE stream_items (
    id INTEGER PRIMARY KEY,
    host_id INTEGER NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    node_id TEXT,
    repository_owner TEXT NOT NULL,
    repository_name TEXT NOT NULL,
    number INTEGER NOT NULL,
    item_type TEXT NOT NULL CHECK (item_type IN ('issue', 'pull_request', 'discussion')),
    title TEXT NOT NULL,
    author_login TEXT,
    author_avatar_url TEXT,
    html_url TEXT NOT NULL,
    api_url TEXT,
    state TEXT NOT NULL CHECK (state IN ('open', 'closed')),
    is_draft INTEGER CHECK (is_draft IN (0, 1)),
    is_merged INTEGER CHECK (is_merged IN (0, 1)),
    review_status TEXT CHECK (review_status IN (
        'none',
        'review_required',
        'changes_requested',
        'approved',
        'unknown'
    )),
    comment_count INTEGER NOT NULL DEFAULT 0,
    created_at_github TEXT NOT NULL,
    updated_at_github TEXT NOT NULL,
    closed_at_github TEXT,
    merged_at_github TEXT,
    last_seen_at TEXT NOT NULL,
    last_enriched_at TEXT,
    raw_search_json TEXT,
    raw_graphql_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (host_id, repository_owner, repository_name, number, item_type)
);

CREATE TABLE stream_item_labels (
    stream_item_id INTEGER NOT NULL REFERENCES stream_items(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    color TEXT,
    description TEXT,
    PRIMARY KEY (stream_item_id, name)
);

CREATE TABLE stream_item_assignees (
    stream_item_id INTEGER NOT NULL REFERENCES stream_items(id) ON DELETE CASCADE,
    login TEXT NOT NULL,
    avatar_url TEXT,
    PRIMARY KEY (stream_item_id, login)
);

CREATE TABLE stream_item_review_requests (
    stream_item_id INTEGER NOT NULL REFERENCES stream_items(id) ON DELETE CASCADE,
    login TEXT NOT NULL,
    avatar_url TEXT,
    PRIMARY KEY (stream_item_id, login)
);

CREATE TABLE stream_item_reviews (
    stream_item_id INTEGER NOT NULL REFERENCES stream_items(id) ON DELETE CASCADE,
    login TEXT NOT NULL,
    avatar_url TEXT,
    state TEXT NOT NULL CHECK (state IN ('approved', 'changes_requested', 'commented')),
    PRIMARY KEY (stream_item_id, login)
);

CREATE TABLE stream_item_participants (
    stream_item_id INTEGER NOT NULL REFERENCES stream_items(id) ON DELETE CASCADE,
    login TEXT NOT NULL,
    avatar_url TEXT,
    PRIMARY KEY (stream_item_id, login)
);

CREATE TABLE stream_item_mentions (
    stream_item_id INTEGER NOT NULL REFERENCES stream_items(id) ON DELETE CASCADE,
    login TEXT NOT NULL,
    PRIMARY KEY (stream_item_id, login)
);

CREATE TABLE saved_query_matches (
    saved_query_id INTEGER NOT NULL REFERENCES saved_queries(id) ON DELETE CASCADE,
    stream_item_id INTEGER NOT NULL REFERENCES stream_items(id) ON DELETE CASCADE,
    first_seen_at TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    search_rank INTEGER,
    PRIMARY KEY (saved_query_id, stream_item_id)
);

CREATE TABLE filter_streams (
    id INTEGER PRIMARY KEY,
    saved_query_id INTEGER NOT NULL REFERENCES saved_queries(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    filter_query TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (saved_query_id, name)
);

CREATE TABLE item_state (
    stream_item_id INTEGER PRIMARY KEY REFERENCES stream_items(id) ON DELETE CASCADE,
    is_unread INTEGER NOT NULL DEFAULT 1 CHECK (is_unread IN (0, 1)),
    is_bookmarked INTEGER NOT NULL DEFAULT 0 CHECK (is_bookmarked IN (0, 1)),
    is_archived INTEGER NOT NULL DEFAULT 0 CHECK (is_archived IN (0, 1)),
    read_at TEXT,
    unread_at TEXT,
    bookmarked_at TEXT,
    archived_at TEXT,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_saved_queries_host_position
    ON saved_queries(host_id, enabled, position, name);

CREATE INDEX idx_stream_items_host_updated
    ON stream_items(host_id, updated_at_github DESC);

CREATE INDEX idx_stream_items_host_repo_number
    ON stream_items(host_id, repository_owner, repository_name, number);

CREATE INDEX idx_saved_query_matches_item
    ON saved_query_matches(stream_item_id);

CREATE INDEX idx_filter_streams_parent_position
    ON filter_streams(saved_query_id, enabled, position, name);

CREATE INDEX idx_item_state_flags
    ON item_state(is_archived, is_unread, is_bookmarked);
"#;

const V3_INCREMENTAL_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS stream_item_review_requests (
    stream_item_id INTEGER NOT NULL REFERENCES stream_items(id) ON DELETE CASCADE,
    login TEXT NOT NULL,
    avatar_url TEXT,
    PRIMARY KEY (stream_item_id, login)
);

CREATE TABLE IF NOT EXISTS stream_item_reviews (
    stream_item_id INTEGER NOT NULL REFERENCES stream_items(id) ON DELETE CASCADE,
    login TEXT NOT NULL,
    avatar_url TEXT,
    state TEXT NOT NULL CHECK (state IN ('approved', 'changes_requested', 'commented')),
    PRIMARY KEY (stream_item_id, login)
);

CREATE TABLE IF NOT EXISTS stream_item_participants (
    stream_item_id INTEGER NOT NULL REFERENCES stream_items(id) ON DELETE CASCADE,
    login TEXT NOT NULL,
    avatar_url TEXT,
    PRIMARY KEY (stream_item_id, login)
);

CREATE TABLE IF NOT EXISTS stream_item_mentions (
    stream_item_id INTEGER NOT NULL REFERENCES stream_items(id) ON DELETE CASCADE,
    login TEXT NOT NULL,
    PRIMARY KEY (stream_item_id, login)
);

CREATE TABLE IF NOT EXISTS filter_streams (
    id INTEGER PRIMARY KEY,
    saved_query_id INTEGER NOT NULL REFERENCES saved_queries(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    filter_query TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (saved_query_id, name)
);

CREATE INDEX IF NOT EXISTS idx_filter_streams_parent_position
    ON filter_streams(saved_query_id, enabled, position, name);
"#;

const V6_STREAM_SOURCE_MIGRATION: &str = r#"
BEGIN IMMEDIATE;

ALTER TABLE saved_queries RENAME TO saved_queries_v5;
CREATE TABLE saved_queries (
    id INTEGER PRIMARY KEY,
    host_id INTEGER NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    query TEXT NOT NULL,
    resource_type TEXT NOT NULL DEFAULT 'issue_or_pull_request'
        CHECK (resource_type IN ('issue_or_pull_request', 'discussion')),
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_successful_sync_at TEXT,
    last_sync_error TEXT,
    UNIQUE (host_id, name)
);
INSERT INTO saved_queries SELECT * FROM saved_queries_v5;
DROP TABLE saved_queries_v5;
CREATE INDEX idx_saved_queries_host_position
    ON saved_queries(host_id, enabled, position, name);

ALTER TABLE stream_items RENAME TO stream_items_v5;
CREATE TABLE stream_items (
    id INTEGER PRIMARY KEY,
    host_id INTEGER NOT NULL REFERENCES hosts(id) ON DELETE CASCADE,
    node_id TEXT,
    repository_owner TEXT NOT NULL,
    repository_name TEXT NOT NULL,
    number INTEGER NOT NULL,
    item_type TEXT NOT NULL CHECK (item_type IN ('issue', 'pull_request', 'discussion')),
    title TEXT NOT NULL,
    author_login TEXT,
    author_avatar_url TEXT,
    html_url TEXT NOT NULL,
    api_url TEXT,
    state TEXT NOT NULL CHECK (state IN ('open', 'closed')),
    is_draft INTEGER CHECK (is_draft IN (0, 1)),
    is_merged INTEGER CHECK (is_merged IN (0, 1)),
    review_status TEXT CHECK (review_status IN (
        'none',
        'review_required',
        'changes_requested',
        'approved',
        'unknown'
    )),
    comment_count INTEGER NOT NULL DEFAULT 0,
    created_at_github TEXT NOT NULL,
    updated_at_github TEXT NOT NULL,
    closed_at_github TEXT,
    merged_at_github TEXT,
    last_seen_at TEXT NOT NULL,
    last_enriched_at TEXT,
    raw_search_json TEXT,
    raw_graphql_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (host_id, repository_owner, repository_name, number, item_type)
);
INSERT INTO stream_items SELECT * FROM stream_items_v5;
DROP TABLE stream_items_v5;
CREATE INDEX idx_stream_items_host_updated
    ON stream_items(host_id, updated_at_github DESC);
CREATE INDEX idx_stream_items_host_repo_number
    ON stream_items(host_id, repository_owner, repository_name, number);

COMMIT;
"#;

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::{migrate, V3_SCHEMA};

    #[test]
    fn v5_schema_migrates_existing_rows_and_accepts_discussion_types() {
        let connection = Connection::open_in_memory().expect("connection");
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .expect("foreign keys");
        let v5_schema = V3_SCHEMA.replace(", 'discussion'", "");
        connection.execute_batch(&v5_schema).expect("v5 schema");
        connection
            .pragma_update(None, "user_version", 5)
            .expect("version");
        connection
            .execute_batch(
                "INSERT INTO hosts (
                    id, fingerprint, name, kind, scheme, hostname, rest_api_base_path,
                    created_at, updated_at
                 ) VALUES (
                    1, 'github|https|api.github.com|/', 'GitHub.com', 'github', 'https',
                    'api.github.com', '/', '2026-05-27T00:00:00Z', '2026-05-27T00:00:00Z'
                 );
                 INSERT INTO saved_queries (
                    id, host_id, name, query, resource_type, enabled, position, created_at, updated_at
                 ) VALUES (
                    1, 1, 'Issues', 'is:issue', 'issue_or_pull_request', 1, 0,
                    '2026-05-27T00:00:00Z', '2026-05-27T00:00:00Z'
                 );
                 INSERT INTO filter_streams (
                    id, saved_query_id, name, filter_query, enabled, position, created_at, updated_at
                 ) VALUES (
                    1, 1, 'Open', 'is:open', 1, 0,
                    '2026-05-27T00:00:00Z', '2026-05-27T00:00:00Z'
                 );",
            )
            .expect("existing rows");

        migrate(&connection).expect("migration");
        connection
            .execute(
                "INSERT INTO saved_queries (
                    host_id, name, query, resource_type, enabled, position, created_at, updated_at
                 ) VALUES (1, 'Discussions', 'feedback', 'discussion', 1, 1, ?1, ?1)",
                ["2026-05-27T00:00:00Z"],
            )
            .expect("discussion source");
        connection
            .execute(
                "INSERT INTO stream_items (
                    host_id, repository_owner, repository_name, number, item_type, title,
                    html_url, state, created_at_github, updated_at_github, last_seen_at,
                    created_at, updated_at
                 ) VALUES (
                    1, 'acme', 'project', 12, 'discussion', 'Feedback',
                    'https://github.com/acme/project/discussions/12', 'open', ?1, ?1, ?1, ?1, ?1
                 )",
                ["2026-05-27T00:00:00Z"],
            )
            .expect("discussion item");
        let filter_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM filter_streams", [], |row| row.get(0))
            .expect("filter count");
        let foreign_key_violations: i64 = connection
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .expect("foreign key check");

        assert_eq!(filter_count, 1);
        assert_eq!(foreign_key_violations, 0);
    }
}
