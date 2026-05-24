# Stream View

This reference defines stream view behavior, filters, sorting, local item state,
and item opening.

## Layout

The primary UI is a two-pane layout:

- Left pane: Library entries, saved queries, unread counts, and access to query
  management
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

## Item State

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

## Item Opening

The app opens item details in the system default web browser using the stored
HTML URL.

v1 does not include an in-app browser or embedded detail pane.
