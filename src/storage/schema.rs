use rusqlite::Connection;

use super::Result;

pub const SCHEMA_VERSION: i64 = 3;

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
        connection.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    }
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
        CHECK (resource_type IN ('issue_or_pull_request')),
    -- Retained for compatibility with databases created before sorting became view-local.
    sort TEXT NOT NULL DEFAULT 'updated_desc'
        CHECK (sort IN (
            'updated_desc',
            'updated_asc',
            'created_desc',
            'created_asc',
            'comments_desc',
            'comments_asc'
        )),
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
    item_type TEXT NOT NULL CHECK (item_type IN ('issue', 'pull_request')),
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

CREATE TABLE saved_query_matches (
    saved_query_id INTEGER NOT NULL REFERENCES saved_queries(id) ON DELETE CASCADE,
    stream_item_id INTEGER NOT NULL REFERENCES stream_items(id) ON DELETE CASCADE,
    first_seen_at TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    search_rank INTEGER,
    PRIMARY KEY (saved_query_id, stream_item_id)
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
"#;
