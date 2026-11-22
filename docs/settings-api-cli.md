# Settings, API keys and CLI

## Settings

The Settings page contains:

- **Language**: English or Tiếng Việt. The choice is saved.
- **Engine**: Camoufox status, install or update (runs in the background; one install at a time).
- **API keys**: one key per agent or machine. Keys are sent as the `X-API-Key` header. The window itself
  never needs a key. Turn on "require API key" to protect the HTTP API.
- **Devices**: device sync (see [sync.md](sync.md)).

## HTTP API

The API is served under `/api` on `127.0.0.1:<CM_PORT>` (default `7878`). The UI uses the same API.

## CLI

The same binary has a command-line interface. Run `camoufox-manager --help`, or `<command> --help`, for the
full list. The commands are:

- `status`
- `profile list`, `profile create --name <name> [--os <win|mac|lin>]`, `profile show`,
  `profile start`, `profile stop`, `profile delete`
- `proxy list`, `proxy import` (`--file <path>` or `--text <lines>`), `proxy check`, `proxy assign`
- `doctor` (see [doctor.md](doctor.md))
- `skill` and `docs`, which print the generated command reference

Common flags: `--json` for machine-readable output, `--port <n>` (default `$CM_PORT`, else `7878`) and
`--api-key <key>` (default `$CM_API_KEY`).

The CLI talks to a running app or `serve` process. Start one first.
