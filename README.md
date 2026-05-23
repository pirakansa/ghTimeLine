# ghStreamListner

`ghStreamListner` is a planned desktop tool for reading GitHub activity through
saved search queries, inspired by [Jasper](https://github.com/jasperapp/jasper).

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

Planned usage:

- Save named GitHub search queries.
- Refresh each query and show the latest matching results.
- Separate unread or newly seen results from already reviewed ones.
- Keep local state on the machine running the app.

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
