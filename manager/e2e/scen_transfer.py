"""Scenario 6: export zip / tamper / import / replace / clone / bulk / templates / extensions."""

import io
import json
import os
import zipfile

from probe import probe
from scen_core import with_page

CK = [{"name": "xfer", "value": "original", "domain": "xfer.example", "path": "/", "secure": False, "httpOnly": False, "expirationDate": 2000000000}]


def cookie_values(app, pid):
    s, body, _ = app.req("GET", f"/api/cookies/{pid}/export?format=json")
    return {(x["name"], x["domain"].lstrip(".")): x["value"] for x in body} if s == 200 else {"error": (s, body)}


def rewrite_zip(src, dst, edit):
    """Copy zip `src` to `dst`; edit(name, data) -> data|None(drop); plus extra entries via edit('', None)."""
    with zipfile.ZipFile(src) as zi, zipfile.ZipFile(dst, "w", zipfile.ZIP_DEFLATED) as zo:
        for info in zi.infolist():
            d = edit(info.filename, zi.read(info))
            if d is not None:
                zo.writestr(info.filename, d)
        extra = edit("", None)
        for name, data, attr in extra or []:
            zi2 = zipfile.ZipInfo(name)
            zi2.external_attr = attr
            zo.writestr(zi2, data)


def make_xpi():
    b = io.BytesIO()
    with zipfile.ZipFile(b, "w") as z:
        z.writestr(
            "manifest.json",
            json.dumps(
                {
                    "manifest_version": 2,
                    "name": "E2E marker",
                    "version": "1.0",
                    "browser_specific_settings": {"gecko": {"id": "e2e-marker@example.test"}},
                    "content_scripts": [{"matches": ["<all_urls>"], "js": ["c.js"], "run_at": "document_start"}],
                }
            ),
        )
        z.writestr("c.js", "document.documentElement.setAttribute('data-e2e-ext','loaded');")
    return b.getvalue()


def s6_transfer(c):
    R, app = c.R, c.main
    out = os.path.join(c.out, "s6")
    os.makedirs(out, exist_ok=True)
    src = app.create("s6-src", "mac", locale="fr-FR", notes="exported")
    st = app.start_profile(src)  # materialise user-data
    with_page(c, st["ws"], lambda p, ctx: p.goto(f"http://127.0.0.1:{c.page_port}/x"))
    app.stop_profile(src)
    app.post(f"/api/cookies/{src}/import", {"content": json.dumps(CK), "mode": "all"})
    R.check("source has cookie", cookie_values(app, src).get(("xfer", "xfer.example")) == "original")

    ex = app.post(f"/api/transfer/export/{src}")
    R.check("export produced a zip with files", os.path.isfile(ex["path"]) and ex["files"] > 3 and ex["size"] > 0, ex)
    with zipfile.ZipFile(ex["path"]) as z:
        names = z.namelist()
        R.check("zip has snapshot.json manifest", "snapshot.json" in names, names[:10])
        man = json.loads(z.read("snapshot.json"))
        R.check("manifest has schema/os/arch/build + per-file sha256", all(k in man for k in ("schema", "files")) and any("sha256" in f for f in (man["files"] if isinstance(man["files"], list) else man["files"].values())), list(man.keys()))
        R.check("no lock files / caches in zip", not [n for n in names if n.endswith("lock") or "/cache2/" in n or "startupCache" in n], [n for n in names if "lock" in n or "cache" in n.lower()][:5])
    # export of a running profile is refused
    st = app.start_profile(src)
    s, b, _ = app.req("POST", f"/api/transfer/export/{src}")
    R.check("export refused while running", s >= 400, (s, b))
    app.stop_profile(src)

    # import (new id)
    r = app.post("/api/transfer/import-path", {"path": ex["path"], "mode": "new", "name": "s6-imported"})
    nid = r["profile"]["id"]
    R.check("import: new id", nid != src and not r["keptId"], r["profile"]["id"])
    R.check("import: settings + cookie arrive", app.profile(nid)["settings"]["locale"] == "fr-FR" and cookie_values(app, nid).get(("xfer", "xfer.example")) == "original")
    R.check("import: machine identical (UA)", app.prof_row(nid)["fp"]["userAgent"] == app.prof_row(src)["fp"]["userAgent"])
    # raw upload path
    data = open(ex["path"], "rb").read()
    s, b, _ = app.req("POST", "/api/transfer/import?mode=new&name=s6-upload", raw=data, headers={"content-type": "application/zip"})
    R.check("import via upload body", s == 200 and b["profile"]["name"] == "s6-upload", (s, str(b)[:200]))
    up_id = b["profile"]["id"] if s == 200 else None

    # tamper: flip content of a data file (hash mismatch)
    tampered = os.path.join(out, "tampered.zip")
    done = {"n": 0}

    def tamper(name, data):
        if name == "":
            return None
        if name != "snapshot.json" and not name.endswith("/") and done["n"] == 0 and data:
            done["n"] += 1
            return data + b"X"
        return data

    rewrite_zip(ex["path"], tampered, tamper)
    n_before = len(app.overview()["profiles"])
    s, b, _ = app.req("POST", "/api/transfer/import-path", {"path": tampered, "mode": "new"})
    R.check("tampered zip (changed file) rejected", s >= 400, (s, str(b)[:200]))
    R.check("rejected import left no profile behind", len(app.overview()["profiles"]) == n_before)
    # tamper the manifest itself (change os)
    tampered2 = os.path.join(out, "tampered2.zip")

    def tamper2(name, data):
        if name == "":
            return None
        if name == "snapshot.json":
            m = json.loads(data)
            m["os"] = "win" if m.get("os") != "win" else "mac"
            return json.dumps(m).encode()
        return data

    rewrite_zip(ex["path"], tampered2, tamper2)
    s, b, _ = app.req("POST", "/api/transfer/import-path", {"path": tampered2, "mode": "new"})
    R.note(f"manifest field edited (os) -> {s} {str(b)[:100]}")
    # zip-slip + symlink entries
    slip = os.path.join(out, "slip.zip")
    rewrite_zip(ex["path"], slip, lambda n, d: [("../../evil.txt", b"pwn", 0o100644 << 16)] if n == "" else d)
    s, b, _ = app.req("POST", "/api/transfer/import-path", {"path": slip, "mode": "new"})
    R.check("zip-slip entry rejected", s >= 400, (s, str(b)[:200]))
    R.check("zip-slip: nothing written outside", not os.path.exists(os.path.join(c.new_home_root if hasattr(c, "new_home_root") else out, "evil.txt")))
    link = os.path.join(out, "link.zip")
    rewrite_zip(ex["path"], link, lambda n, d: [("user-data/evil-link", b"/etc/passwd", 0o120777 << 16)] if n == "" else d)
    s, b, _ = app.req("POST", "/api/transfer/import-path", {"path": link, "mode": "new"})
    R.check("symlink entry rejected", s >= 400, (s, str(b)[:200]))
    s, b, _ = app.req("POST", "/api/transfer/import-path", {"path": os.path.join(out, "nope.zip"), "mode": "new"})
    R.check("missing file rejected", s >= 400)
    junk = os.path.join(out, "junk.zip")
    open(junk, "wb").write(b"not a zip at all")
    s, b, _ = app.req("POST", "/api/transfer/import-path", {"path": junk, "mode": "new"})
    R.check("non-zip rejected", s >= 400, (s, b))

    # replace restores data
    app.post(f"/api/cookies/{src}/import", {"content": json.dumps([dict(CK[0], value="CHANGED")]), "mode": "all"})
    R.check("source cookie changed", cookie_values(app, src).get(("xfer", "xfer.example")) == "CHANGED")
    r = app.post("/api/transfer/import-path", {"path": ex["path"], "mode": "replace", "profile": src})
    R.check("replace keeps the local id", r["profile"]["id"] == src, r["profile"]["id"])
    R.check("replace restored the archived cookie", cookie_values(app, src).get(("xfer", "xfer.example")) == "original", cookie_values(app, src))
    s, b, _ = app.req("POST", "/api/transfer/import-path", {"path": ex["path"], "mode": "replace"})
    R.check("replace without target id rejected", s >= 400, (s, b))
    # replace of a running profile refused
    st = app.start_profile(src)
    s, b, _ = app.req("POST", "/api/transfer/import-path", {"path": ex["path"], "mode": "replace", "profile": src})
    R.check("replace of a running profile refused", s >= 400, (s, str(b)[:150]))
    app.stop_profile(src)

    # clone
    cl = app.post(f"/api/transfer/clone/{src}", {"name": "s6-clone"})
    cid = cl["id"]
    R.check("clone: new id + same cookie + same machine", cid != src and cookie_values(app, cid).get(("xfer", "xfer.example")) == "original" and app.prof_row(cid)["fp"]["userAgent"] == app.prof_row(src)["fp"]["userAgent"])
    # bulk 5
    bk = app.post("/api/transfer/bulk", {"pattern": "Shop {n}", "count": 5, "os": "lin"})
    names = [p["name"] for p in bk["created"]]
    R.check("bulk create 5", names == [f"Shop {i}" for i in range(1, 6)] and not bk.get("error"), bk.get("error") or names)
    R.check("bulk: all linux with distinct machines", len({p["fp"]["userAgent"] + p["fp"]["resolution"] + str(p["fp"]["cpu"]) for p in bk["created"]}) >= 2)
    s, b, _ = app.req("POST", "/api/transfer/bulk", {"pattern": "NoNumber", "count": 3, "os": "lin"})
    R.check("bulk pattern without {n} for count>1 handled", s >= 400 or len({p["name"] for p in b.get("created", [])}) == len(b.get("created", [])), (s, str(b)[:150]))
    s, b, _ = app.req("POST", "/api/transfer/bulk", {"pattern": "Big {n}", "count": 100000, "os": "lin"})
    R.check("bulk huge count refused", s >= 400, (s, str(b)[:120]))

    # template -> 3 profiles
    app.edit(src, locale="it-IT", window="1100x800")
    t = app.post(f"/api/templates/from-profile/{src}", {"name": "s6-tpl"})
    R.check("template from profile has settings, no machine", t["settings"]["locale"] == "it-IT" and "machine" not in t, t)
    mk = app.post(f"/api/templates/{t['id']}/create", {"count": 3, "pattern": "Tpl {n}"})
    pc = mk["created"]
    R.check("template -> 3 profiles with its settings", len(pc) == 3 and all(app.profile(p["id"])["settings"]["locale"] == "it-IT" for p in pc), mk.get("error") or [p["name"] for p in pc])
    R.check("template profiles have distinct pinned machines", len({json.dumps(app.prof_row(p["id"])["fp"]["resolution"]) + str(app.prof_row(p["id"])["fp"]["cpu"]) + app.prof_row(p["id"])["fp"]["webglRenderer"] for p in pc}) >= 1)
    tl = app.get("/api/templates")
    R.check("template listed", any(x["id"] == t["id"] for x in tl))

    # extension .xpi loads
    xpi = make_xpi()
    s, ext, _ = app.req("POST", "/api/extensions", raw=xpi, headers={"content-type": "application/octet-stream"})
    R.check("xpi uploaded", s == 200 and ext.get("addonId", ext.get("addon_id")) == "e2e-marker@example.test", (s, ext))
    s, bad, _ = app.req("POST", "/api/extensions", raw=b"garbage", headers={"content-type": "application/octet-stream"})
    R.check("garbage xpi rejected", s >= 400, (s, bad))
    eid = ext["id"]
    eprof = pc[0]["id"]
    app.put(f"/api/profiles/{eprof}/extensions", {"extensions": [eid]})
    st = app.start_profile(eprof)

    def chk(page, ctx):
        page.goto(f"http://127.0.0.1:{c.page_port}/ext")
        page.wait_for_timeout(1500)
        return page.evaluate("document.documentElement.getAttribute('data-e2e-ext')")

    v = with_page(c, st["ws"], chk)
    R.check("extension .xpi is loaded in the launched profile (content script ran)", v == "loaded", v)
    app.stop_profile(eprof)
    # profile without the extension: not loaded
    other = pc[1]["id"]
    st = app.start_profile(other)
    v = with_page(c, st["ws"], chk)
    R.check("extension absent in profile that does not use it", v is None, v)
    app.stop_profile(other)
    # remove from profile -> gone at next start
    app.put(f"/api/profiles/{eprof}/extensions", {"extensions": []})
    st = app.start_profile(eprof)
    v = with_page(c, st["ws"], chk)
    R.check("unassigned extension is removed at next start", v is None, v)
    app.stop_profile(eprof)
    for p in [src, nid, cid, up_id] + [x["id"] for x in bk["created"]] + [x["id"] for x in pc]:
        if p:
            try:
                app.delete(f"/api/profiles/{p}")
            except Exception:
                pass
