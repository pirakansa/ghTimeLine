# ghStreamListner

`ghStreamListner` is a desktop tool under development for reading GitHub
activity through saved search queries, inspired by
[Jasper](https://github.com/jasperapp/jasper).

I have used Jasper as part of my regular GitHub workflow. It shaped the way I
think about search-query based issue and pull request tracking, but it is no
longer the best fit for my own environment and workflow: maintenance outside
Apple Silicon Macs appears to have slowed, some Jasper features are more than I
need, and some behaviors I want are not available there.

This project is an attempt to build a small Rust-based replacement for my own
use. The goal is to turn GitHub search URLs into a lightweight stream reader:
define the searches you care about, refresh them from one native window, and
review new matching issues or pull requests without rebuilding the same filters
in a browser.

## Concept

GitHub is strongest when a team already knows exactly what to search for:
labels, authors, review states, milestones, repositories, or query fragments
that represent a working queue. `ghStreamListner` treats those searches as
first-class subscriptions.

Current implemented foundation:

- First-run setup for one GitHub.com or GHES host.
- Plain text YAML settings for host, PAT, UI, sort, and polling interval.
- SQLite storage for saved queries, stream items, query matches, unread state,
  bookmarks, and archives.
- A two-pane `egui` shell with library views, saved query management, local item
  actions, filters, and external browser opening.
- Manual and polling-based REST Search refresh for saved issue and pull request
  queries, with results written to SQLite before rendering.

GraphQL enrichment is scaffolded but not yet implemented. The v1 product and
storage contract is tracked in [docs/plan/v1.md](docs/plan/v1.md).

## Development

Prerequisites:

- Rust stable toolchain
- `vorbere`

Useful commands:

```sh
vorbere run check
vorbere run test
vorbere run build
```

`vorbere run run` starts the native desktop app. The other commands match the
local validation expected before opening a pull request.
