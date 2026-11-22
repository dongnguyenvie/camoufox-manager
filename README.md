# Camoufox Manager

A local, single-user macOS app for managing many [Camoufox](https://github.com/daijro/camoufox)
(Firefox anti-detect) browser profiles, written in Rust. Each profile keeps its own fingerprint, proxy,
cookies and data. Everything runs on your machine, binds to `127.0.0.1` only, and needs no cloud account.

## Features

- Create and delete profiles. The hardware fingerprint is pinned at creation and kept on every launch.
- Deleted profiles go to a trash (restore, delete forever, or empty it); retention is configurable, default 7 days.
- Assign proxies from a proxy library; check IP, country, timezone and coordinates.
- Start/stop profiles; the Playwright WebSocket endpoint is shown for automation.
- Geography (timezone, locale, geolocation, WebRTC IP) follows the assigned proxy at each launch.
- Local fail-closed proxy relay: browsers only see `127.0.0.1:<port>`, credentials stay in the app.
- Proxy change-IP link: call your provider's rotate URL manually or on each start, then re-check the geography.
- Cookie import/export; profile transfer (zip export/import, clone, bulk create).
- Tags, groups, saved views, bulk actions; spreadsheet-style profile table (copy as TSV).
- Per-profile data saving toggles (passwords, history, cache, site storage) and custom Firefox prefs.
- Per-profile bookmarks in a managed "Camoufox Manager" folder, with `bookmarks.html` import.
- Block local network by default: the relay refuses loopback and LAN targets for profiles with a proxy.
- Templates and browser extensions; scheduled launches; xlsx/csv import and export.
- Agent leases, a headless `serve` mode, a CLI with an API key, and a doctor report.
- Device sync between two of your own Macs over Tailscale (see [docs/sync.md](docs/sync.md)).
- UI in English (default) and Vietnamese, switchable in settings.

## Requirements

- macOS 11 or later, or Windows 10/11 x64 (see [Windows](#windows)), and a Rust toolchain (the app is built from source).
- The packaged app ships [uv](https://docs.astral.sh/uv/) (MIT, license in `Contents/Resources/licenses`). Running from source needs `brew install uv` for the Camoufox setup script (optional, see below).
- For device sync only: the Tailscale app.

## Install Camoufox (optional)

Camoufox is not required to build or open the app. Without it the app starts, reports that the engine
is not installed, and disables the create and start actions. In the packaged app, the Profiles page then
shows an **Install Camoufox** button that opens Settings and starts the install right away (live log, Retry,
Copy log and manual commands if it fails). When running from source, use the setup script:

```bash
manager/camoufox/setup.sh
```

The script creates a dedicated venv with `uv`, installs the pinned `camoufox` package and downloads the
browser (about 2.4 GB into the Camoufox cache). Set `CAMOUFOX_VENV=/path/to/venv` to put the venv
elsewhere, and then point `CAMOUFOX_PYTHON` at its Python.

In the packaged app, the in-app Camoufox Install / Update uses the bundled `uv` (`Contents/MacOS/uv`), so
nothing needs to be preinstalled. `CAMOUFOX_UV=/path/to/uv` overrides it.

## Build and run

```bash
cd manager
cargo run --release            # desktop window
cargo run --release -- serve   # headless server on http://127.0.0.1:7878
cargo run --release -- status  # CLI (see docs/settings-api-cli.md)
```

Package a macOS app bundle:

```bash
cd manager
./packaging/bundle-macos.sh    # -> manager/dist/Camoufox Manager.app
```

## Windows

Windows 10 or 11, x64. The window uses WebView2, which is preinstalled on current Windows 10/11.
Build from source with the MSVC Rust toolchain:

```powershell
cd manager
cargo run --release            # desktop window
cargo run --release -- serve   # headless server on http://127.0.0.1:7878
```

Package a plain zip (no installer, not signed):

```powershell
cd manager
powershell -NoProfile -ExecutionPolicy Bypass -File packaging\bundle-windows.ps1   # -> manager\dist\camoufox-manager-<version>-windows-x64.zip
```

The zip holds `camoufox-manager.exe`, `uv.exe` (pinned release, SHA-256 checked) and `licenses\`. Unzip it
anywhere and run `camoufox-manager.exe`. Camoufox is installed from the app, as on macOS.

Data lives in `%APPDATA%\camoufox-manager\` (the database is `camoufox-manager.db`).

## Configuration

Data lives in `~/Library/Application Support/camoufox-manager/` (the database is
`camoufox-manager.db`). Environment variables can be set in the shell or in a `.env` file in that folder.

| Variable | Default | Meaning |
| --- | --- | --- |
| `CM_PORT` | `7878` | UI/API port for `serve` and the CLI. The desktop window uses a random port unless this is set. |
| `CM_PROXY_BASE_PORT` | `31000` | First local relay port; proxy N uses `base + N - 1`. |
| `CM_SYNC_PORT` | `7879` | Sync listener on `127.0.0.1` (never serves the UI or `/api`). |
| `CM_SYNC_HTTPS_PORT` | `8443` | HTTPS port used for `tailscale serve`. |
| `CM_SYNC_ADVERTISE` | unset | `host:port` advertised as plain HTTP instead of the tailnet. Tests and development only. |
| `CM_SYNC_CHUNK_SIZE` | built-in | Transfer chunk size in bytes. Tests and development only. |
| `CM_TAILSCALE_BIN` | unset | Path to the `tailscale` binary; otherwise `PATH` and usual locations are searched. |
| `CAMOUFOX_PYTHON` | `<data dir>/camoufox/venv/bin/python` | Python of the Camoufox venv. |
| `CAMOUFOX_UV` | bundled, else `PATH` | Explicit path to `uv` used by the in-app Install / Update. |
| `CAMOUFOX_HEADLESS` | off | `1` launches browsers without a window. |
| `PROXY_FILE` | `<data dir>/proxies.txt` | Legacy proxy file, imported on first start. |
| `CM_API_KEY` | unset | API key the CLI sends as `X-API-Key`. |

## Tests and gates

```bash
cd manager
./gates.sh        # fmt --check, clippy -D warnings, build, test, node --check on UI scripts
./e2e/run.sh      # end-to-end scenarios (not part of gates.sh); ./e2e/run.sh 1 3 5 selects some
```

`e2e/run.sh` uses private temporary HOMEs and headless browsers, and needs the real Camoufox venv.

## Crate layout

```text
manager/
  crates/cm-core/             domain types and traits (no I/O)
  crates/cm-store/            SQLite repositories and migrations
  crates/cm-relay/            local fail-closed proxy relay and proxy checks
  crates/cm-engine-camoufox/  Camoufox engine and its Python helper
  crates/cm-service/          use-cases (Manager), one module per area
  crates/cm-api/              REST API (axum), one module per area
  crates/cm-sync/             device pairing, request signing, diff, chunked transfer
  crates/cm-tailscale/        Tailscale probe and serve helper
  crates/cm-ui/               Vue 3 + Tailwind UI assets
  apps/camoufox-manager/      binary: desktop window, serve, CLI (wiring only)
  gates.sh                    the gate run for every change
```

## Documentation

- [docs/profiles.md](docs/profiles.md): profiles, pinned fingerprint, geography, canvas modes
- [docs/proxies.md](docs/proxies.md): proxy library, relay, fail-closed behaviour
- [docs/cookies.md](docs/cookies.md): cookie import and export
- [docs/transfer.md](docs/transfer.md): import/export, clone, templates and extensions
- [docs/organization.md](docs/organization.md): tags, groups, saved views, sheets
- [docs/scheduling.md](docs/scheduling.md): scheduled launches
- [docs/settings-api-cli.md](docs/settings-api-cli.md): settings, API keys, CLI
- [docs/doctor.md](docs/doctor.md): diagnostics and notifications
- [docs/sync.md](docs/sync.md): device sync user guide

## Notes

- Use only with profiles, accounts and systems you are allowed to manage.
- The data folder holds proxy credentials. Protect it, and do not expose the local port through a tunnel or
  port forward without authentication.
- Check a proxy's IP and geography before starting a profile.
