# ghTimeLine

<p align="center">
        <img src="assets/icon.png" alt="ghTimeLine logo" width="320">
</p>

`ghTimeLine` is a desktop tool under development for reading GitHub
activity through saved search queries, inspired by
[Jasper](https://github.com/jasperapp/jasper).

I have used Jasper as part of my regular GitHub workflow. It shaped the way I
think about search-query based issue and pull request tracking, but it is no
longer the best fit for my own environment and workflow: maintenance outside
Apple Silicon Macs appears to have slowed, some Jasper features are more than I
need, and some behaviors I want are not available there.

For day-to-day usage, see the [user guides](docs/user-guides/README.md). For
implemented behavior that should remain stable as the app evolves, see the
[specification references](docs/specifications/README.md).

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
