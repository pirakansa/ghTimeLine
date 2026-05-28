# Saved Queries

Saved queries track GitHub issues, pull requests, discussions, or ProjectV2
items. Use **Manage** in the left pane to open the saved query management
screen.

## Add A Query

1. Select **New** in the saved query management screen.
2. Enter a user-visible name.
3. Select **Issues and pull requests**, **Discussions**, or **Project items** as
   the source.
4. Enter the GitHub Search query string, or for Project items enter a project
   URL, `node:PROJECT_ID`, `org:OWNER number:N`, or `user:OWNER number:N`.
5. Choose whether the query is **Enabled**.
6. Select **Add**.

## Edit A Query

1. Select a saved query in the saved query management screen.
2. Update the name, source, query string, or **Enabled** state.
3. Select **Preview** to open the current query draft in the GitHub search UI,
   or open the resolved project page for **Project items**.
4. Select **Save changes**.

Refreshes always look for the most recently updated GitHub items first. Change
the stored item list ordering from the stream toolbar.

Issue and pull request sources keep their REST Search plus GraphQL enrichment
flow. Discussion sources are fetched with GitHub GraphQL Search. Project item
sources fetch non-archived ProjectV2 issue and pull request items with GraphQL;
draft issues and redacted items are skipped. Project item refresh requires a
GitHub token with `read:project`.

Use **▲** and **▼** in the saved query list to reorder saved queries. Reordering
only swaps within the enabled or disabled section you are currently editing.

## Add A Filter Stream

1. Select an existing saved query in the saved query management screen.
2. Select **F+**.
3. Enter a filter stream name and a local filter query.
4. Choose whether the filter stream is **Enabled**.
5. Select **Add**.

Filter streams stay local to the parent saved query's cached items. Refreshing a
selected filter stream refreshes its parent saved query instead of issuing a
separate GitHub request.

## Delete A Query

Select a saved query in the saved query management screen and use **Delete**.

## Export And Import Queries

Use **Import / export** in the saved query management screen toolbar to open a
dedicated transfer screen for saving or restoring saved query definitions as
YAML.

- The default file path is `saved-queries.yml` under the app config directory.
- Export writes the current host plus saved query source and filter stream
  definitions only.
- Import replaces the current host's saved queries and clears cached matches
  until the next refresh rebuilds them.

## Disabled Queries

Disabled queries remain visible in the saved query management screen, but are
hidden from the left pane and skipped by refreshes and aggregated library views.
