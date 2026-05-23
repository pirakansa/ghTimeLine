# Application Behavior Specification

This document describes the implemented v1 behavior that should remain stable as
the app evolves. Code and passing tests are the source of truth; this document is
a compact reference for maintainers and intentionally omits database table
definitions that can be read from the storage implementation.

## Product Scope

`ghStreamListner` is a Rust desktop app for tracking GitHub issues and pull
requests through saved GitHub Search queries.

Implemented v1 scope:

- One configured GitHub host
- GitHub.com and GitHub Enterprise Server host kinds
- Personal Access Token authentication
- YAML configuration for host, token, UI, sort, and polling settings
- SQLite-backed saved queries and stream item state
- REST Search discovery for issues and pull requests
- GraphQL enrichment for discovered pull requests
- Two-pane `egui` stream UI
- Manual refresh and automatic polling
- Local unread, bookmark, and archive state
- External browser opening for item details

Out of v1 scope:

- Multiple configured hosts
- OAuth or GitHub App authentication
- Team or cross-device state sync
- Server-side backend components
- Embedded item detail browser
- Search resource types beyond issues and pull requests

## Configuration Contract

The app stores settings in a plain text YAML file. The top-level keys are:

- `host`
- `auth`
- `ui`
- `refresh`

Example:

```yaml
host:
  name: "GitHub.com"
  scheme: "https"
  hostname: "api.github.com"
  rest_api_base_path: "/"
  kind: "github"
auth:
  pat: "ghp_example"
ui:
  theme: "system"
  accent_color: "#4F8CC9"
  default_sort: "updated_desc"
  font_size: "default"
refresh:
  polling_interval_seconds: 180
```

Configuration rules:

- `host.name` must be non-empty after trimming whitespace.
- `host.scheme` must be `https` or `http`.
- `host.hostname` must be non-empty and must not include a scheme, path, query,
  fragment, username, password, or port.
- `host.rest_api_base_path` is normalized to exactly one leading slash and one
  trailing slash.
- `host.kind` must be `github` or `ghes`.
- `host.kind: "github"` requires `host.hostname: "api.github.com"`.
- `auth.pat` must be non-empty and is stored as plain text in v1.
- `ui.theme` must be `light`, `dark`, or `system`.
- `ui.accent_color` must be a `#RRGGBB` hex color.
- `ui.default_sort` must be one of the supported sort values.
- `ui.font_size` must be `default`, `large`, or `x_large`.
- `refresh.polling_interval_seconds` must be from `15` through `3600`.

Unknown enum values are rejected by deserialization and validation. Unknown
object keys may be ignored by the YAML parser.

The effective REST API base URL is:

```text
{scheme}://{hostname}{rest_api_base_path}
```

The effective GraphQL URL is:

- GitHub.com: `{scheme}://api.github.com/graphql`
- GHES: `{scheme}://{hostname}/api/graphql`

## Local Persistence Boundary

The YAML configuration stores user-editable app settings and the Personal Access
Token.

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

## Saved Query Behavior

A saved query has:

- Host association
- User-visible name
- GitHub Search query string
- Sort order
- Enabled state
- Position
- Last successful sync timestamp
- Last sync error message

The UI supports creating, editing, deleting, listing, and selecting saved
queries. Query names and query strings must not be empty when created or updated.

Saved queries target GitHub Search for issues and pull requests.

Saved query unread badges count distinct matched items where local state is
unread and not archived.

## Stream View Behavior

The primary UI is a two-pane layout:

- Left pane: Library entries, saved queries, unread counts, query management
- Right pane: Toolbar and database-backed item list

Library entries:

- Inbox: distinct non-archived items across enabled saved queries
- Bookmark: distinct bookmarked, non-archived items across enabled saved queries
- Archived: distinct archived items across enabled saved queries

Saved query views show items matched to that query and exclude archived items by
default.

When the same issue or pull request matches multiple saved queries, aggregated
library views display it as a single item.

The right pane may show `0 items` when no stored item matches the selected view.
Refresh and network errors must not clear the previously stored list.

## Filters And Sorting

Filters:

- `Open`: issues with `state = open`; pull requests with `state = open` and not
  merged; draft pull requests are included when open and not merged
- `Unread`: local unread state
- `Bookmarked`: local bookmarked state

Sort values:

- `updated_desc`
- `updated_asc`
- `created_desc`
- `created_asc`
- `comments_desc`
- `comments_asc`

The default sort is `updated_desc`.

Library views use `ui.default_sort`. Saved query views use the saved query sort,
falling back to `ui.default_sort` when needed.

Changing the default sort writes the updated value to the YAML configuration.
Changing a saved query sort writes the updated value to the database.

## Item State Behavior

Unread, bookmark, and archive are local-only states.

The app supports:

- Marking an item read
- Marking an item unread
- Bookmarking and unbookmarking an item
- Archiving an item

These states persist across restarts and are not synced back to GitHub.

Refreshes may update GitHub-derived metadata, but must not overwrite local
unread, bookmark, or archive state.

Archived unread items retain unread state and appear as unread in the Archived
library view. Archived unread items are excluded from saved query unread badges.

## Refresh And API Strategy

Manual refresh:

- Refreshes the selected saved query when a saved query is selected.
- Refreshes all saved queries when a library entry is selected.

Automatic polling:

- Runs against enabled saved queries.
- Defaults to a 5 minute interval.
- Uses the interval stored in the YAML configuration.
- Persists interval changes to the YAML configuration.

API strategy:

- REST Search is the discovery source for issues and pull requests.
- REST Search results are parsed into normalized stream item data.
- Pull requests with node IDs are enriched through GraphQL.
- GraphQL enrichment fills draft state, merge state, merged timestamp, and
  review status when available.
- Failed GraphQL enrichment must not prevent REST Search data from being stored
  or rendered.

Refresh write flow:

1. Fetch REST Search results for a saved query.
2. Attempt GraphQL enrichment for discovered pull requests.
3. Upsert stream items and query matches into SQLite.
4. Mark query sync success or store a short sync error.
5. Reload the current view from SQLite.

The app must not assume polling can infer every GitHub state transition. Cached
matches may remain when an item stops appearing in a later search result.

## Error And Status Behavior

Configuration parse and validation errors surface in setup.

Authentication, network, API, and database failures are surfaced as user-visible
status messages. Error messages must not include the Personal Access Token.

The status bar is separate from the item list. Communication failures should not
replace the item list with a blocking error screen.

## Item Opening Behavior

The app opens item details in the system default web browser using the stored
HTML URL.

v1 does not include an in-app browser or embedded detail pane.

## Validation Coverage

The current test suite covers:

- Configuration normalization and validation
- PAT redaction from error messages
- REST Search response parsing
- GraphQL enrichment parsing and review status derivation
- Refresh write-before-render behavior
- Refresh failure preserving existing stored items
- Host initialization without storing the PAT
- Item state persistence across metadata upserts
- Archived unread badge behavior
- Saved query updates
- UI state/event handling
- `egui_kittest` component interactions for toolbar, left pane, and item list
