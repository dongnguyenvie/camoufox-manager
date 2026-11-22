# Proxies and relay

## Proxy library

Proxies are stored in the library (SQLite). Add them in the Proxies page or import a file with one proxy per
line from the CLI (`proxy import --file`, `--text`). The legacy `proxies.txt` is imported once on first start.

Supported upstream schemes: `http`, `https`, `socks5`, `socks5h`, `socks4a` and `socks4`.

- SOCKS5 and SOCKS4a resolve destination names on the proxy (remote DNS).
- SOCKS4 (plain) is handled as SOCKS4a.
- SOCKS4 does not support IPv6 destinations, and such connections are refused.

Check a proxy to see its IP, country, city, timezone and coordinates. The check result is shown in the
library and used for geography (see [profiles.md](profiles.md)).

## Assignment

A profile uses at most one proxy. Assign from the profile form or from the assignment dialog, which also
accepts several profiles at once.

## Local relay (fail-closed)

Each assigned proxy gets a local relay on `127.0.0.1:<port>`, starting at `CM_PROXY_BASE_PORT` (default
`31000`; proxy N uses `base + N - 1`). The browser is pointed at that port with no credentials. The relay
adds the upstream credentials and forwards the traffic.

Fail-closed means: if the upstream proxy fails, the connection is closed. The browser never falls back to a
direct connection, so your real IP is not used.

Credentials are kept in the app process. They are not passed to the browser or shown in the UI.
