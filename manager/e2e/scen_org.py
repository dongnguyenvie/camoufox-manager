"""Scenario 7: tags / groups / views / bulk actions, sheets export -> edit -> dry-run -> apply."""

import csv
import io
import json
import time

from scen_core import procs_for


def s7_org(c):
    R, app = c.R, c.main
    ids = [app.create(f"s7-{i}", "lin") for i in range(4)]
    t1 = app.post("/api/tags", {"name": "alpha", "color": "#ff0000"})
    t2 = app.post("/api/tags", {"name": "beta"})
    s, b, _ = app.req("POST", "/api/tags", {"name": "alpha"})
    R.check("duplicate tag name rejected", s >= 400, (s, b))
    s, b, _ = app.req("POST", "/api/tags", {"name": "  "})
    R.check("empty tag name rejected", s >= 400, (s, b))
    s, b, _ = app.req("POST", "/api/tags", {"name": "x", "color": "javascript:alert(1)"})
    R.check("bad tag color rejected", s >= 400, (s, b))
    g = app.post("/api/groups", {"name": "grp-one"})
    r = app.post("/api/bulk", {"action": "tag", "profileIds": ids[:3], "tagId": t1["id"]})
    R.check("bulk tag 3 profiles", r["ok"] == 3 and r["failed"] == 0, r)
    app.post("/api/bulk", {"action": "tag", "profileIds": ids[:2], "tagId": t2["id"]})
    r = app.post("/api/bulk", {"action": "group", "profileIds": ids[1:3], "groupId": g["id"]})
    org = app.get("/api/org")
    R.check("org overview: many tags per profile", sorted(org["profileTags"][ids[0]]) == sorted([t1["id"], t2["id"]]), org["profileTags"])
    R.check("org overview: one group per profile", org["profileGroups"].get(ids[1]) == g["id"] and ids[0] not in org["profileGroups"], org["profileGroups"])
    r = app.post("/api/bulk", {"action": "untag", "profileIds": [ids[0]], "tagId": t2["id"]})
    R.check("bulk untag", t2["id"] not in app.get("/api/org")["profileTags"][ids[0]])
    v = app.post("/api/views", {"name": "view1", "query": {"tag": t1["id"], "sort": "name"}})
    R.check("saved view roundtrip", any(x["id"] == v["id"] and x["query"]["sort"] == "name" for x in app.get("/api/views")))

    # bulk start + stop + errors for unknown ids
    r = app.post("/api/bulk", {"action": "start", "profileIds": ids[:3] + ["ffffffffffffffffffffffff"]})
    R.check("bulk start: 3 ok, unknown id reported", r["ok"] == 3 and r["failed"] == 1 and any(x["reason"] == "not-found" for x in r["results"]), r)
    R.check("bulk start: all 3 running", all(app.prof_row(i)["running"] for i in ids[:3]))
    r = app.post("/api/bulk", {"action": "start", "profileIds": [ids[0]]})
    R.note(f"bulk start of an already running profile -> {r['ok']}/{r['failed']} {r['results']}")
    r = app.post("/api/bulk", {"action": "stop", "profileIds": ids[:3]})
    R.check("bulk stop: 3 ok", r["ok"] == 3, r)
    R.check("bulk stop: no processes left", all(not procs_for(i) for i in ids[:3]) or time.sleep(3) or all(not procs_for(i) for i in ids[:3]))
    victim = app.create("s7-victim", "lin")
    app.post("/api/bulk", {"action": "start", "profileIds": [victim]})
    r = app.post("/api/bulk", {"action": "delete", "profileIds": [victim]})
    R.check("bulk delete of a RUNNING profile stops it, then deletes profile + data (by design)", r["ok"] == 1 and victim not in {p["id"] for p in app.overview()["profiles"]} and not procs_for(victim), r)
    r = app.post("/api/bulk", {"action": "stop", "profileIds": [ids[3]]})
    R.note(f"bulk stop of not running -> {r['results']}")
    s, b, _ = app.req("POST", "/api/bulk", {"action": "nonsense", "profileIds": ids})
    R.check("bulk unknown action rejected", s >= 400, (s, str(b)[:100]))
    # deleting a tag / group removes assignments
    app.delete(f"/api/tags/{t2['id']}")
    R.check("delete tag removes assignments", all(t2["id"] not in v for v in app.get("/api/org")["profileTags"].values()))
    app.delete(f"/api/groups/{g['id']}")
    R.check("delete group clears membership", not app.get("/api/org")["profileGroups"])

    # ---- sheets ----
    s, body, hd = app.req("GET", "/api/sheets/export?format=csv")
    R.check("csv export", s == 200 and isinstance(body, str) and body.startswith("id,name") or "id" in str(body)[:60], str(body)[:100])
    R.note(f"csv BOM: {body.startswith(chr(0xfeff))}")
    rows = list(csv.DictReader(io.StringIO(body.lstrip(chr(0xfeff)))))
    R.check("csv has all profiles", {r["id"] for r in rows} >= set(ids), len(rows))
    s, xl, _ = app.req("GET", "/api/sheets/export?format=xlsx")
    R.check("xlsx export is a zip", s == 200 and str(xl).startswith("PK"), str(xl)[:10])
    # edit: rename ids[0], set notes, set tags, add a new row
    for r_ in rows:
        if r_["id"] == ids[0]:
            r_["name"] = "s7-renamed"
            r_["notes"] = "via sheet"
            r_["tags"] = "alpha,gamma"
            r_["timezone"] = "Europe/Paris"
    rows.append({"id": "", "name": "s7-from-sheet", "os": "win", "notes": "new row", "group": "sheetgrp", "tags": "alpha"})
    out = io.StringIO()
    w = csv.DictWriter(out, fieldnames=list(rows[0].keys()) if rows else [], extrasaction="ignore", restval="")
    w.writeheader()
    for r_ in rows:
        w.writerow(r_)
    data = (chr(0xfeff) + out.getvalue()).encode()  # Excel saves CSV with a BOM
    n_before = len(app.overview()["profiles"])
    s, pv, _ = app.req("POST", "/api/sheets/import/preview?format=csv&createMissing=true", raw=data, headers={"content-type": "text/csv"})
    R.check("preview ok", s == 200, (s, str(pv)[:300]))
    if s != 200:
        return
    R.check("preview: dry run changed nothing", len(app.overview()["profiles"]) == n_before and app.profile(ids[0])["name"] == "s7-0")
    R.check("preview: 1 update + 1 create", pv["summary"]["update"] == 1 and pv["summary"]["create"] == 1, pv["summary"])
    R.check("preview: new tag/group announced", "gamma" in pv["newTags"] and "sheetgrp" in pv["newGroups"], (pv["newTags"], pv["newGroups"]))
    ap = app.post("/api/sheets/import/apply", {"token": pv["token"], "createMissing": True})
    R.check("apply: created 1, updated 1", ap["created"] == 1 and ap["updated"] == 1 and not ap["failed"], ap)
    p0 = app.profile(ids[0])
    R.check("apply: name/notes/timezone updated", p0["name"] == "s7-renamed" and p0["notes"] == "via sheet" and p0["settings"]["timezone"] == "Europe/Paris", (p0["name"], p0["notes"], p0["settings"]["timezone"]))
    org = app.get("/api/org")
    tag_names = {t["id"]: t["name"] for t in org["tags"]}
    R.check("apply: tags set from sheet (alpha, gamma)", sorted(tag_names[t] for t in org["profileTags"][ids[0]]) == ["alpha", "gamma"], org["profileTags"].get(ids[0]))
    R.check("apply: new profile created with OS win", any(p["name"] == "s7-from-sheet" and p["os"] == "win" for p in app.overview()["profiles"]))
    s, b, _ = app.req("POST", "/api/sheets/import/apply", {"token": pv["token"], "createMissing": True})
    R.check("token cannot be applied twice (or is idempotent)", s >= 400 or b.get("created") == 0, (s, str(b)[:150]))
    # re-export, unmodified -> everything unchanged
    s, body2, _ = app.req("GET", "/api/sheets/export?format=csv")
    s, pv2, _ = app.req("POST", "/api/sheets/import/preview?format=csv", raw=body2.encode(), headers={"content-type": "text/csv"})
    R.check("re-import of an untouched export = no changes", s == 200 and pv2["summary"]["update"] == 0 and pv2["summary"]["create"] == 0 and pv2["summary"]["errors"] == 0, (s, pv2.get("summary") if isinstance(pv2, dict) else pv2))
    # errors in dry-run: bad timezone / bad webrtc / bad id
    bad = "id,name,timezone,webrtc\n%s,ok-name,Mars/Base,bogus\nzzzz,x,,\n" % ids[1]
    s, pv3, _ = app.req("POST", "/api/sheets/import/preview?format=csv", raw=bad.encode(), headers={"content-type": "text/csv"})
    R.check("dry run flags bad rows as errors, apply skips them", s == 200 and pv3["summary"]["errors"] >= 1, (s, str(pv3)[:300]))
    if s == 200:
        ap = app.post("/api/sheets/import/apply", {"token": pv3["token"]})
        R.check("apply skipped the error rows", ap["skipped"] >= 1 and app.profile(ids[1])["name"] == "s7-1", ap)
    # xlsx untouched re-import
    s, pvx, _ = app.req("POST", "/api/sheets/import/preview?format=xlsx", raw=_bytes_from_export(app), headers={"content-type": "application/octet-stream"})
    R.check("xlsx re-import of untouched export = no changes", s == 200 and pvx["summary"]["update"] == 0 and pvx["summary"]["create"] == 0, (s, str(pvx)[:200]))
    for p in app.overview()["profiles"]:
        if p["name"].startswith("s7-"):
            app.delete(f"/api/profiles/{p['id']}")


def _bytes_from_export(app):
    import urllib.request

    return urllib.request.urlopen(app.base + "/api/sheets/export?format=xlsx").read()
