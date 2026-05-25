# Configuration

The app stores settings in a plain text YAML file. The top-level keys are:

- `host`
- `auth`
- `ui`
- `refresh`

Example:

```yaml
host:
  name: "GitHub.com"
  scheme: "https"
  hostname: "api.github.com"
  rest_api_base_path: "/"
  kind: "github"
auth:
  pat: "ghp_example"
ui:
  theme: "system"
  accent_color: "#4F8CC9"
  default_sort: "updated_desc"
  font_size: "default"
refresh:
  polling_interval_seconds: 180
```

Configuration rules:

- `host.name` must be non-empty after trimming whitespace.
- `host.scheme` must be `https` or `http`.
- `host.hostname` must be non-empty and must not include a scheme, path, query,
  fragment, username, password, or port.
- `host.rest_api_base_path` is normalized to exactly one leading slash and one
  trailing slash.
- `host.kind` must be `github` or `ghes`.
- `host.kind: "github"` requires `host.hostname: "api.github.com"`.
- `auth.pat` must be non-empty and is stored as plain text in v1.
- `ui.theme` must be `light`, `dark`, or `system`.
- `ui.accent_color` must be a `#RRGGBB` hex color.
- `ui.default_sort` must be one of the supported sort values.
- `ui.font_size` must be `default`, `large`, or `x_large`.
- `refresh.polling_interval_seconds` must be from `15` through `3600`.

Unknown enum values are rejected by deserialization and validation. Unknown
object keys may be ignored by the YAML parser.

The effective REST API base URL is:

```text
{scheme}://{hostname}{rest_api_base_path}
```

The effective GraphQL URL is:

- GitHub.com: `{scheme}://api.github.com/graphql`
- GHES: `{scheme}://{hostname}/api/graphql`
