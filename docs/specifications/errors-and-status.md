# Errors And Status

Configuration parse and validation errors surface in setup. The setup screen is
also reachable from the main stream view through the preferences menu so the
active host and token can be edited after first-run setup.

Authentication, network, API, and database failures are surfaced as user-visible
status messages. Error messages must not include the Personal Access Token.

The status bar is separate from the item list. Communication failures should not
replace the item list with a blocking error screen.
