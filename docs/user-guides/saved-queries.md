# Saved Queries

Saved queries are GitHub Search query strings for issues and pull requests. Use
**Manage** in the left pane to open the saved query management screen.

## Add A Query

1. Select **New** in the saved query management screen.
2. Enter a user-visible name.
3. Enter the GitHub Search query string.
4. Choose whether the query is **Enabled**.
5. Select **Add**.

## Edit A Query

1. Select a saved query in the saved query management screen.
2. Update the name, query string, or **Enabled** state.
3. Select **Save changes**.

Refreshes always look for the most recently updated GitHub items first. Change
the stored item list ordering from the stream toolbar.

## Delete A Query

Select a saved query in the saved query management screen and use **Delete**.

## Export And Import Queries

Use the **Import / export** section in the saved query management screen to save
or restore saved query definitions as YAML.

- The default file path is `saved-queries.yml` under the app config directory.
- Export writes the current host and saved query definitions only.
- Import replaces the current host's saved queries and clears cached matches
  until the next refresh rebuilds them.

## Disabled Queries

Disabled queries remain visible in the saved query management screen, but are
hidden from the left pane and skipped by refreshes and aggregated library views.
