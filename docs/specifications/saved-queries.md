# Saved Queries

A saved query has:

- Host association
- User-visible name
- GitHub Search query string
- Source type (`issue_or_pull_request`, `discussion`, or `project_v2`)
- Enabled state
- Position
- Last successful sync timestamp
- Last sync error message

A saved query may also own zero or more filter streams. A filter stream has:

- Parent saved query association
- User-visible name
- Local filter query string
- Enabled state
- Position within that parent saved query

The UI supports creating, editing, deleting, enabling, disabling, listing,
previewing, and selecting saved queries through a full-window saved query
management screen. Preview opens the current saved query draft in the host's
GitHub search web UI for search-based sources, or the resolved project page for
ProjectV2 sources. That screen provides access to a separate full-window
import/export screen for exporting and importing saved query definitions as YAML.
Query names and query strings must not be empty when created or updated. Saved
query names must be unique per host, and filter stream names must be unique
within a saved query.

The saved query manager also supports moving saved queries up or down. Reordering
only swaps with another saved query in the same enabled or disabled group, so
enabled queries stay above disabled queries in the main left pane.

Saved query YAML import/export includes:

- File format version
- Host identity fields
- Saved query name
- GitHub Search query string
- Source type
- Enabled state
- Position
- Filter stream name
- Filter stream local query string
- Filter stream enabled state
- Filter stream position within its saved query

Saved query YAML import must replace the current host's saved queries and filter
stream definitions. Import does not preserve cached matches or sync metadata;
the next refresh rebuilds matches from the imported definitions.

Saved query YAML import trims required names and query strings, rejects
duplicate saved query names and duplicate filter stream names within a saved
query, and normalizes imported positions into contiguous local order.

Saved queries select one remote source:

- Issue and pull request streams use REST Search discovery followed by GraphQL
  enrichment.
- Discussion streams use GraphQL Search with `type: DISCUSSION`.
- ProjectV2 streams use GraphQL ProjectV2 items discovery and include issue and
  pull request content only.

Remote sources request updated-item discovery independently of the stream view
sort. Existing definitions without a source type import as issue and pull
request streams.

ProjectV2 query strings must identify a project as a project URL,
`node:PROJECT_ID`, `org:OWNER number:N`, or `user:OWNER number:N`. ProjectV2
refreshes page through up to 500 non-archived project items, skip draft issues
and redacted items, and use the later of the project item update timestamp and
issue or pull request update timestamp for stream change detection. ProjectV2
refresh requires a GitHub token with `read:project`.

Filter streams remain local child views over cached saved query matches.
Refreshing a selected filter stream refreshes its parent saved query rather than
issuing an additional remote query for the filter stream itself.

Saved query unread badges count distinct matched items where local state is
unread and not archived.

Disabled saved queries remain editable in the saved query management screen, but
the main stream left pane only lists enabled saved queries.

Disabled filter streams remain editable in the saved query management screen,
but the main stream left pane only lists enabled filter streams.
