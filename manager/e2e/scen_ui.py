"""Scenario 12: UI smoke with Camoufox (headless) driven by Playwright: every page loads without
console errors in en and vi, and the dialogs that were never clicked before open and submit."""

import json
import os
import re

from lib import App, dead_port

PAGES = ["profiles", "proxies", "templates", "schedules", "devices", "settings"]
RAW_KEY = re.compile(r"\b(nav|profiles|proxies|org|tpl|schedules|devices|settings|transfer|sheets|common|doctor|bell|profileform|cookies|sync|engine|app)\.[a-z][A-Za-z0-9]*(\.[A-Za-z0-9-]+)*\b")


class Watch:
    def __init__(self, page):
        self.errors = []
        page.on("console", lambda m: self.errors.append(f"console.{m.type}: {m.text[:200]}") if m.type in ("error",) else None)
        page.on("pageerror", lambda e: self.errors.append(f"pageerror: {str(e)[:200]}"))
        page.on("requestfailed", lambda r: self.errors.append(f"requestfailed: {r.url[:100]}"))
        page.on("response", lambda r: self.errors.append(f"http {r.status}: {r.url[:100]}") if r.status >= 400 and "/api/" not in r.url else None)

    def take(self):
        e, self.errors = self.errors, []
        return e


def s12_ui(c):
    R = c.R
    c.main.stop()
    base_home = c.new_home()
    env = {"CM_SYNC_ADVERTISE": "127.0.0.1:47921"}
    U = App(base_home, 7921, 47921, 51000, env=env, name="ui")
    P = App(c.new_home(), 7922, 47922, 52000, env={"CM_SYNC_ADVERTISE": "127.0.0.1:47922"}, name="peer")
    c.apps += [U, P]
    U.start()
    P.start()
    from camoufox.utils import launch_options

    shots = os.path.join(c.out, "ui-shots")
    os.makedirs(shots, exist_ok=True)
    # fixtures
    ids = [U.create(f"ui-{i}", "lin") for i in range(4)]
    hp = c.fakes.http_proxy()
    U.post("/api/proxies/import", {"text": f"http://pu:pp@127.0.0.1:{hp.port}\n"})
    tag = U.post("/api/tags", {"name": "uitag"})

    browser = c.pw.firefox.launch(**launch_options(headless=True, geoip=False, i_know_what_im_doing=True))
    try:
        ctx = browser.new_context(accept_downloads=True, viewport={"width": 1500, "height": 900})
        page = ctx.new_page()
        w = Watch(page)

        def step(name, fn):
            try:
                ok = fn()
                R.check(name, ok is not False, "returned False")
            except Exception as e:
                R.check(name, False, f"{type(e).__name__}: {str(e)[:260]}")
                try:
                    page.screenshot(path=os.path.join(shots, re.sub(r"\W+", "_", name)[:60] + ".png"))
                except Exception:
                    pass
                # close any stuck dialog
                try:
                    page.keyboard.press("Escape")
                except Exception:
                    pass
            return None

        def go(p):
            page.goto(f"{U.base}/#/{p}")
            page.wait_for_load_state("networkidle")
            page.wait_for_timeout(700)

        def btn(text, exact=False):
            return page.get_by_role("button", name=text, exact=exact).first

        def dlg():
            return page.locator("[role=dialog]")

        def toast_text():
            page.wait_for_timeout(500)
            return page.inner_text("body")

        # ---------- every page, both languages, no console errors, no raw keys ----------
        for lang in ("en", "vi"):
            U.put("/api/settings", {"language": lang}, headers={"Origin": U.base})
            page.goto(f"{U.base}/")
            page.evaluate("try{localStorage.clear()}catch(e){}")
            for p in PAGES:
                def load(p=p, lang=lang):
                    go(p)
                    errs = w.take()
                    body = page.inner_text("body")
                    raw = [m.group(0) for m in RAW_KEY.finditer(body)]
                    title = page.title()
                    if errs:
                        raise AssertionError(f"errors: {errs[:3]}")
                    if raw:
                        raise AssertionError(f"untranslated keys visible: {raw[:5]}")
                    if len(body.strip()) < 40:
                        raise AssertionError("page is empty")
                    return title != ""

                step(f"[{lang}] page #{p} loads without console errors or raw i18n keys", load)
        U.put("/api/settings", {"language": "en"}, headers={"Origin": U.base})
        page.goto(f"{U.base}/")
        page.evaluate("try{localStorage.clear()}catch(e){}")

        # language switch from the sidebar persists
        def switch():
            go("profiles")
            page.locator("aside select").select_option("vi")
            page.wait_for_timeout(600)
            ok = U.get("/api/settings")["language"] == "vi"
            page.locator("aside select").select_option("en")
            page.wait_for_timeout(600)
            return ok and U.get("/api/settings")["language"] == "en"

        step("sidebar language switch is persisted to settings", switch)

        # ---------- profiles page: bulk delete confirm ----------
        def bulk_delete():
            go("profiles")
            boxes = page.locator("tbody input[type=checkbox]")
            boxes.nth(2).click()
            boxes.nth(3).click()
            page.locator("button:has-text('Delete')").first.click()
            page.wait_for_selector("[role=dialog]")
            txt = dlg().inner_text()
            assert "Delete 2 profiles" in txt, txt
            dlg().locator("button:has-text('Delete')").last.click()
            page.wait_for_timeout(1500)
            left = {p["id"] for p in U.overview()["profiles"]}
            return ids[2] not in left and ids[3] not in left and ids[0] in left

        step("bulk delete: confirm dialog opens, submit deletes the selected profiles", bulk_delete)

        # ---------- transfer: export / clone / import / bulk ----------
        exported = {}

        def export_dlg():
            go("profiles")
            page.locator("button[title='Export profile']").first.click()
            page.wait_for_selector("[role=dialog]")
            page.screenshot(path=os.path.join(shots, "export-dialog.png"))
            box = dlg().bounding_box()
            R.check("[B5] dialogs are centred over the viewport (not stuck inside the page header)", box and box["y"] > 120, box)
            dlg().get_by_role("button", name="Export", exact=True).click(timeout=8000)
            page.wait_for_selector("text=Exported", timeout=60000)
            f = [x for x in os.listdir(os.path.join(U.data_dir, "exports")) if x.endswith(".zip")]
            exported["zip"] = os.path.join(U.data_dir, "exports", f[0])
            return True

        step("transfer: export dialog opens and writes the zip", export_dlg)

        def clone_dlg():
            page.keyboard.press("Escape")
            go("profiles")
            n = len(U.overview()["profiles"])
            page.locator("button[title='Clone profile']").first.click()
            page.wait_for_selector("[role=dialog]")
            dlg().get_by_role("button", name=re.compile("^Clone")).last.click()
            page.wait_for_timeout(5000)
            return len(U.overview()["profiles"]) == n + 1

        step("transfer: clone dialog opens and creates the copy", clone_dlg)

        def import_dlg():
            go("profiles")
            n = len(U.overview()["profiles"])
            btn("Import", exact=False).click()
            page.wait_for_selector("[role=dialog]")
            dlg().locator("input[type=file]").set_input_files(exported["zip"])
            dlg().get_by_role("button", name="Import", exact=True).click()
            page.wait_for_selector("text=Imported", timeout=30000)
            return len(U.overview()["profiles"]) == n + 1

        step("transfer: import dialog accepts the zip", import_dlg)

        def bulk_dlg():
            page.keyboard.press("Escape")
            go("profiles")
            n = len(U.overview()["profiles"])
            btn("Bulk create").click()
            page.wait_for_selector("[role=dialog]")
            inputs = dlg().locator("input[type=text], input:not([type])")
            inputs.first.fill("UiBulk {n}")
            dlg().locator("input[type=number]").first.fill("2")
            dlg().get_by_role("button", name="Create", exact=True).click()
            page.wait_for_selector("text=Created 2 profiles", timeout=60000)
            return len(U.overview()["profiles"]) == n + 2

        step("transfer: bulk create dialog creates 2 profiles", bulk_dlg)

        # ---------- sheets ----------
        def sheets():
            page.keyboard.press("Escape")
            go("profiles")
            page.get_by_role("button", name=re.compile(r"Export$")).first.click()
            page.wait_for_selector("[role=dialog]")
            with page.expect_download() as d:
                dlg().get_by_text('.csv').first.click()
            path = os.path.join(c.out, "ui-sheet.csv")
            d.value.save_as(path)
            txt = open(path).read()
            assert txt.startswith("id,name") or "id" in txt.splitlines()[0], txt[:100]
            page.keyboard.press("Escape")
            # edit a name and a new row, import with the diff
            lines = txt.splitlines()
            lines[1] = lines[1].replace(",ui-0,", ",ui-0-sheet,", 1)
            open(path, "w").write("\n".join(lines) + "\n")
            btn("Import sheet").click()
            page.wait_for_selector("[role=dialog]")
            dlg().locator("input[type=file]").set_input_files(path)
            dlg().get_by_role("button", name="Check file").click()
            page.wait_for_selector("text=to update", timeout=15000)
            summary = dlg().inner_text()
            assert "1 to update" in summary, summary[:300]
            ap = dlg().get_by_role("button", name="Apply", exact=True)
            try:
                ap.click(timeout=4000)
                clickable = True
            except Exception:
                clickable = False
                ap.dispatch_event("click")  # keep going so the rest of the flow is still verified
            R.check("[B5] sheets import: the Apply button can be clicked with the mouse", clickable, "covered by the table (dialog is clipped to the page header)")
            page.wait_for_selector("text=updated", timeout=20000)
            return any(p["name"] == "ui-0-sheet" for p in U.overview()["profiles"])

        step("sheets: export download + import dry-run diff + apply", sheets)

        # ---------- org: tag bar / manage dialog ----------
        def org_manage():
            page.keyboard.press("Escape")
            go("profiles")
            page.get_by_role("button", name=re.compile("Tags & groups")).first.click()
            page.wait_for_selector("[role=dialog]")
            ok = "uitag" in dlg().inner_text()
            page.keyboard.press("Escape")
            return ok

        step("org: manage tags/groups dialog opens and lists tags", org_manage)

        # ---------- proxies ----------
        def proxies_add():
            go("proxies")
            btn("Add proxy").click()
            page.wait_for_selector("[role=dialog]")
            ins = dlg().locator("input.input")  # name, server, port, username, password
            ins.nth(1).fill("127.0.0.1")
            ins.nth(2).fill(str(dead_port()))
            dlg().get_by_role("button", name=re.compile("Save|Add", re.I)).last.click()
            page.wait_for_selector("text=Proxy saved", timeout=10000)
            return len(U.get("/api/proxies")["proxies"]) == 2

        step("proxies: add dialog saves a proxy", proxies_add)

        def proxies_import():
            go("proxies")
            btn("Import list").click()
            page.wait_for_selector("[role=dialog]")
            dlg().locator("textarea").fill(f"socks5://x:y@127.0.0.1:{dead_port()}\n127.0.0.1:{dead_port()}:u:p")
            dlg().get_by_role("button", name="Import", exact=True).click()
            page.wait_for_selector("text=Imported 2 proxies", timeout=10000)
            return len(U.get("/api/proxies")["proxies"]) == 4

        step("proxies: import list dialog imports 2 lines", proxies_import)

        def proxies_import_bad():
            go("proxies")
            btn("Import list").click()
            page.wait_for_selector("[role=dialog]")
            dlg().locator("textarea").fill("garbage line here")
            dlg().get_by_role("button", name="Import", exact=True).click()
            page.wait_for_selector("text=Nothing was imported", timeout=10000)
            page.keyboard.press("Escape")
            return True

        step("proxies: invalid list shows the per-line rejection", proxies_import_bad)

        def proxies_check_assign():
            go("proxies")
            page.get_by_role("button", name="Check").first.click()
            page.wait_for_selector("text=/Proxy OK|Proxy failed|Brazil|Sao Paulo/", timeout=20000)
            page.get_by_role("button", name="Assign", exact=True).first.click()
            page.wait_for_selector("[role=dialog]")
            dlg().locator("input[type=checkbox]").first.check()
            dlg().get_by_role("button", name=re.compile("^Assign to")).click()
            page.wait_for_selector("text=Assigned to", timeout=10000)
            return any(p["assigned"] for p in U.overview()["profiles"])

        step("proxies: check + assign dialog assigns a proxy to a profile", proxies_check_assign)

        def proxies_seq():
            go("proxies")
            btn("Assign proxy list").click()
            page.wait_for_selector("[role=dialog]")
            page.wait_for_timeout(300)
            profs = dlg().locator("button, label").filter(has_text=re.compile("ui-|Shop|UiBulk|copy", re.I))
            profs.first.click()
            dlg().get_by_role("button", name="Preview mapping").click()
            page.wait_for_timeout(800)
            txt = dlg().inner_text()
            dlg().get_by_role("button", name="Apply", exact=True).click()
            page.wait_for_selector("text=Assigned 1 profiles", timeout=10000)
            return True

        step("proxies: sequential assignment dialog previews and applies", proxies_seq)

        # ---------- templates + extensions ----------
        def templates():
            go("templates")
            btn("New template").click()
            page.wait_for_selector("[role=dialog]")
            dlg().locator("input").first.fill("ui-template")
            dlg().get_by_role("button", name=re.compile("Save|Create", re.I)).last.click()
            page.wait_for_selector("text=Template saved", timeout=10000)
            page.get_by_role("button", name="Create profiles").first.click()
            page.wait_for_selector("[role=dialog]")
            dlg().get_by_role("button", name=re.compile(r"^Create \d")).click()
            page.wait_for_selector("text=/Created \\d+ profiles/", timeout=90000)
            return any(p["name"] == "ui-template" for p in U.overview()["profiles"])

        step("templates: create a template, then profiles from it", templates)

        def extensions():
            from scen_transfer import make_xpi

            go("templates")
            page.get_by_role("button", name=re.compile(r"^Extension")).first.click()
            page.wait_for_timeout(500)
            path = os.path.join(c.out, "ui-ext.xpi")
            open(path, "wb").write(make_xpi())
            page.locator("input[type=file]").first.set_input_files(path)
            page.wait_for_selector("text=Added", timeout=10000)
            page.get_by_role("button", name="Assign", exact=True).first.click()
            page.wait_for_selector("[role=dialog]")
            dlg().locator("input[type=checkbox]").first.check()
            dlg().get_by_role("button", name="Add to selected").click()
            page.wait_for_selector("text=/Updated \\d+ profiles/", timeout=10000)
            return len(U.get("/api/extensions")) == 1

        step("extensions: upload .xpi and assign it to a profile", extensions)

        # ---------- schedules ----------
        def schedules():
            go("schedules")
            btn("New schedule").click()
            page.wait_for_selector("[role=dialog]")
            dlg().locator("input.input").first.fill("ui-warm")
            # pick the first profile in the target select
            for s in dlg().locator("select").all():
                opts = s.locator("option").all_inner_texts()
                if any(o.startswith("ui-") for o in opts):
                    s.select_option(index=[i for i, o in enumerate(opts) if o.startswith("ui-")][0])
                    break
            dlg().locator("textarea").first.fill(f"http://127.0.0.1:{c.page_port}/ui-warm")
            dlg().get_by_role("button", name=re.compile("Save|Create", re.I)).last.click()
            page.wait_for_selector("text=Schedule saved", timeout=10000)
            page.get_by_role("button", name="Run now").first.click()
            page.wait_for_selector("text=Started", timeout=10000)
            page.wait_for_timeout(1500)
            runs = U.get("/api/schedules/runs")
            return len(runs) >= 1

        step("schedules: new schedule dialog saves, Run now starts a run, history lists it", schedules)

        # ---------- devices ----------
        def devices():
            go("devices")
            offer = P.get("/api/devices/pairing", headers={"Origin": P.base})
            link = page.get_by_placeholder("cmsync://…/pair?c=…&fp=…")
            link.fill(offer["url"])
            page.get_by_role("button", name="Pair", exact=True).click()
            page.wait_for_selector("text=/Paired with/", timeout=15000)
            page.get_by_role("button", name="Ping").first.click()
            page.wait_for_selector("text=/answered/", timeout=15000)
            page.get_by_role("button", name="Revoke").first.click()
            page.wait_for_selector("[role=dialog]")
            dlg().get_by_role("button", name="Revoke").last.click()
            page.wait_for_timeout(1000)
            return any(d.get("revokedAt") for d in U.get("/api/devices")["devices"])

        step("devices: pair by pasted link, ping, revoke dialog", devices)

        def settings_doctor():
            go("settings")
            page.get_by_role("button", name=re.compile("Run|Doctor|Check", re.I)).first.click()
            page.wait_for_timeout(3000)
            return "ok" in page.inner_text("body").lower()

        step("settings: doctor panel runs", settings_doctor)

        errs = w.take()
        R.check("no console errors during the whole dialog session", not errs, errs[:5])
        ctx.close()
    finally:
        browser.close()
    P.stop()
    U.stop()
