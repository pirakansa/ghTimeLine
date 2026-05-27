# Stream View

This guide explains the main stream window, filters, sorting, item actions, and
status messages.

## Main Window

The main window uses two panes:

- The left pane contains library entries and saved queries.
- The right pane contains the item list for the selected library entry or saved
  query.

The library section contains:

- **Inbox**: non-archived items across enabled saved queries
- **Bookmark**: bookmarked, non-archived items across enabled saved queries
- **Archived**: archived items across enabled saved queries

Saved queries can also contain **filter streams**. A filter stream reuses the
parent saved query's locally cached items and adds another local filter layer,
so opening it does not create extra GitHub traffic by itself.

Each saved query entry shows its unread count. Archived unread items are not
included in saved query unread counts.

Right-click a Library entry or saved query with unread items and select
**Mark all as read** to clear its unread count. For Library entries, this
applies across enabled saved queries: **Inbox** and **Bookmark** affect
non-archived items, while **Archived** affects archived items.

## Filtering And Sorting

The toolbar supports these filters:

- **All**: no additional filter
- **Open**: open issues and open, unmerged pull requests
- **Unread**: unread items
- **Bookmarked**: bookmarked items

The toolbar also includes a temporary **Local filter** field. It narrows the
currently visible list from the local database only, so applying it does not
trigger a GitHub refresh.

Supported local filter terms:

- `author:<login>`
- `assignee:<login>`
- `draft:true` or `draft:false`
- `involves:<login>`
- `is:issue`, `is:pr`, `is:open`, `is:closed`, or `is:merged`
- `label:<name>`
- `org:<owner>`
- `repo:<owner/name>`
- `review-requested:<login>`
- `reviewed-by:<login>`
- `user:<owner>`

`involves:` is broader than the other people filters. It matches authors,
assignees, requested reviewers, reviewers, stored participants/commenters, and
users mentioned in the fetched body or comment text.

You can combine different keys with spaces. Different keys are matched together,
repeated values of the same key are matched as alternatives, repeated `is:`
terms narrow by category such as `is:pr is:open`, and repeated `label:` terms
must all be present. Use **Apply** to activate the filter and **Clear** to
remove it.

Select an author or assignee avatar in an item card to add its matching local
filter term. Selecting an avatar whose term is already active does not add a
duplicate term.

Filter streams use the same local filter syntax, but they persist under a saved
query in the left pane and saved query manager.

The sort selector supports:

- Updated descending
- Updated ascending
- Created descending
- Created ascending
- Comments descending
- Comments ascending

Library views and saved query views use the toolbar sort, which is saved as the
default sort in the YAML configuration. Sorting changes the stored item list
display; saved query refreshes always retrieve recently updated GitHub items
first.

When either manual refresh or automatic polling finds remote changes that
affect the currently visible list, the app keeps your current reading position
and displays an update banner above the list. Select **Show updates** to load
the new ordering and contents. Actions you make directly on an item, such as
marking it read or archiving it, remain immediately visible.

## Item Actions

Each list item shows the repository, item type, number, title, update time,
unread state, the author avatar, assignee avatars, reviewer avatars and review
status when available, comment count, state, and labels.

The list supports direct actions:

- **Mark read**
- **Mark unread**
- **Bookmark**
- **Remove bookmark**
- **Archive**
- **Open**

**Open** launches the item URL in the system default web browser. The app does
not include an embedded detail browser in v1.

Unread, bookmark, and archive state are local to this app and are not synced
back to GitHub.

## Status Messages

The bottom status bar shows lightweight status messages for setup, refresh,
errors, host name, and redacted token state. Errors are shown without clearing
the current item list.
