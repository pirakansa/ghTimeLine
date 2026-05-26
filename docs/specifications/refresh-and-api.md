# Refresh And API Strategy

This reference defines refresh scope, polling, API usage, and refresh write
flow.

## Manual Refresh

- Refreshes the selected saved query when a saved query is selected.
- Refreshes all enabled saved queries when a library entry is selected.
- Persists remote results immediately, but does not automatically redraw the
  current item list when those results change its displayed contents.

## Automatic Polling

- Runs against enabled saved queries.
- Defaults to a 180 second interval.
- Uses the interval stored in the YAML configuration.
- Persists interval changes to the YAML configuration.
- Persists remote results immediately, but does not automatically redraw the
  current item list when those results change its displayed contents.

## API Strategy

- REST Search is the discovery source for issues and pull requests.
- REST Search discovery always requests results ordered by most recently updated
  first (`sort=updated&order=desc`) so display preferences do not displace
  newly updated items from the fetched page.
- REST Search results are parsed into normalized stream item data.
- Pull requests with node IDs are enriched through GraphQL.
- GraphQL enrichment fills draft state, merge state, merged timestamp, review
  status, and reviewer metadata when available.
- Failed GraphQL enrichment must not prevent REST Search data from being stored
  or rendered.
- Failed GraphQL enrichment must preserve previously stored merge and review
  metadata for an existing pull request.
- When multiple saved queries refresh together, REST Search requests remain
  sequential and query-specific.
- GraphQL enrichment deduplicates pull request node IDs across those REST Search
  results and fetches enrichment in batches of at most 50 node IDs.
- A failed GraphQL enrichment batch must not prevent other batches from being
  applied or REST Search data from being stored.

## Refresh Write Flow

1. Fetch REST Search results for a saved query.
2. Attempt GraphQL enrichment for discovered pull requests.
3. Upsert stream items and query matches into SQLite; identical items returned
   by multiple saved queries in one refresh reuse a single metadata save.
4. Mark query sync success or store a short sync error.
5. Update refresh status and reload sidebar counts when stored items changed or
   a query refresh failed.
6. If the refresh changes the displayed current view, retain the visible item
   list snapshot and show an update banner until the user selects **Show
   updates** or otherwise reloads the view.

The app must not assume polling can infer every GitHub state transition. Cached
matches may remain when an item stops appearing in a later search result.
