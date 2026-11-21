"""Scenarios 1-2: profile lifecycle + settings, cookies."""

import json
import subprocess
import time

from lib import ApiError, connect
from probe import GEO_JS, PROBE_JS, probe


def procs_for(pid):
    """Lines of `ps` mentioning the profile id (helper + browser)."""
    out = subprocess.run(["ps", "-axo", "pid,command"], capture_output=True, text=True).stdout
    return [l for l in out.splitlines() if pid in l and "ps -axo" not in l]


def wait_gone(pid, timeout=20):
    t0 = time.time()
    while time.time() - t0 < timeout:
        if not procs_for(pid):
            return True
        time.sleep(0.5)
    return False


def with_page(c, ws, fn, url=None):
    """Run fn(page, ctx) against the running profile and always disconnect."""
    b, ctx = connect(c.pw, ws)
    try:
        page = ctx.pages[0] if ctx.pages else ctx.new_page()
        return fn(page, ctx)
    finally:
        try:
            b.close()
        except Exception:
            pass


def s1_lifecycle(c):
    R, app = c.R, c.main
    page_url = f"http://127.0.0.1:{c.page_port}/probe"
    ids = {}
    for os_ in ("win", "mac", "lin"):
        ids[os_] = app.create(f"s1-{os_}", os_)
    R.check("3 profiles created", len(app.overview()["profiles"]) >= 3)

    # edit settings on the win profile
    wid = ids["win"]
    app.edit(wid, name="s1-win-edited", notes="hello", locale="de-DE", timezone="Europe/Berlin", window="1000x700", screen="1366x768")
    pv = app.profile(wid)
    R.check("edit persisted (name/notes/settings)", pv["name"] == "s1-win-edited" and pv["settings"]["locale"] == "de-DE" and pv["settings"]["window"] == "1000x700", pv["settings"])

    # invalid input is rejected
    s, b, _ = app.req("PUT", f"/api/profiles/{wid}", {"name": "x", "settings": {"timezone": "Mars/Base"}})
    R.check("[B1] unknown IANA timezone rejected on edit (4xx)", 400 <= s < 500, (s, "accepted; fails only at start"))
    s, b, _ = app.req("PUT", f"/api/profiles/{wid}", {"name": "x", "settings": {"window": "abc"}})
    R.check("bad window size rejected (4xx)", 400 <= s < 500, (s, b))

    app.edit(wid, locale="de-DE", timezone="Europe/Berlin", window="1000x700", screen="1366x768")  # PUT is a full document: restore
    for os_ in ("win", "mac", "lin"):
        pid = ids[os_]
        row = app.prof_row(pid)
        fp = row["fp"]
        st = app.start_profile(pid)
        R.check(f"{os_}: start returns ws", st.get("ws", "").startswith("ws://127.0.0.1:"), st)
        R.check(f"{os_}: overview shows running", bool(app.prof_row(pid)["running"]))
        o = with_page(c, st["ws"], lambda p, _c: probe(p, page_url))
        R.check(f"{os_}: UA matches pinned", o["ua"] == fp["userAgent"], (o["ua"], fp["userAgent"]))
        R.check(f"{os_}: platform matches pinned", o["platform"] == fp["platform"], (o["platform"], fp["platform"]))
        want_ua = {"win": "Windows NT", "mac": "Macintosh", "lin": "X11"}[os_]
        R.check(f"{os_}: UA mentions {want_ua}", want_ua in o["ua"], o["ua"])
        if os_ == "win":
            R.check("win: screen override 1366x768", (o["sw"], o["sh"]) == (1366, 768), (o["sw"], o["sh"]))
            R.check("win: window override 1000x700 (outer)", (o["ow"], o["oh"]) == (1000, 700), (o["ow"], o["oh"], o["iw"], o["ih"]))
            R.check("win: locale de-DE", o["lang"] == "de-DE", (o["lang"], o["langs"]))
            R.check("win: timezone override Europe/Berlin", o["tz"] == "Europe/Berlin", o["tz"])
            # a second start while running is idempotent
            st2 = app.start_profile(pid)
            R.check("win: second start says already", st2.get("already") is True, st2)
        else:
            w, h = (int(x) for x in fp["resolution"].split("x"))
            R.check(f"{os_}: screen equals pinned {fp['resolution']}", (o["sw"], o["sh"]) == (w, h), (o["sw"], o["sh"]))
            R.check(f"{os_}: default locale en-US", o["lang"].startswith("en"), o["lang"])
            R.note(f"{os_}: no proxy, timezone reported={o['tz']} (host tz, expected)")
        app.stop_profile(pid)
        R.check(f"{os_}: stop -> not running", app.prof_row(pid)["running"] is None)
        R.check(f"{os_}: processes gone after stop", wait_gone(pid), procs_for(pid))

    # OS change on a pinned profile needs an explicit action
    mid = ids["mac"]
    s, b, _ = app.req("PUT", f"/api/profiles/{mid}", {"name": "s1-mac", "os": "win", "settings": app.profile(mid)["settings"]})
    R.check("os change without osAction refused", 400 <= s < 500, (s, b))
    app.edit(mid, os="win", osAction="keep")
    R.check("osAction=keep keeps machine/os", app.prof_row(mid)["os"] == "mac", app.prof_row(mid)["os"])
    app.edit(mid, os="win", osAction="regenerate")
    r = app.prof_row(mid)
    R.check("osAction=regenerate pins a win machine", r["os"] == "win" and r["fp"]["platform"] == "Win32", (r["os"], r["fp"]["platform"]))
    # delete
    for pid in ids.values():
        app.delete(f"/api/profiles/{pid}")
    R.check("deleted", not [p for p in app.overview()["profiles"] if p["id"] in ids.values()])


COOKIES = [
    {"name": "sid", "value": "abc123", "domain": "keep.example", "path": "/", "secure": False, "httpOnly": False, "expirationDate": 2000000000},
    {"name": "tok", "value": "zzz", "domain": "other.example", "path": "/", "secure": False, "httpOnly": False, "expirationDate": 2000000000},
]


def s2_cookies(c):
    R, app = c.R, c.main
    pid = app.create("s2-cookies", "lin")
    st = app.start_profile(pid)
    base = f"http://127.0.0.1:{c.page_port}"

    def setup(page, ctx):
        page.goto(base + "/c1")
        page.evaluate("document.cookie = 'persist=yes; max-age=100000; path=/'")
        return ctx.cookies()

    ck = with_page(c, st["ws"], setup)
    R.check("cookie set in browser", any(x["name"] == "persist" for x in ck), ck)
    app.stop_profile(pid)
    app.stop()  # restart the whole app
    app.start()
    st = app.start_profile(pid)
    ck = with_page(c, st["ws"], lambda p, ctx: ctx.cookies())
    R.check("cookie persists across app + profile restart", any(x["name"] == "persist" and x["value"] == "yes" for x in ck), ck)
    s, b, _ = app.req("GET", f"/api/cookies/{pid}/export")
    R.check("export refused while running", s >= 400, (s, b))
    app.stop_profile(pid)

    # import JSON (domains mode) - existing 127.0.0.1 cookie stays
    r = app.post(f"/api/cookies/{pid}/import", {"content": json.dumps(COOKIES), "mode": "domains"})
    R.check("import json ok", r.get("imported", r.get("count", 0)) >= 2 or r, r)
    s, body, hd = app.req("GET", f"/api/cookies/{pid}/export?format=json")
    names = {(x["name"], x["domain"].lstrip(".")) for x in body}
    R.check("export json has imported + pre-existing cookies", ("sid", "keep.example") in names and ("tok", "other.example") in names and ("persist", "127.0.0.1") in names, names)
    R.check("x-cookie-count header", hd.get("x-cookie-count") == str(len(body)), hd.get("x-cookie-count"))

    # netscape export, then domain-mode import of a file with only keep.example: other domains stay, keep.example replaced
    s, nets, hd = app.req("GET", f"/api/cookies/{pid}/export?format=netscape")
    R.check("netscape export looks like cookies.txt", s == 200 and "keep.example" in nets and "\t" in nets, nets[:200])
    ns = "# Netscape HTTP Cookie File\n.keep.example\tTRUE\t/\tFALSE\t2000000000\tsid\tNEWVALUE\n.keep.example\tTRUE\t/\tFALSE\t2000000000\tsid2\tv2\n"
    r = app.post(f"/api/cookies/{pid}/import", {"content": ns, "mode": "domains"})
    s, body, _ = app.req("GET", f"/api/cookies/{pid}/export?format=json")
    d = {(x["name"], x["domain"].lstrip(".")): x["value"] for x in body}
    R.check("domain mode: keep.example replaced by file", d.get(("sid", "keep.example")) == "NEWVALUE" and ("sid2", "keep.example") in d, d)
    R.check("domain mode: other domains kept", ("tok", "other.example") in d and ("persist", "127.0.0.1") in d, d)
    # all mode wipes the rest
    r = app.post(f"/api/cookies/{pid}/import", {"content": ns, "mode": "all"})
    s, body, _ = app.req("GET", f"/api/cookies/{pid}/export?format=json")
    d = {(x["name"], x["domain"].lstrip(".")) for x in body}
    R.check("all mode: only file cookies remain", d == {("sid", "keep.example"), ("sid2", "keep.example")}, d)
    # bad content
    s, b, _ = app.req("POST", f"/api/cookies/{pid}/import", {"content": "garbage that is neither", "mode": "domains"})
    R.check("garbage import rejected or reports skipped", s >= 400 or (isinstance(b, dict) and (b.get("imported", 0) == 0)), (s, b))
    # the browser sees imported cookies
    st = app.start_profile(pid)
    ck = with_page(c, st["ws"], lambda p, ctx: ctx.cookies(["http://keep.example/"]))
    R.check("browser sees imported cookie", any(x["name"] == "sid" and x["value"] == "NEWVALUE" for x in ck), ck)
    app.stop_profile(pid)
    app.delete(f"/api/profiles/{pid}")
