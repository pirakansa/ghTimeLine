# Refresh And API Strategy

This reference defines refresh scope, polling, API usage, and refresh write
flow.

## Manual Refresh

- Refreshes the selected saved query when a saved query is selected.
- Refreshes the parent saved query when a filter stream is selected.
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
- Discussion saved queries use GraphQL `search(type: DISCUSSION)` discovery and
  are stored as discussion stream items.
- Discussion discovery adds recently updated ordering to the GitHub search
  query and does not run pull request enrichment.
- ProjectV2 saved queries use GraphQL ProjectV2 item discovery. The query string
  identifies the project as a project URL, `node:PROJECT_ID`,
  `org:OWNER number:N`, or `user:OWNER number:N`.
- ProjectV2 discovery pages through up to 500 non-archived project items, stores
  issue and pull request content, and skips draft issues and redacted items.
- ProjectV2 item updates are reflected in the stream item update timestamp so
  project field changes can mark the item unread even when the underlying issue
  or pull request did not change.
- ProjectV2 discovery requires a GitHub token with `read:project`.
- Issues and pull requests with node IDs are enriched through GraphQL.
- GraphQL enrichment fills draft state, merge state, merged timestamp, review
  status, reviewer metadata, and local involvement metadata such as
  participants/commenters and parsed mentions when available.
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

1. Fetch results using the saved query source: REST Search for issue and pull
    request streams, GraphQL Search for discussion streams, or GraphQL ProjectV2
    items for ProjectV2 streams.
2. Attempt GraphQL enrichment for discovered issue and pull request stream
   items only, including items discovered from ProjectV2 streams.
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

ProjectV2 streams do not currently store draft issues, redacted items, or full
project field values. They use ProjectV2 item timestamps for change detection
but render the underlying issue or pull request as the stream item.
