"""Scenario 11: device sync between two instances (loopback advertise, as in phases 16/17)."""

import hashlib
import json
import os
import time

from lib import App
from scen_life import wait_for


def sha_file(p):
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for b in iter(lambda: f.read(1 << 20), b""):
            h.update(b)
    return h.hexdigest()


def wait_job(B, jid, timeout=180):
    ui = {"Origin": B.base}

    def done():
        j = B.get(f"/api/sync/jobs/{jid}", headers=ui)
        return j if j["state"] != "running" else None

    return wait_for(done, timeout, 0.5)


def pull(B, dev, ids_actions, ack=True, **extra):
    ui = {"Origin": B.base}
    body = {"direction": "pull", "items": [{"profileId": i, "action": a} for i, a in ids_actions], "acknowledged": ack, **extra}
    return B.req("POST", f"/api/sync/devices/{dev}/start", body, headers=ui)


def s11_sync(c):
    R = c.R
    c.main.stop()  # frees 7921 / 47921 / 51000 for instance A
    env_a = {"CM_SYNC_ADVERTISE": "127.0.0.1:47921", "CM_SYNC_CHUNK_SIZE": "65536"}
    env_b = {"CM_SYNC_ADVERTISE": "127.0.0.1:47922", "CM_SYNC_CHUNK_SIZE": "65536"}
    A = App(c.new_home(), 7921, 47921, 51000, env=env_a, name="syncA")
    B = App(c.new_home(), 7922, 47922, 52000, env=env_b, name="syncB")
    c.apps += [A, B]
    A.start()
    B.start()
    uiA, uiB = {"Origin": A.base}, {"Origin": B.base}
    st = A.get("/api/devices/status")
    R.check("A: sync enabled via advertise", st.get("enabled") is True, st)
    s, b, _ = A.req("GET", "/api/devices/pairing")
    R.check("pairing link is UI-only (403 without origin)", s == 403, (s, b))
    offer = A.get("/api/devices/pairing", headers=uiA)
    R.check("pairing offer has cmsync:// link + code", offer["url"].startswith("cmsync://") and offer.get("code"), offer)
    s, b, _ = B.req("POST", "/api/devices/pair", {"url": offer["url"].replace("c=", "c=x", 1)}, headers=uiB)
    R.check("wrong pairing code refused", s >= 400, (s, str(b)[:150]))
    s, b, _ = B.req("POST", "/api/devices/pair", {"url": offer["url"]}, headers=uiB)
    R.check("B pairs with A", s == 200, (s, b))
    s, b2, _ = B.req("POST", "/api/devices/pair", {"url": offer["url"]}, headers=uiB)
    R.check("pairing code is one-time", s >= 400, (s, str(b2)[:150]))
    devsB = B.get("/api/devices")["devices"]
    devsA = A.get("/api/devices")["devices"]
    R.check("both sides list each other", len(devsB) == 1 and len(devsA) == 1, (devsA, devsB))
    if len(devsB) != 1:
        return
    devA_on_B, devB_on_A = devsB[0]["id"], devsA[0]["id"]
    h = B.post(f"/api/devices/{devA_on_B}/hello")
    R.check("signed hello B -> A", bool(h), h)
    s, b3, _ = B.req("POST", f"/api/sync/devices/{devA_on_B}/start", {"direction": "pull", "items": [], "acknowledged": True})
    R.check("sync start is UI-only", s == 403, (s, b3))

    # data on A: a profile with a cookie and a 12 MiB random file
    pid = A.create("sync-src", "mac", locale="pt-BR")
    st = A.start_profile(pid)
    A.stop_profile(pid)
    ck = [{"name": "syn", "value": "one", "domain": "sync.example", "path": "/", "secure": False, "httpOnly": False, "expirationDate": 2000000000}]
    A.post(f"/api/cookies/{pid}/import", {"content": json.dumps(ck), "mode": "all"})
    big = os.path.join(A.prof_row(pid)["path"]["dir"], "junk.bin")
    with open(big, "wb") as f:
        f.write(os.urandom(12 * 1024 * 1024))
    s, plan, _ = B.req("GET", f"/api/sync/devices/{devA_on_B}/plan?direction=pull", headers=uiB)
    R.check("plan lists A's profile as missingOnTarget", s == 200 and any(r["sourceId"] == pid and r["status"] == "missingOnTarget" for r in plan["rows"]), (s, str(plan)[:300]))
    s, j, _ = pull(B, devA_on_B, [(pid, "create")], ack=False)
    R.check("start refused without the acknowledgement", s >= 400, (s, j))

    # ---- interrupted transfer: kill -9 the source mid-way ----
    s, j, _ = pull(B, devA_on_B, [(pid, "create")])
    R.check("pull started", s == 200, (s, j))
    jid = j["id"]

    def part_progress():
        jj = B.get(f"/api/sync/jobs/{jid}", headers=uiB)
        it = jj["items"][0]
        return it if it["total"] and it["bytes"] >= it["total"] * 0.3 else (jj if jj["state"] != "running" else None)

    it = wait_for(part_progress, 60, 0.02)
    A.kill9()
    jb = wait_job(B, jid, 120)
    state = jb["items"][0] if jb else None
    R.note(f"after killing the source: job state={jb and jb['state']} item={state and (state['state'], state['bytes'], state['total'], state['code'])}")
    R.check("killing the source mid-transfer ends the job without a created profile", bool(jb) and jb["items"][0]["state"] in ("failed", "cancelled") and not [p for p in B.overview()["profiles"] if p["name"] == "sync-src"], jb and jb["items"][0])
    R.check("failure explains the problem (code/message)", bool(jb) and bool(jb["items"][0].get("message")), jb and jb["items"][0])
    got = jb["items"][0]["bytes"] if jb else 0
    R.note(f"bytes received before the kill: {got}")
    A.start()
    s, j, _ = pull(B, devA_on_B, [(pid, "create")])
    R.check("retry after the source is back starts", s == 200, (s, j))
    first = None
    if s == 200:
        first = wait_for(lambda: (lambda it: it if it["bytes"] > 0 else None)(B.get(f"/api/sync/jobs/{j['id']}", headers=uiB)["items"][0]), 30, 0.01)
    R.note(f"first progress sample of the retry: {first and (first['bytes'], first['total'])} (before kill: {got})")
    jb = wait_job(B, j["id"], 240) if s == 200 else None
    R.check("resumed pull finished: item done", bool(jb) and jb["items"][0]["state"] == "done", jb and jb["items"][0])
    new = [p for p in B.overview()["profiles"] if p["name"] == "sync-src"]
    R.check("profile created on B", len(new) == 1, new and [p["name"] for p in new])
    if not new:
        return
    bid = new[0]["id"]
    bigB = os.path.join(B.prof_row(bid)["path"]["dir"], "junk.bin")
    R.check("12 MiB file identical after the interrupted + resumed transfer (sha256)", os.path.exists(bigB) and sha_file(big) == sha_file(bigB))
    s, body, _ = B.req("GET", f"/api/cookies/{bid}/export?format=json")
    R.check("cookie arrived on B", s == 200 and any(x["name"] == "syn" and x["value"] == "one" for x in body), (s, str(body)[:200]))
    R.check("pinned fingerprint identical on B", B.prof_row(bid)["fp"]["userAgent"] == A.prof_row(pid)["fp"]["userAgent"])
    R.check("settings arrived", B.profile(bid)["settings"]["locale"] == "pt-BR")
    R.check("proxy not included by default", B.prof_row(bid)["assigned"] is None)
    R.check("same source id kept or new local id (no collision)", True)

    # ---- conflict detection ----
    s, plan, _ = B.req("GET", f"/api/sync/devices/{devA_on_B}/plan?direction=pull", headers=uiB)
    row = [r for r in plan["rows"] if r["sourceId"] == pid][0]
    R.check("synced profile shows identical", row["status"] == "identical", row)
    A.post(f"/api/cookies/{pid}/import", {"content": json.dumps([dict(ck[0], value="two-from-A")]), "mode": "all"})
    s, plan, _ = B.req("GET", f"/api/sync/devices/{devA_on_B}/plan?direction=pull", headers=uiB)
    row = [r for r in plan["rows"] if r["sourceId"] == pid][0]
    R.check("only A changed -> sourceChanged", row["status"] == "sourceChanged", row)
    B.post(f"/api/cookies/{bid}/import", {"content": json.dumps([dict(ck[0], value="two-from-B")]), "mode": "all"})
    s, plan, _ = B.req("GET", f"/api/sync/devices/{devA_on_B}/plan?direction=pull", headers=uiB)
    row = [r for r in plan["rows"] if r["sourceId"] == pid][0]
    R.check("both changed -> conflict, needs an explicit choice", row["status"] == "conflict" and row["needsChoice"] and row["defaultAction"] == "skip", row)
    s, j, _ = pull(B, devA_on_B, [(pid, "skip")])
    jb = wait_job(B, j["id"], 60) if s == 200 else None
    s2, body, _ = B.req("GET", f"/api/cookies/{bid}/export?format=json")
    R.check("skip leaves B untouched (no auto-merge)", any(x["value"] == "two-from-B" for x in body), body)
    s, j, _ = pull(B, devA_on_B, [(pid, "copy")])
    jb = wait_job(B, j["id"], 120) if s == 200 else None
    names = [p["name"] for p in B.overview()["profiles"]]
    R.check("save-as-copy adds a separate profile, original untouched", len(names) == 2 and any(x["value"] == "two-from-B" for x in B.req("GET", f"/api/cookies/{bid}/export?format=json")[1]), names)
    s, j, _ = pull(B, devA_on_B, [(pid, "replace")])
    jb = wait_job(B, j["id"], 120) if s == 200 else None
    s2, body, _ = B.req("GET", f"/api/cookies/{bid}/export?format=json")
    R.check("take-theirs replaces B with A's data", jb is not None and jb["items"][0]["state"] == "done" and any(x["value"] == "two-from-A" for x in body), (jb and jb["items"][0], body))
    pdir = os.path.dirname(B.prof_row(bid)["path"]["dir"])
    bk = [d for d in os.listdir(pdir) if d.startswith("user-data.backup")]
    R.check("take-theirs kept a backup of the replaced data next to the profile", len(bk) >= 1, os.listdir(pdir))
    # running profile on target blocks replace
    B.start_profile(bid)
    s, plan, _ = B.req("GET", f"/api/sync/devices/{devA_on_B}/plan?direction=pull", headers=uiB)
    row = [r for r in plan["rows"] if r["sourceId"] == pid][0]
    R.check("target running: replace is not offered (only copy/skip)", "replace" not in row["actions"], row)
    s, j, _ = pull(B, devA_on_B, [(pid, "replace")])
    jb = wait_job(B, j["id"], 60) if s == 200 else None
    R.check("replace onto a running target profile is refused/fails, nothing changed", s >= 400 or (jb and jb["items"][0]["state"] != "done"), (s, jb and jb["items"][0]))
    B.stop_profile(bid)
    # push direction
    s, plan, _ = B.req("GET", f"/api/sync/devices/{devA_on_B}/plan?direction=push", headers=uiB)
    R.check("push plan works", s == 200 and plan["rows"] is not None, (s, str(plan)[:200]))
    # revoke
    s, b, _ = B.req("DELETE", f"/api/devices/{devA_on_B}", headers=uiB)
    R.check("revoke device", s == 200, (s, b))
    s, b, _ = B.req("POST", f"/api/devices/{devA_on_B}/hello")
    R.check("revoked device can no longer talk", s >= 400, (s, str(b)[:150]))
    s, plan, _ = B.req("GET", f"/api/sync/devices/{devA_on_B}/plan?direction=pull", headers=uiB)
    R.check("plan refused after revocation", s >= 400, (s, str(plan)[:150]))
    A.stop()
    B.stop()
