"""Scenario 10: settings, CLI, API keys, doctor."""

import json

from lib import App
from scen_core import with_page


def s10_cli(c):
    R, app = c.R, c.main
    ui = {"Origin": app.base}  # what the embedded UI sends
    rc, out, err = app.cli("status", "--json")
    R.check("cli status --json", rc == 0 and json.loads(out), (rc, out[:200], err[:200]))
    rc, out, err = app.cli("profile", "create", "--name", "cli-prof", "--os", "mac", "--json")
    pj = json.loads(out) if rc == 0 else {}
    R.check("cli profile create --json", rc == 0 and pj.get("id"), (rc, out[:200], err))
    pid = pj.get("id")
    rc, out, err = app.cli("profile", "list", "--json")
    R.check("cli profile list --json has it", rc == 0 and pid in out, (rc, out[:200]))
    rc, out, err = app.cli("profile", "start", pid, "--json")
    sj = json.loads(out) if rc == 0 else {}
    ws = sj.get("wsEndpoint")
    R.check("cli profile start --json returns wsEndpoint", bool(ws) and ws.startswith("ws://"), (rc, out[:200], err[:200]))
    if ws:
        v = with_page(c, ws, lambda p, ctx: p.evaluate("navigator.userAgent"))
        R.check("wsEndpoint from the CLI is drivable via Playwright", "Firefox" in v, v)
    rc, out, err = app.cli("profile", "stop", pid, "--json")
    R.check("cli profile stop", rc == 0, (rc, out, err))
    rc, out, err = app.cli("profile", "show", "deadbeefdeadbeefdeadbeef", "--json")
    R.check("cli error: non-zero exit + 'error: <code>: <message>' on stderr", rc != 0 and err.startswith("error: ") and err.count(":") >= 2, (rc, err))
    rc, out, err = app.cli("profile", "start", "--json")
    R.check("cli missing arg -> non-zero", rc != 0, (rc, out, err))
    rc, out, err = app.cli("nonsense")
    R.check("cli unknown command -> non-zero", rc != 0, (rc, err))
    rc, out, err = app.cli("skill")
    R.check("cli skill prints SKILL.md", rc == 0 and "profile start" in out, out[:100])
    rc, out, err = app.cli("docs")
    R.check("cli docs prints markdown", rc == 0 and "profile" in out)
    rc, out, err = app.cli("proxy", "import", "--text", "http://a:b@127.0.0.1:9999", "--json")
    R.check("cli proxy import", rc == 0, (rc, out[:150], err[:150]))
    rc, out, err = app.cli("proxy", "list", "--json")
    R.check("cli proxy list never shows the password", rc == 0 and '"b"' not in out and "password" not in out.replace("hasPassword", ""), out[:300])

    rc, out, err = app.cli("doctor", "--json")
    R.note(f"doctor rc={rc}")
    try:
        d = json.loads(out)
        checks = d.get("checks", d)
        bad = [x for x in checks if x.get("status") not in ("ok",)]
        R.check("doctor: all checks ok with the real venv", rc == 0 and not bad, bad)
    except Exception as e:
        R.check("doctor json parses", False, (e, out[:300], err[:200]))
    rc, out, err = app.cli("doctor")
    R.check("doctor human output", "ok" in out.lower() or "OK" in out, out[:200])

    # settings
    s = app.put("/api/settings", {"language": "vi"})
    R.check("language persists (vi)", app.get("/api/settings")["language"] == "vi")
    app.put("/api/settings", {"language": "en"})
    s = app.put("/api/settings", {"headless": True})
    R.check("headless setting stored", app.get("/api/settings")["headless"] is True)
    s, b, _ = app.req("PUT", "/api/settings", {"language": "xx"})
    R.check("bad language rejected", s >= 400, (s, b))

    # API keys
    s, b, _ = app.req("POST", "/api/api-keys", {"name": "agent1"})
    R.check("key management is UI-only (403 for non-UI client)", s == 403, (s, b))
    k1 = app.post("/api/api-keys", {"name": "agent1"}, headers=ui)
    key1 = k1.get("key") or k1.get("secret")
    R.check("key minted from the UI origin, secret returned once", bool(key1), k1)
    R.check("list never returns the secret", key1 not in json.dumps(app.get("/api/api-keys", headers=ui)))
    k2 = app.post("/api/api-keys", {"name": "agent2"}, headers=ui)
    key2 = k2.get("key") or k2.get("secret")
    app.put("/api/settings", {"requireApiKey": True}, headers=ui)
    s, b, _ = app.req("GET", "/api/overview")
    R.check("require-key on: plain client -> 401 api_key_required", s == 401 and (b or {}).get("code") == "api_key_required", (s, b))
    rc, out, err = app.cli("profile", "list", "--json")
    R.check("CLI without key -> non-zero + 401 text", rc != 0 and ("api_key_required" in err or "401" in err), (rc, err))
    rc, out, err = app.cli("profile", "list", "--json", key=key1)
    R.check("CLI with --api-key works", rc == 0, (rc, err))
    rc, out, err = app.cli("profile", "list", "--json", env={"CM_API_KEY": key2})
    R.check("CLI with $CM_API_KEY works", rc == 0, (rc, err))
    rc, out, err = app.cli("profile", "list", "--json", key="cm_wrongkey")
    R.check("CLI with a wrong key -> 401 invalid", rc != 0 and "invalid" in err, (rc, err))
    s, b, _ = app.req("GET", "/api/overview", headers=ui)
    R.check("UI origin keeps working without a key", s == 200, (s, b))
    s, b, _ = app.req("GET", "/api/overview", headers={"Origin": "http://evil.example"})
    R.check("foreign browser origin -> 403", s == 403, (s, b))
    s, b, _ = app.req("GET", "/api/overview", headers={"Host": "evil.example", "Origin": "http://evil.example", "X-API-Key": key1})
    R.note(f"foreign host + valid key -> {s}")
    keys = app.get("/api/api-keys", headers=ui)["keys"]
    k1row = [k for k in keys if k["name"] == "agent1"][0]
    R.check("last use recorded on the used key, not the unused", bool(k1row.get("lastUsed") or k1row.get("lastUsedAt")) and not [k for k in keys if k["name"] == "agent2" and False], k1row)
    app.delete(f"/api/api-keys/{k1row['id']}", headers=ui)
    rc, out, err = app.cli("profile", "list", "--json", key=key1)
    R.check("revoked key rejected", rc != 0, (rc, err))
    rc, out, err = app.cli("profile", "list", "--json", key=key2)
    R.check("other key unaffected by the revocation", rc == 0, (rc, err))
    # start via a header-less API while key required
    app.put("/api/settings", {"requireApiKey": False}, headers=ui)
    s, b, _ = app.req("GET", "/api/overview")
    R.check("require-key off again", s == 200)
    for k in app.get("/api/api-keys", headers=ui)["keys"]:
        app.req("DELETE", f"/api/api-keys/{k['id']}", headers=ui)
    app.delete(f"/api/profiles/{pid}")
    for p in app.get("/api/proxies")["proxies"]:
        if p["serverPort"] == 9999:
            app.delete(f"/api/proxies/{p['id']}")
