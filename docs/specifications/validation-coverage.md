# Validation Coverage

The current test suite covers:

- Configuration normalization and validation
- PAT redaction from error messages
- REST Search response parsing
- GraphQL enrichment parsing and review status derivation
- Refresh write-before-render behavior
- Refresh failure preserving existing stored items
- GraphQL enrichment failure preserving existing pull request metadata
- Shared item metadata save deduplication across overlapping saved queries
- Host initialization without storing the PAT
- Item state persistence across metadata upserts
- Archived unread badge behavior
- Saved query updates
- UI state/event handling
- `egui_kittest` component interactions for toolbar, left pane, and item list
