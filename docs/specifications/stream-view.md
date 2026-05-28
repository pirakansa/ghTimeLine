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

Filter stream views are local child views under a saved query. They reuse the
parent saved query's cached matches and apply an additional SQLite-backed local
filter without performing a new GitHub search.

When the same issue, pull request, or discussion matches multiple saved
queries, aggregated library views display it as a single item.

The right pane may show `0 items` when no stored item matches the selected view.
Refresh and network errors must not clear the previously stored list.
Changing the selected library entry or saved query resets the item list scroll
position to the top.

Remote changes discovered by either manual refresh or automatic polling must
not reorder or replace the currently displayed item list without user action.
When stored remote changes affect that view, the right pane displays an update
banner. Selecting **Show updates** reloads the item list from SQLite and clears
the pending update banner. Local item actions remain immediately visible.

## Filters And Sorting

Filters:

- `Open`: issues with `state = open`; pull requests with `state = open` and not
  merged; and retrieved discussions; draft pull requests are included when
  open and not merged
- `Unread`: local unread state
- `Bookmarked`: local bookmarked state

The toolbar also supports a temporary local filter query that narrows the
currently displayed SQLite-backed list without issuing new GitHub API requests.
Supported local filter terms are:

- `author:<login>`
- `assignee:<login>`
- `draft:true` or `draft:false`
- `involves:<login>`
- `is:issue`, `is:pr`, `is:discussion`, `is:open`, `is:closed`, or `is:merged`
- `label:<name>`
- `org:<owner>`
- `repo:<owner/name>`
- `review-requested:<login>`
- `reviewed-by:<login>`
- `user:<owner>`

`involves:` matches against locally stored author, assignee, review-requested,
reviewed-by, participant, commenter, and parsed `@mention` metadata.

Local filter terms combine as follows:

- Different filter keys use `AND`
- Repeated values of the same key use `OR`
- Repeated `is:` terms combine by category (`type`, `state`, `draft`) so
  `is:pr is:open` narrows to open pull requests
- Repeated `label:` terms use `AND`

Unsupported or malformed local filter terms must be rejected with a user-visible
error, and must not replace the previously active local filter.

Selecting a person avatar in an item card appends the matching term to the
temporary local filter input unless it is already present: author avatars use
`author:`, assignee avatars use `assignee:`, requested-reviewer avatars use
`review-requested:`, and completed-review avatars use `reviewed-by:`.

Selecting the repository reference in an item header appends
`repo:<owner/name>`. Selecting its type icon appends `is:issue` for issues,
`is:pr` for pull requests, or `is:discussion` for discussions. These controls
must not activate the draft filter; only applying the local filter input
changes the displayed items.

Sort values:

- `updated_desc`
- `created_desc`
- `read_desc`
- `closed_desc`
- `merged_desc`

The default sort is `updated_desc`.

Library views, saved query views, and filter stream views use `ui.default_sort`,
which is controlled from the stream toolbar. This value controls local item list
ordering only; refresh discovery uses updated descending order independently.

Changing the default sort writes the updated value to the YAML configuration.

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

Library entries and saved queries with unread items provide **Mark all as read**
from their context menus. For Library entries, the action marks unread items
across enabled saved queries within the selected library scope. **Inbox** and
**Bookmark** exclude archived items; **Archived** marks archived unread items.

Filter streams also provide **Mark all as read** for unread, non-archived items
visible within that filter stream.

## Preferences And Status

The **Preferences** menu provides:

- Host settings
- Theme selection (`system`, `light`, `dark`)
- Font size selection (`x_small`, `small`, `default`, `large`, `x_large`)
- Polling interval editing with an allowed range of 15 through 3600 seconds

The bottom status bar shows the latest info or error status without replacing
the current item list. Selecting the status indicator opens a status log screen
that keeps recent messages in reverse chronological order.

## Item Opening

The app opens item details in the system default web browser using the stored
HTML URL.

v1 does not include an in-app browser or embedded detail pane.
