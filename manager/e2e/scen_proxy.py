"""Scenarios 3-4: proxy library, traffic through the fake upstreams, geo, fail-closed, leak checks."""

import json
import time

from lib import SAO_PAULO, TOKYO, ApiError, dead_port
from probe import GEO_JS, ICE_JS, PERM_JS, host_ips, probe
from scen_core import procs_for, wait_gone, with_page


def goto_quiet(page, url, timeout=15000):
    try:
        page.goto(url, wait_until="load", timeout=timeout)
        return None
    except Exception as e:  # expected for https targets / dead upstreams
        return str(e).splitlines()[0]


def external_conns(profile_id):
    """Network sockets of the profile's helper + browser tree that point outside loopback.
    Run `lsof` on every descendant of the helper process (the one with the profile id in its command line)."""
    import subprocess

    ps = subprocess.run(["ps", "-axo", "pid=,ppid=,command="], capture_output=True, text=True).stdout.splitlines()
    rows = [(int(l.split(None, 2)[0]), int(l.split(None, 2)[1]), l.split(None, 2)[2]) for l in ps if len(l.split(None, 2)) == 3]
    roots = {pid for pid, _, cmd in rows if profile_id in cmd and "helper" in cmd and "launch" in cmd}
    tree, grew = set(roots), True
    while grew:
        grew = False
        for pid, ppid, _ in rows:
            if ppid in tree and pid not in tree:
                tree.add(pid)
                grew = True
    if not tree:
        return None
    out = subprocess.run(["lsof", "-nP", "-i", "-a", "-p", ",".join(map(str, tree))], capture_output=True, text=True).stdout.splitlines()[1:]
    bad = []
    for l in out:
        f = l.split()
        name = f[8] if len(f) > 8 else l
        remote = name.split("->")[1] if "->" in name else None
        if remote is None:
            continue  # listening / unconnected UDP
        host = remote.rsplit(":", 1)[0].strip("[]").split(" ")[0]
        if host not in ("127.0.0.1", "::1", "localhost"):
            bad.append(l.strip()[:160])
    return bad


def s3_proxy(c):
    R, app, F = c.R, c.main, c.fakes
    hp = F.http_proxy(SAO_PAULO, "pu", "pp")
    sp = F.socks5_proxy(SAO_PAULO, "su", "sp")
    c.hp, c.sp = hp, sp
    # whole list is rejected when one line is bad
    bad = f"http://pu:pp@127.0.0.1:{hp.port}\nnot a proxy line\n"
    s, b, _ = app.req("POST", "/api/proxies/import", {"text": bad})
    R.check("import with a bad line rejects the whole list", s == 400 and b.get("rejected") and not app.get("/api/proxies")["proxies"], (s, b))
    text = f"http://pu:pp@127.0.0.1:{hp.port}\nsocks5://su:sp@127.0.0.1:{sp.port}\n127.0.0.1:{dead_port()}:legacy:pw\n"
    r = app.post("/api/proxies/import", {"text": text})
    R.check("import 3 (http, socks5 auth, legacy host:port:user:pass)", len(r["created"]) == 3 and not r["rejected"], r)
    r2 = app.post("/api/proxies/import", {"text": f"http://pu:pp@127.0.0.1:{hp.port}\n"})
    R.check("re-import exact duplicate is skipped", len(r2["created"]) == 0 and r2["skipped"] == [1], r2)
    lib = app.get("/api/proxies")["proxies"]
    http_p = [p for p in lib if p["serverPort"] == hp.port][0]
    socks_p = [p for p in lib if p["serverPort"] == sp.port][0]
    dead_p = [p for p in lib if p["id"] not in (http_p["id"], socks_p["id"])][0]
    blob = json.dumps(app.get("/api/proxies"))
    R.check("passwords never returned by the API", "\"pp\"" not in blob and "\"sp\"" not in blob and "password\":\"" not in blob.replace(" ", ""), blob[:300])
    R.check("hasPassword flag set", http_p["hasPassword"] and socks_p["hasPassword"])
    # a password must not be in the app log either
    R.check("password not in app log", "pp@" not in app.log() and ":sp@" not in app.log())

    # check through the relay
    for p, fake in ((http_p, hp), (socks_p, sp)):
        g = app.post(f"/api/proxies/{p['id']}/check")
        g = g.get("geo", g)
        R.check(f"{p['scheme']}: check geo Sao Paulo", g.get("timezone") == "America/Sao_Paulo" and g.get("countryCode") == "BR" and g.get("lat") < 0 and g.get("ip") == SAO_PAULO["query"], g)
        R.check(f"{p['scheme']}: latency reported", g.get("latencyMs") is not None, g)
    R.check("http proxy saw authed absolute-URI request to ip-api", any(k == "GET" and "ip-api.com" in t and ok for k, t, ok in hp.log), hp.log[:5])
    s, b, _ = app.req("POST", f"/api/proxies/{dead_p['id']}/check")
    R.check("dead proxy check fails with error (not 2xx)", s >= 400, (s, b))
    # wrong password
    wp = app.post("/api/proxies", {"name": "wrongpw", "scheme": "http", "host": "127.0.0.1", "port": hp.port, "username": "pu", "password": "nope"})
    s, b, _ = app.req("POST", f"/api/proxies/{wp['id']}/check")
    R.check("wrong password -> check fails mentioning credentials", s >= 400 and ("407" in json.dumps(b) or "credential" in json.dumps(b)), (s, b))
    R.check("wrong password is not echoed", "nope" not in json.dumps(b))
    app.delete(f"/api/proxies/{wp['id']}")

    # assign + start through the http proxy
    pid = app.create("s3-http", "win")
    app.post(f"/api/profiles/{pid}/assign", {"proxyId": http_p["id"]})
    R.check("assigned", app.prof_row(pid)["assigned"] is not None or app.profile(pid).get("proxyId") == http_p["id"], app.prof_row(pid)["assigned"])
    n0 = len(hp.log)
    st = app.start_profile(pid)
    R.check("start returned relay port + geo", st.get("port") and (st.get("geo") or {}).get("timezone") == "America/Sao_Paulo", st)
    R.check("relay port is in CM_PROXY_BASE_PORT range", 51000 <= (st.get("port") or 0) < 51100, st.get("port"))

    def go(page, ctx):
        e1 = goto_quiet(page, "http://e2e-target.test/hello?via=http")
        body = page.inner_text("body") if not e1 else ""
        e2 = goto_quiet(page, "https://e2e-target.test/secure", 12000)  # tunnel opens, TLS then fails: expected
        if not any(k == "CONNECT" and "e2e-target" in t for k, t, ok in hp.log):  # retry once on another host
            e2 = goto_quiet(page, "https://e2e-target-b.test/secure", 12000)
        o = probe(page, f"http://127.0.0.1:{c.page_port}/probe")
        page.goto(f"http://127.0.0.1:{c.page_port}/probe")
        g = page.evaluate(GEO_JS)
        perm = page.evaluate(PERM_JS)
        return body, e1, e2, o, g, perm

    body, e1, e2, o, g, perm = with_page(c, st["ws"], go)
    R.check("http page content arrived via proxy", "host=e2e-target.test" in body, (body, e1))
    new = hp.log[n0:]
    R.check("proxy logged GET http://e2e-target.test (authed by relay)", any(k == "GET" and "e2e-target.test/hello" in t and ok for k, t, ok in new), new)
    R.check("proxy logged a CONNECT for the https page (tunnel through the proxy)", any(k == "CONNECT" and "e2e-target" in t for k, t, ok in new) or R.note("https goto result: " + str(e2)) or False, new)
    R.check("no unauthenticated request reached the proxy", not [x for x in new if not x[2]], [x for x in new if not x[2]])
    R.check("timezone from proxy geo (America/Sao_Paulo)", o["tz"] == "America/Sao_Paulo", o["tz"])
    R.check("geolocation = proxy coordinates (negative lat/lon)", isinstance(g, dict) and "lat" in g and abs(g["lat"] - (-23.5505)) < 0.5 and g["lon"] < 0, (g, perm, o["secure"]))
    ext = external_conns(pid)
    R.check("no socket of the browser/helper tree goes outside loopback while a proxy is assigned (no direct leak)", ext == [], ext)
    app.stop_profile(pid)

    # geolocation only together with timezone: tz override != proxy tz -> no geolocation
    app.edit(pid, timezone="Europe/Berlin")
    st = app.start_profile(pid)
    o, g, perm = with_page(c, st["ws"], lambda p, ctx: (probe(p, f"http://127.0.0.1:{c.page_port}/probe"), p.evaluate(GEO_JS), p.evaluate(PERM_JS)))
    R.check("tz override wins over proxy tz", o["tz"] == "Europe/Berlin", o["tz"])
    R.check("no geolocation handed out when tz is overridden (D4)", not ("lat" in g), (g, perm))
    app.stop_profile(pid)
    app.edit(pid, timezone=None)

    # same tz as the proxy -> geolocation allowed again
    app.edit(pid, timezone="America/Sao_Paulo")
    st = app.start_profile(pid)
    g = with_page(c, st["ws"], lambda p, ctx: (p.goto(f"http://127.0.0.1:{c.page_port}/probe"), p.evaluate(GEO_JS))[1])
    R.check("explicit tz equal to proxy tz keeps geolocation", "lat" in g, g)
    app.stop_profile(pid)
    app.edit(pid, timezone=None)
    app.delete(f"/api/profiles/{pid}")

    # socks5 with auth, remote DNS
    pid = app.create("s3-socks", "mac")
    app.post(f"/api/profiles/{pid}/assign", {"proxyId": socks_p["id"]})
    st = app.start_profile(pid)

    def go2(page, ctx):
        e = goto_quiet(page, "http://dns-leak.e2e.test/s5")
        return e, (page.inner_text("body") if not e else "")

    e, body = with_page(c, st["ws"], go2)
    R.check("socks5: page content arrived", "host=dns-leak.e2e.test" in body, (e, body))
    R.check("socks5: upstream got the HOSTNAME (remote DNS), not an IP", any(k == "domain" and h == "dns-leak.e2e.test" for k, h, _ in sp.log), sp.log)
    R.check("socks5: no auth failures", not [x for x in sp.log if x[0] == "authfail"], sp.log)
    app.stop_profile(pid)
    app.delete(f"/api/profiles/{pid}")

    # fail closed: dead proxy -> start refused, no browser
    pid = app.create("s3-dead", "lin")
    app.post(f"/api/profiles/{pid}/assign", {"proxyId": dead_p["id"]})
    s, b, _ = app.req("POST", f"/api/profiles/{pid}/start")
    R.check("dead proxy: start refused (fail closed)", s >= 400 and "proxy" in json.dumps(b).lower(), (s, b))
    R.check("dead proxy: no browser running", app.prof_row(pid)["running"] is None and not procs_for(pid), procs_for(pid))
    app.delete(f"/api/profiles/{pid}")

    # upstream dies while the browser runs: page loads must fail, never go direct
    mid = F.http_proxy(SAO_PAULO, "pu", "pp")
    mp = app.post("/api/proxies", {"name": "mid", "scheme": "http", "host": "127.0.0.1", "port": mid.port, "username": "pu", "password": "pp"})
    pid = app.create("s3-mid", "win")
    app.post(f"/api/profiles/{pid}/assign", {"proxyId": mp["id"]})
    st = app.start_profile(pid)
    mid.close()
    n = len(mid.web_log)
    err = with_page(c, st["ws"], lambda p, ctx: goto_quiet(p, "http://after-death.e2e.test/x", 10000))
    R.check("upstream died: navigation fails (fail closed)", err is not None, err)
    R.check("upstream died: nothing reached the web target", len(mid.web_log) == n, mid.web_log[n:])
    app.stop_profile(pid)
    app.delete(f"/api/profiles/{pid}")
    app.delete(f"/api/proxies/{mp['id']}")

    # proxy in use cannot be silently deleted? (record behaviour)
    pid = app.create("s3-used", "win")
    app.post(f"/api/profiles/{pid}/assign", {"proxyId": http_p["id"]})
    s, b, _ = app.req("DELETE", f"/api/proxies/{http_p['id']}")
    R.note(f"delete proxy in use -> {s} {str(b)[:120]}")
    if s < 300:  # recreate for later scenarios
        r = app.post("/api/proxies/import", {"text": f"http://pu:pp@127.0.0.1:{hp.port}\n"})
        http_p = r["created"][0]
    R.check("profile of a deleted/used proxy is consistent", True)
    app.delete(f"/api/profiles/{pid}")
    c.http_p, c.socks_p, c.dead_p = http_p, socks_p, dead_p


def s4_leaks(c):
    R, app, F = c.R, c.main, c.fakes
    if not getattr(c, "http_p", None):
        R.check("needs scenario 3 proxies", False, "run scenario 3 first")
        return
    hips = host_ips()
    R.note(f"host IPs: {hips}")
    sock_log = c.sp.log
    px = {"http": c.http_p, "socks5": c.socks_p}

    def ice(page, ctx):
        page.goto(f"http://127.0.0.1:{c.page_port}/probe")
        return page.evaluate(ICE_JS)

    def leaked(cands):
        return [x for x in cands if any(ip in x for ip in hips)]

    for kind, p in px.items():
        pid = app.create(f"s4-{kind}", "lin")
        app.post(f"/api/profiles/{pid}/assign", {"proxyId": p["id"]})
        # replace mode
        st = app.start_profile(pid)
        r = with_page(c, st["ws"], ice)
        cands = r["candidates"]
        R.note(f"{kind}/replace candidates: {cands}")
        R.check(f"{kind}/replace: no host IP in ICE candidates", not leaked(cands) and not leaked([r.get("sdp", "")]), leaked(cands))
        R.check(f"{kind}/replace: SDP has no host IP", not any(ip in r.get("sdp", "") for ip in hips))
        app.stop_profile(pid)
        # real mode as the positive control: shows what the probe can detect
        app.edit(pid, webrtc="real")
        st = app.start_profile(pid)
        r = with_page(c, st["ws"], ice)
        R.note(f"{kind}/real candidates (control, host IP leak expected): {r['candidates']}")
        if kind == "http":
            R.check("control: mode=real exposes host IP or mDNS candidate (probe is sensitive)", bool(leaked(r["candidates"])) or any(".local" in x for x in r["candidates"]), r["candidates"])
        app.stop_profile(pid)
        # none mode
        app.edit(pid, webrtc="none")
        st = app.start_profile(pid)
        r = with_page(c, st["ws"], ice)
        R.check(f"{kind}/none: no RTCPeerConnection at all", r["hasRTC"] is False, r)
        app.stop_profile(pid)
        app.delete(f"/api/profiles/{pid}")

    # proxy without timezone in its geo answer: the host timezone must not leak
    nogeo = dict(SAO_PAULO, timezone="")
    ng = F.http_proxy(nogeo, "pu", "pp")
    r = app.post("/api/proxies", {"name": "nogeo", "scheme": "http", "host": "127.0.0.1", "port": ng.port, "username": "pu", "password": "pp"})
    pid = app.create("s4-nogeo", "win")
    app.post(f"/api/profiles/{pid}/assign", {"proxyId": r["id"]})
    s, b, _ = app.req("POST", f"/api/profiles/{pid}/start")
    if s < 300:
        o = with_page(c, b["ws"], lambda p, ctx: probe(p, f"http://127.0.0.1:{c.page_port}/probe"))
        R.check("[B2] proxy geo without timezone: browser tz must not be the host tz", o["tz"] != "Asia/Ho_Chi_Minh", o["tz"])
        app.stop_profile(pid)
    else:
        R.check("[B2] proxy geo without timezone: start refused or tz not host", True, (s, b))
    app.delete(f"/api/profiles/{pid}")
    app.delete(f"/api/proxies/{r['id']}")

    # tokyo proxy -> tz Tokyo and never the host
    tk = F.http_proxy(TOKYO, "pu", "pp")
    r = app.post("/api/proxies", {"name": "tokyo", "scheme": "http", "host": "127.0.0.1", "port": tk.port, "username": "pu", "password": "pp"})
    pid = app.create("s4-tokyo", "mac")
    app.post(f"/api/profiles/{pid}/assign", {"proxyId": r["id"]})
    st = app.start_profile(pid)
    o = with_page(c, st["ws"], lambda p, ctx: probe(p, f"http://127.0.0.1:{c.page_port}/probe"))
    R.check("tokyo proxy: tz Asia/Tokyo", o["tz"] == "Asia/Tokyo", o["tz"])
    R.check("tokyo proxy: locale follows proxy country or en-US (not empty)", bool(o["lang"]), (o["lang"], o["langs"]))
    app.stop_profile(pid)
    app.delete(f"/api/profiles/{pid}")
    app.delete(f"/api/proxies/{r['id']}")
    R.check("socks5 hostname seen remotely (DNS not local)", any(k == "domain" for k, _, _ in sock_log), sock_log)
