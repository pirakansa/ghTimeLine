# ghTimeLine

<p align="center">
        <img src="assets/icon.png" alt="ghTimeLine logo" width="320">
</p>

`ghTimeLine` is a native desktop app for reading GitHub activity through saved
queries. It tracks issues, pull requests, discussions, and supported ProjectV2
items in a local SQLite cache so you can review updates without keeping a
browser tab open for every search.

## Run locally

Prerequisites:

- Rust stable toolchain
- `vorbere`

Start the desktop app with:

```sh
vorbere run run
```

On first launch, complete the setup screen with one GitHub or GHES host and a
Personal Access Token.

## Development

Useful commands:

```sh
vorbere run check
vorbere run test
vorbere run build
```

`vorbere run run` starts the native desktop app. The other commands match the
local validation expected before opening a pull request.

## Documentation

- [User guides](docs/user-guides/README.md): day-to-day usage and workflows
- [Specification references](docs/specifications/README.md): implemented
  behavior that should remain stable as the app evolves
