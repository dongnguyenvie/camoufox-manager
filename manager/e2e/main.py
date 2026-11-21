"""e2e entry point (run through run.sh). `main.py [scenario numbers...]`, default all."""

import faulthandler
import importlib
import json
import os
import sys
import traceback

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lib  # noqa: E402
from lib import App, Fakes, Results, make_home, rm_home  # noqa: E402

# number -> (module, function, description)
SCENARIOS = {
    1: ("scen_core", "s1_lifecycle", "1 create/edit/start/verify/stop"),
    2: ("scen_core", "s2_cookies", "2 cookies persist + import/export"),
    3: ("scen_proxy", "s3_proxy", "3 proxy library + traffic + geo + fail closed"),
    4: ("scen_proxy", "s4_leaks", "4 leak checks (WebRTC, timezone, DNS)"),
    5: ("scen_fp", "s5_fingerprint", "5 canvas/WebGL/audio unique + stable"),
    6: ("scen_transfer", "s6_transfer", "6 export/import/clone/bulk/template/extension"),
    7: ("scen_org", "s7_org", "7 org + sheets"),
    8: ("scen_life", "s8_lifecycle", "8 crash recovery, lease, two instances"),
    9: ("scen_life", "s9_schedule", "9 scheduling"),
    10: ("scen_cli", "s10_cli", "10 settings/CLI/API keys/doctor"),
    11: ("scen_sync", "s11_sync", "11 device sync"),
    12: ("scen_ui", "s12_ui", "12 UI smoke"),
}


class Ctx:
    pass


def main():
    faulthandler.dump_traceback_later(int(os.environ.get("E2E_HANG_SECS", "240")), repeat=True)  # shows where a scenario hangs
    want = [int(x) for x in sys.argv[1:]] or sorted(SCENARIOS)
    c = Ctx()
    c.R = Results()
    c.fakes = Fakes()
    c.homes = []
    c.out = os.environ.get("E2E_OUT") or os.path.join(os.environ.get("TMPDIR", "/tmp"), "cm-e2e-out")
    os.makedirs(c.out, exist_ok=True)
    c.new_home = lambda: (c.homes.append(make_home()) or c.homes[-1])
    c.page_port, c.page_log = c.fakes.web(lib.SAO_PAULO)
    from playwright.sync_api import sync_playwright

    c.pw_cm = sync_playwright()
    c.pw = c.pw_cm.start()
    apps = []
    c.apps = apps
    home = c.new_home()
    c.main = App(home, 7921, 47921, 51000, name="main")
    apps.append(c.main)
    c.main.start()
    print(f"main app home: {home}")
    try:
        for n in want:
            mod, fn, desc = SCENARIOS[n]
            m = importlib.import_module(mod)
            if not c.main.alive():
                c.R.note("main app was not running; restarting")
                c.main.start()
            c.R.run(desc, getattr(m, fn), c)
    finally:
        for a in apps:
            try:
                a.stop()
            except Exception:
                traceback.print_exc()
        try:
            c.pw_cm.__exit__(None, None, None)
        except Exception:
            pass
        c.fakes.close()
        bad = c.R.summary()
        with open(os.path.join(c.out, "results.json"), "w") as f:
            json.dump(c.R.rows, f, indent=1)
        if not os.environ.get("E2E_KEEP"):
            for h in c.homes:
                rm_home(h)
        else:
            print("kept homes:", c.homes)
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
