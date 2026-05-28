# Validation Coverage

The current test suite covers:

- Configuration normalization and validation
- PAT redaction from error messages
- REST Search response parsing
- REST Search discovery ordering remains updated-descending
- GraphQL enrichment parsing and review status derivation
- GraphQL Discussion Search discovery and discussion item persistence
- ProjectV2 discovery, persistence, and unread change detection
- Refresh write-before-render behavior
- Refresh failure preserving existing stored items
- GraphQL enrichment failure preserving existing pull request metadata
- Shared item metadata save deduplication across overlapping saved queries
- GraphQL enrichment deduplication and bounded batch execution
- Successful GraphQL batch application when another batch fails
- Host initialization without storing the PAT
- Item state persistence across metadata upserts
- Archived unread badge behavior
- Saved query updates, source persistence, import/export, and filter stream transfer
- Saved query source persistence and discussion preview routing
- Stream toolbar sort ordering for selected saved query views
- Polling interval persistence and status history retention
- Local SQL-backed toolbar filter validation and matching
- UI state/event handling
- `egui_kittest` component interactions for toolbar, left pane, and item list
