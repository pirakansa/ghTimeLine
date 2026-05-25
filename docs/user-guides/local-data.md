# Local Data

This guide explains where `ghStreamListner` stores local settings and fetched
stream data.

## Configuration File

Application settings are stored in a plain text YAML file:

- Linux: `$XDG_CONFIG_HOME/ghstreamlistner/config.yml`, or
  `~/.config/ghstreamlistner/config.yml` when `XDG_CONFIG_HOME` is not set
- Windows: `%APPDATA%/ghstreamlistner/config.yml`

The v1 app stores the Personal Access Token in this file as plain text. Treat
the file as sensitive.

Saved query export uses a separate YAML file by default:

- Linux: `$XDG_CONFIG_HOME/ghstreamlistner/saved-queries.yml`, or
  `~/.config/ghstreamlistner/saved-queries.yml` when `XDG_CONFIG_HOME` is not
  set
- Windows: `%APPDATA%/ghstreamlistner/saved-queries.yml`

## Database File

Fetched stream data, saved queries, unread state, bookmarks, and archived state
are stored in a local SQLite database:

- Linux: `$XDG_DATA_HOME/ghstreamlistner/ghstreamlistner.db`, or
  `~/.local/share/ghstreamlistner/ghstreamlistner.db` when `XDG_DATA_HOME` is
  not set
- Windows: `%LOCALAPPDATA%/ghstreamlistner/ghstreamlistner.db`

The token is not stored in the database.
