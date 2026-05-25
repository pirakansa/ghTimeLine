# Persistence

This reference defines the local persistence boundary and host identity behavior.

## Local Persistence Boundary

The YAML configuration stores user-editable app settings and the Personal Access
Token.

Saved query import/export uses a separate YAML document for transferring saved
query definitions between databases or machines.

The local SQLite database stores:

- Host identity metadata
- Saved query definitions
- Materialized stream items
- Query-to-item matches
- Local unread, bookmark, and archive state
- Cached GitHub-derived metadata used for rendering

The database must not store the Personal Access Token.

## Host Behavior

v1 supports exactly one active configured host.

The storage layer keeps a stable local host identity based on host kind, scheme,
hostname, and normalized REST API base path. This lets saved queries and stream
items survive edits to the host display name.

The code structure should continue to leave room for multiple hosts later, but
current UI behavior does not expose cross-host query management.
