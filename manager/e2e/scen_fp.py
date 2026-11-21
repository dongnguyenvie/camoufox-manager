"""Scenario 5: canvas / WebGL / audio fingerprints are unique per profile and stable per profile."""

from probe import probe
from scen_core import with_page

CORE = ("canvasData", "webgl", "audio")  # the readbacks the init script makes unique+stable
KEYS = ("canvasURL", "canvasSimpleURL", "canvasBlob", "canvasData", "webgl", "audio")


def launch_probe(c, pid):
    app = c.main
    st = app.start_profile(pid)
    try:
        def both(p, ctx):
            first = probe(p, f"http://127.0.0.1:{c.page_port}/probe")
            p2 = ctx.new_page()  # a tab the user opens later lives in the same context
            first["newpage"] = probe(p2, f"http://127.0.0.1:{c.page_port}/probe")
            p2.close()
            return first

        return with_page(c, st["ws"], both)
    finally:
        app.stop_profile(pid)


def s5_fingerprint(c):
    R, app = c.R, c.main
    a = app.create("s5-a", "win")
    b = app.create("s5-b", "win")
    R.check("default canvas mode is stable", app.profile(a)["settings"]["canvas"] == "stable")
    a1, a2 = launch_probe(c, a), launch_probe(c, a)
    b1, b2 = launch_probe(c, b), launch_probe(c, b)
    for r_ in (a1, a2, b1, b2):
        for k in KEYS:
            if (k + "Err") in r_ or r_.get(k) is None:  # audio is only measurable in the main world
                R.check(f"probe produced {k}", False, r_.get(k + "Err"))
    R.check("repeated toDataURL in one session agrees", a1["canvasSimpleURL"] == a1["canvasSimpleURL2"])
    for k in KEYS:
        R.check(f"{k}: stable across 2 launches (profile A)", a1[k] == a2[k], (a1[k], a2[k]))
        R.check(f"{k}: stable across 2 launches (profile B)", b1[k] == b2[k], (b1[k], b2[k]))
        R.check(f"{k}: differs between profile A and B", a1[k] != b1[k], (a1[k], b1[k]))
    for k in ("canvasData", "webgl", "audio", "canvasURL"):
        R.check(f"{k}: first tab and a new tab of the same profile agree", a1[k] == a1["newpage"][k], (a1[k], a1["newpage"][k]))
    R.check("same hardware config pinned per profile (GL renderer stable)", a1["glRenderer"] == a2["glRenderer"], (a1["glRenderer"], a2["glRenderer"]))
    R.note(f"A: {[a1[k] for k in KEYS]}  B: {[b1[k] for k in KEYS]}")

    # stable across an app restart too
    app.stop()
    app.start()
    a3 = launch_probe(c, a)
    R.check("canvas/webgl/audio stable across an app restart", all(a1[k] == a3[k] for k in CORE), (a1, a3))

    # native mode: identical on every profile (documented), random: changes per launch
    app.edit(a, canvas="native")
    app.edit(b, canvas="native")
    n1, n2 = launch_probe(c, a), launch_probe(c, b)
    R.check("native mode: A and B share the host canvas/webgl hash (documented behaviour)", (n1["canvasData"], n1["webgl"]) == (n2["canvasData"], n2["webgl"]), (n1["canvasData"], n2["canvasData"]))
    R.check("native mode differs from the unique stable one", n1["canvasData"] != a1["canvasData"])
    app.edit(a, canvas="random")
    r1, r2 = launch_probe(c, a), launch_probe(c, a)
    R.check("random mode: differs between launches", r1["canvasData"] != r2["canvasData"] or r1["webgl"] != r2["webgl"], (r1["canvasData"], r2["canvasData"]))
    # back to stable: returns to the same unique value as before (seed pinned in the machine)
    app.edit(a, canvas="stable")
    a4 = launch_probe(c, a)
    R.check("back to stable: same hash as before switching modes", all(a1[k] == a4[k] for k in CORE), (a1, a4))
    # export + clone keeps the seed? clone (same machine) -> same hash; new machine -> different
    cl = app.post(f"/api/transfer/clone/{a}", {"name": "s5-clone"})
    cid = cl.get("id") or cl.get("profile", {}).get("id")
    c1 = launch_probe(c, cid)
    R.check("clone (same machine) has the same canvas as the source (same pinned seed)", c1["canvasData"] == a1["canvasData"], (c1["canvasData"], a1["canvasData"]))
    cl2 = app.post(f"/api/transfer/clone/{a}", {"name": "s5-clone-new", "newMachine": True})
    cid2 = cl2.get("id") or cl2.get("profile", {}).get("id")
    c2 = launch_probe(c, cid2)
    R.check("clone with new machine has a different canvas", c2["canvasData"] != a1["canvasData"], (c2["canvasData"], a1["canvasData"]))
    for p in (a, b, cid, cid2):
        app.delete(f"/api/profiles/{p}")
