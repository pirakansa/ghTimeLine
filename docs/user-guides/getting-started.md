# Getting Started

This guide explains how to start `ghTimeLine` and complete first-run setup.

## Start The App

Run the native desktop app with:

```sh
vorbere run run
```

On first launch, the app opens the setup screen because it needs one GitHub host
and a Personal Access Token before it can refresh streams.

## First-Run Setup

The setup screen collects:

- Host display name
- Scheme: `https` or `http`
- Hostname
- REST API base path
- Host kind: `github` or `ghes`
- Personal Access Token

For GitHub.com, use:

- Host kind: `github`
- Hostname: `api.github.com`
- REST API base path: `/`

For GitHub Enterprise Server, use:

- Host kind: `ghes`
- Hostname: your GHES API hostname
- REST API base path: usually `/api/v3/`

Use **Test** to check the connection. A failed connection test does not prevent
you from saving the configuration, so existing local data can still be used
while offline or while credentials are being fixed.

Use **Save** to write the configuration and enter the main stream view. After
setup, use **Preferences** > **Host settings** to reopen this screen and update
the host or token.
