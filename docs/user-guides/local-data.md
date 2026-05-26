# Local Data

This guide explains where `ghTimeLine` stores local settings and fetched
stream data.

## Configuration File

Application settings are stored in a plain text YAML file:

- Linux: `$XDG_CONFIG_HOME/ghtl/config.yml`, or
  `~/.config/ghtl/config.yml` when `XDG_CONFIG_HOME` is not set
- Windows: `%APPDATA%/ghtl/config.yml`

The v1 app stores the Personal Access Token in this file as plain text. Treat
the file as sensitive.

Saved query export uses a separate YAML file by default:

- Linux: `$XDG_CONFIG_HOME/ghtl/saved-queries.yml`, or
  `~/.config/ghtl/saved-queries.yml` when `XDG_CONFIG_HOME` is not
  set
- Windows: `%APPDATA%/ghtl/saved-queries.yml`

## Database File

Fetched stream data, saved queries, unread state, bookmarks, and archived state
are stored in a local SQLite database:

- Linux: `$XDG_DATA_HOME/ghtl/ghtl.db`, or
  `~/.local/share/ghtl/ghtl.db` when `XDG_DATA_HOME` is
  not set
- Windows: `%LOCALAPPDATA%/ghtl/ghtl.db`

The token is not stored in the database.
