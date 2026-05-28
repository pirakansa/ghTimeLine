# Specification References

These references describe implemented v1 behavior that should remain stable as
the app evolves. Code and passing tests are the source of truth. These
references intentionally omit database table definitions that can be read from
the storage implementation.

## Product Scope

`ghTimeLine` is a Rust desktop app for tracking GitHub issues, pull requests,
and discussions through saved GitHub Search queries.

Implemented v1 scope:

- One configured GitHub host
- GitHub.com and GitHub Enterprise Server host kinds
- Personal Access Token authentication
- YAML configuration for host, token, UI, sort, and polling settings
- SQLite-backed saved queries and stream item state
- REST Search discovery for issues and pull requests
- GraphQL Search discovery for discussions
- GraphQL enrichment for discovered pull requests
- Two-pane `egui` stream UI
- Manual refresh and automatic polling
- Local unread, bookmark, and archive state
- External browser opening for item details

Out of v1 scope:

- Multiple configured hosts
- OAuth or GitHub App authentication
- Team or cross-device state sync
- Server-side backend components
- Embedded item detail browser
- Project streams and other search resource types

## References

- [Configuration](configuration.md): YAML keys, defaults, validation, and API URL
  derivation.
- [Persistence](persistence.md): local storage boundaries and host identity.
- [Saved queries](saved-queries.md): saved query fields, validation, visibility,
  and unread badges.
- [Stream view](stream-view.md): library views, filters, sorting, item state, and
  item opening.
- [Refresh and API strategy](refresh-and-api.md): manual refresh, polling,
  source-specific discovery, GraphQL enrichment, and write flow.
- [Errors and status](errors-and-status.md): error surfacing, PAT redaction, and
  status behavior.
- [Validation coverage](validation-coverage.md): behaviors covered by the current
  test suite.
