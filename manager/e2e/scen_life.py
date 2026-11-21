"""Scenarios 8-9: crash recovery, leases, two instances on one HOME, scheduling."""

import time

from lib import App, dead_port
from scen_core import procs_for, wait_gone, with_page


def wait_for(fn, timeout=30, step=0.5):
    t0 = time.time()
    while time.time() - t0 < timeout:
        v = fn()
        if v:
            return v
        time.sleep(step)
    return None


def s8_lifecycle(c):
    R, app = c.R, c.main
    pid = app.create("s8-crash", "lin")
    st = app.start_profile(pid)
    R.check("running", bool(app.prof_row(pid)["running"]) and procs_for(pid))
    app.kill9()
    time.sleep(1.5)
    left = procs_for(pid)
    R.note(f"after kill -9 of the app: {len(left)} leftover process lines for the profile")
    app.start()
    gone = wait_gone(pid, 40)
    R.check("orphaned helper + browser cleaned after restart", gone, procs_for(pid)[:3])
    row = app.prof_row(pid)
    R.check("profile not running after restart", row["running"] is None, row["running"])
    R.check("profile shown as crashed", row.get("crashed") is not None, row.get("crashed"))
    st2 = app.start_profile(pid)
    R.check("crashed profile is startable", st2.get("ws", "").startswith("ws://"), st2)
    R.check("crashed marker cleared after a clean start", app.prof_row(pid).get("crashed") is None, app.prof_row(pid).get("crashed"))
    app.stop_profile(pid)

    # graceful SIGTERM of the app stops browsers
    st = app.start_profile(pid)
    app.stop()
    R.check("SIGTERM of the app stops its browsers", wait_gone(pid, 30), procs_for(pid)[:3])
    app.start()
    R.check("after graceful shutdown the profile is not 'crashed'", app.prof_row(pid).get("crashed") is None, app.prof_row(pid).get("crashed"))

    # lease
    s, b, _ = app.req("POST", f"/api/profiles/{pid}/lease", {"holder": "agent-a", "ttlSecs": 60})
    R.check("lease acquired", s == 200, (s, b))
    s, b, _ = app.req("POST", f"/api/profiles/{pid}/lease", {"holder": "agent-b", "ttlSecs": 60})
    R.check("second holder -> 409", s == 409, (s, b))
    s, b, _ = app.req("POST", f"/api/profiles/{pid}/start")
    R.check("start without lease holder -> 409", s == 409, (s, b))
    s, b, _ = app.req("POST", f"/api/profiles/{pid}/start", headers={"X-Lease-Holder": "agent-b"})
    R.check("start by the wrong holder -> 409", s == 409, (s, b))
    R.check("lease visible in overview", (app.prof_row(pid).get("lease") or {}).get("holder") == "agent-a", app.prof_row(pid).get("lease"))
    s, b, _ = app.req("POST", f"/api/profiles/{pid}/start", headers={"X-Lease-Holder": "agent-a"})
    R.check("start by the holder works", s == 200, (s, b))
    s, b, _ = app.req("POST", f"/api/profiles/{pid}/lease/renew", {"holder": "agent-a", "ttlSecs": 60})
    R.check("holder can renew", s == 200, (s, b))
    s, b, _ = app.req("POST", f"/api/profiles/{pid}/lease/renew", {"holder": "agent-b", "ttlSecs": 60})
    R.check("non-holder cannot renew", s >= 400, (s, b))
    app.stop_profile(pid)
    R.check("stop releases the lease", app.get(f"/api/profiles/{pid}/lease")["lease"] is None)
    app.post(f"/api/profiles/{pid}/lease", {"holder": "agent-a", "ttlSecs": 2})
    time.sleep(3.5)
    s, b, _ = app.req("POST", f"/api/profiles/{pid}/lease", {"holder": "agent-b", "ttlSecs": 60})
    R.check("expired lease can be taken by another holder", s == 200, (s, b))
    app.delete(f"/api/profiles/{pid}/lease")

    # two instances on the same HOME
    st = app.start_profile(pid)
    app2 = App(app.home, 7922, 47922, 52000, name="app2")
    c.apps.append(app2)
    app2.start()
    time.sleep(2)
    R.check("2nd instance startup did not kill the 1st instance's browser", bool(procs_for(pid)) and bool(app.prof_row(pid)["running"]), procs_for(pid)[:2])
    ok = False
    try:
        with_page(c, st["ws"], lambda p, ctx: p.evaluate("1+1"))
        ok = True
    except Exception as e:
        R.note(f"connect failed: {e}")
    R.check("1st instance's browser still drivable", ok)
    s, b, _ = app2.req("POST", f"/api/profiles/{pid}/start")
    R.check("2nd instance cannot start the same profile (refused)", s >= 400, (s, str(b)[:200]))
    R.check("2nd instance refusal explains why", s >= 400 and any(w in str(b).lower() for w in ("running", "lock", "another", "in use", "open")), b)
    time.sleep(1)
    R.check("1st browser survived the refused start", bool(procs_for(pid)))
    app.stop_profile(pid)
    s, b, _ = app2.req("POST", f"/api/profiles/{pid}/start")
    R.check("after stop, 2nd instance can start it", s == 200, (s, str(b)[:200]))
    s, b, _ = app.req("POST", f"/api/profiles/{pid}/start")
    R.check("now the 1st instance is refused", s >= 400, (s, str(b)[:200]))
    app2.stop_profile(pid)
    app2.stop()
    app.delete(f"/api/profiles/{pid}")


def wait_run(app, sid, timeout=120):
    def done():
        runs = app.get(f"/api/schedules/{sid}/runs")
        return runs if runs and runs[0]["outcome"] != "running" else None

    return wait_for(done, timeout, 1.0)


def s9_schedule(c):
    R, app = c.R, c.main
    pid = app.create("s9-warm", "lin")
    sched = {
        "name": "warm-now",
        "target": {"type": "profile", "id": pid},
        "action": {"type": "warm", "urls": [f"http://127.0.0.1:{c.page_port}/warm-one", f"http://127.0.0.1:{c.page_port}/warm-two"], "dwellSecs": 5},
        "recurrence": {"type": "daily", "at": "03:00"},
        "timezone": "UTC",
        "enabled": False,
    }
    s, b, _ = app.req("POST", "/api/schedules", dict(sched, recurrence={"type": "daily", "at": "25:99"}))
    R.check("bad time rejected", s >= 400, (s, b))
    s, b, _ = app.req("POST", "/api/schedules", dict(sched, timezone="Mars/Base"))
    R.check("bad timezone rejected", s >= 400, (s, b))
    sc = app.post("/api/schedules", sched)
    R.check("schedule created", sc["id"] > 0, sc)
    n0 = len(c.page_log)
    app.post(f"/api/schedules/{sc['id']}/run")
    runs = wait_run(app, sc["id"])
    R.check("warming run finished ok", bool(runs) and runs[0]["outcome"] == "ok", runs[:1] if runs else "timeout")
    seen = [p for h, p in c.page_log[n0:]]
    R.check("warming visited both URLs", "/warm-one" in seen and "/warm-two" in seen, seen)
    R.check("run recorded start/end/trigger", bool(runs) and runs[0]["startedAt"] and runs[0]["finishedAt"] and runs[0]["trigger"] == "manual", runs[:1] if runs else None)
    R.check("profile closed after the warming run", wait_gone(pid, 30) and app.prof_row(pid)["running"] is None, procs_for(pid)[:2])
    R.check("lease released after the run", app.get(f"/api/profiles/{pid}/lease")["lease"] is None)

    # precheck: dead proxy -> failed, no browser
    dp = app.post("/api/proxies", {"name": "s9-dead", "scheme": "http", "host": "127.0.0.1", "port": dead_port()})
    app.post(f"/api/profiles/{pid}/assign", {"proxyId": dp["id"]})
    n_runs = len(app.get(f"/api/schedules/{sc['id']}/runs"))
    n0 = len(c.page_log)
    app.post(f"/api/schedules/{sc['id']}/run")
    runs = wait_for(lambda: (lambda r: r if len(r) > n_runs and r[0]["outcome"] != "running" else None)(app.get(f"/api/schedules/{sc['id']}/runs")), 90, 1.0)
    R.check("dead proxy precheck -> run failed", bool(runs) and runs[0]["outcome"] == "failed", runs[:1] if runs else "timeout")
    R.check("failed run message mentions the proxy", bool(runs) and "proxy" in (runs[0].get("message") or "").lower(), runs[0].get("message") if runs else None)
    R.check("no browser started / no page hit on precheck failure", not procs_for(pid) and len(c.page_log) == n0, (procs_for(pid)[:1], c.page_log[n0:]))
    notes = app.get("/api/notifications")
    items = notes.get("items", notes) if isinstance(notes, dict) else notes
    R.check("notification raised for the failed run", any("schedule" in str(i.get("kind", "")) for i in items), str(items)[:200])
    app.post(f"/api/profiles/{pid}/assign", {"proxyId": None})
    app.delete(f"/api/proxies/{dp['id']}")

    # precondition profile_closed: running profile -> waits, then fails/skips
    sc2 = app.put(f"/api/schedules/{sc['id']}", dict(sched, name="warm-busy", preconditions={"profileClosed": True, "proxyCheck": False, "timeoutSecs": 4}))
    app.start_profile(pid)
    n_runs = len(app.get(f"/api/schedules/{sc['id']}/runs"))
    app.post(f"/api/schedules/{sc['id']}/run")
    runs = wait_for(lambda: (lambda r: r if len(r) > n_runs and r[0]["outcome"] != "running" else None)(app.get(f"/api/schedules/{sc['id']}/runs")), 60, 1.0)
    R.check("busy profile: run not ok after the timeout", bool(runs) and runs[0]["outcome"] in ("failed", "skipped"), runs[:1] if runs else "timeout")
    R.check("busy profile: user's browser was not closed by the scheduler", bool(app.prof_row(pid)["running"]))
    app.stop_profile(pid)

    # refresh_browser
    sc3 = app.post("/api/schedules", dict(sched, name="refresh", action={"type": "refresh_browser"}, target={"type": "profile", "id": pid}))
    app.post(f"/api/schedules/{sc3['id']}/run")
    runs = wait_run(app, sc3["id"], 60)
    R.check("browser refresh job ok", bool(runs) and runs[0]["outcome"] == "ok", runs[:1] if runs else "timeout")
    # enable/disable + listing
    app.post(f"/api/schedules/{sc['id']}/enabled", {"enabled": True})
    lst = app.get("/api/schedules")
    R.check("schedule listed with next run", any(x["id"] == sc["id"] and x["enabled"] for x in lst), str(lst)[:200])
    app.post(f"/api/schedules/{sc['id']}/enabled", {"enabled": False})
    allruns = app.get("/api/schedules/runs")
    R.check("history survives (all runs list)", len(allruns) >= 3)
    app.delete(f"/api/schedules/{sc['id']}")
    app.delete(f"/api/schedules/{sc3['id']}")
    app.delete(f"/api/profiles/{pid}")
