# Desktop App User Guide

This guide explains how to use `ghStreamListner` as a local desktop stream
reader for GitHub issues and pull requests.

## Start The App

Run the native desktop app with:

```sh
vorbere run run
```

On first launch, the app opens the setup screen because it needs one GitHub host
and a Personal Access Token before it can refresh streams.

## First-Run Setup

The setup screen collects:

- Host display name
- Scheme: `https` or `http`
- Hostname
- REST API base path
- Host kind: `github` or `ghes`
- Personal Access Token

For GitHub.com, use:

- Host kind: `github`
- Hostname: `api.github.com`
- REST API base path: `/`

For GitHub Enterprise Server, use:

- Host kind: `ghes`
- Hostname: your GHES API hostname
- REST API base path: usually `/api/v3/`

Use **Test** to check the connection. A failed connection test does not prevent
you from saving the configuration, so existing local data can still be used
while offline or while credentials are being fixed.

Use **Save** to write the configuration and enter the main stream view.

## Configuration And Local Data

Application settings are stored in a plain text YAML file:

- Linux: `$XDG_CONFIG_HOME/ghstreamlistner/config.yml`, or
  `~/.config/ghstreamlistner/config.yml` when `XDG_CONFIG_HOME` is not set
- Windows: `%APPDATA%/ghstreamlistner/config.yml`

The v1 app stores the Personal Access Token in this file as plain text. Treat
the file as sensitive.

Fetched stream data, saved queries, unread state, bookmarks, and archived state
are stored in a local SQLite database:

- Linux: `$XDG_DATA_HOME/ghstreamlistner/ghstreamlistner.db`, or
  `~/.local/share/ghstreamlistner/ghstreamlistner.db` when `XDG_DATA_HOME` is
  not set
- Windows: `%LOCALAPPDATA%/ghstreamlistner/ghstreamlistner.db`

The token is not stored in the database.

## Main Window

The main window uses two panes:

- The left pane contains library entries and saved queries.
- The right pane contains the item list for the selected library entry or saved
  query.

The library section contains:

- **Inbox**: non-archived items across enabled saved queries
- **Bookmark**: bookmarked, non-archived items across enabled saved queries
- **Archived**: archived items across enabled saved queries

Each saved query entry shows its unread count. Archived unread items are not
included in saved query unread counts.

## Saved Queries

Saved queries are GitHub Search query strings for issues and pull requests.
Use **Manage** in the left pane to open the saved query management screen.

To add a query:

1. Select **New** in the saved query management screen.
2. Enter a user-visible name.
3. Enter the GitHub Search query string.
4. Choose whether the query is **Enabled**.
5. Select **Add**.

To edit a query:

1. Select a saved query in the saved query management screen.
2. Update the name, query string, sort order, or **Enabled** state.
3. Select **Save changes**.

To delete a query, select it in the saved query management screen and use
**Delete**.

Disabled queries remain visible in the left pane but are skipped by refreshes
and aggregated library views.

## Refreshing Streams

Use **Refresh** to manually refresh the selected saved query. When a library
view is selected, manual refresh runs all saved queries.

The app also polls automatically. The default polling interval is 5 minutes. Use
the interval control in the toolbar to change the interval and save it to the
YAML configuration file.

Refresh results are written to the local database before the list is rendered.
If authentication, network, or API errors happen during refresh, previously
stored items remain visible.

## Filtering And Sorting

The toolbar supports these filters:

- **All**: no additional filter
- **Open**: open issues and open, unmerged pull requests
- **Unread**: unread items
- **Bookmarked**: bookmarked items

The sort selector supports:

- Updated descending
- Updated ascending
- Created descending
- Created ascending
- Comments descending
- Comments ascending

Library views use the default sort from the YAML configuration. Saved query
views use the sort configured for that saved query.

## Item Actions

Each list item shows the repository, item type, number, title, update time,
unread state, author, assignees, review status when available, comment count,
state, and labels.

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
