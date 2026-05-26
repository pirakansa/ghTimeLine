# Saved Queries

A saved query has:

- Host association
- User-visible name
- GitHub Search query string
- Enabled state
- Position
- Last successful sync timestamp
- Last sync error message

The UI supports creating, editing, deleting, enabling, disabling, listing, and
selecting saved queries through a full-window saved query management screen.
That screen provides access to a separate full-window import/export screen for
exporting and importing saved query definitions as YAML.
Query names and query strings must not be empty when created or updated.

Saved query YAML import/export includes:

- File format version
- Host identity fields
- Saved query name
- GitHub Search query string
- Enabled state
- Position

Saved query YAML import must replace the current host's saved queries. Import
does not preserve cached matches or sync metadata; the next refresh rebuilds
matches from the imported definitions.

Saved queries target GitHub Search for issues and pull requests. Their refresh
requests use recently updated ordering independently of the stream view sort.

Saved query unread badges count distinct matched items where local state is
unread and not archived.

Disabled saved queries remain editable in the saved query management screen, but
the main stream left pane only lists enabled saved queries.
