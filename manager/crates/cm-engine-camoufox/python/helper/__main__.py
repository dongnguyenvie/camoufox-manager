"""Python helper for the Camoufox engine of Camoufox Manager.

These files are embedded in the binary (include_str!) and written to
<data_dir>/camoufox/helper/ on every start, so edit them here and rebuild. Run the folder with
the camoufox venv's python:  python <data_dir>/camoufox/helper <command> ...

  check
      Prints {"ok": true, "browser": "..."} when the camoufox package and browser are installed.
  generate --os win|mac|lin                                   (generate.py)
      Generates the pinned machine ONCE with Camoufox's generator and prints the final
      Camoufox config as JSON. The app stores it in the database.
  launch --profile <launch.json> --user-data <dir> [--proxy URL]
         [--timezone TZ] [--geo=LAT,LON] [--ip IP] [--headless] [--locale L]
         [--webrtc replace|real|forward|none] [--screen WxH] [--window WxH] [--fonts JSON]
         [--humanize] [--start-url URL] [--canvas unique|native|random] [--extension X.xpi]...   (launch.py)
      Opens Camoufox with the persistent profile, prints {"ws": "ws://127.0.0.1:PORT/TOKEN"}
      (Playwright protocol: playwright.firefox.connect(ws)) and runs until stdin closes / "stop".
  cookies export|import --user-data DIR --file FILE [--mode domains|all]   (cookies.py)
      Cookie store of a CLOSED profile as JSON; prints {"ok": true, "count": N}.
  extensions install --user-data DIR [XPI...]                 (extensions.py)
      Sync <DIR>/extensions/ with the given .xpi files; prints {"ok": true, "installed": [...], "removed": [...]}.
  warm --ws WS --urls JSON --dwell SECONDS                    (warm.py, phase 10)
      Visits each URL on a RUNNING profile (connects over its ws endpoint), dwelling with light scrolling.

Every command prints JSON lines on stdout; an error is {"error": "..."} and exit code 1.
"""

import argparse
import asyncio
import sys

from common import OS_NAMES, emit


def cmd_check(_args):
    from camoufox.pkgman import camoufox_path

    try:
        path = camoufox_path(download_if_missing=False)
    except Exception as e:  # `camoufox fetch` not run yet
        emit({"ok": False, "code": "browser_missing", "error": f"Camoufox browser not downloaded: {e}"})
        return
    emit({"ok": True, "browser": str(path)})


def main():
    ap = argparse.ArgumentParser(prog="helper")
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("check")
    g = sub.add_parser("generate")
    g.add_argument("--os", choices=list(OS_NAMES), required=True)
    l = sub.add_parser("launch")
    l.add_argument("--profile", required=True)
    l.add_argument("--user-data", required=True)
    l.add_argument("--proxy")
    l.add_argument("--timezone")
    # Pass as --geo=LAT,LON: a negative latitude would otherwise look like an option to argparse.
    l.add_argument("--geo")
    l.add_argument("--ip")
    l.add_argument("--headless", action="store_true")
    l.add_argument("--locale")
    l.add_argument("--webrtc", choices=["replace", "real", "forward", "none"])
    l.add_argument("--screen")
    l.add_argument("--window")
    l.add_argument("--fonts", help="JSON array of extra fonts")
    l.add_argument("--humanize", action="store_true")
    l.add_argument("--start-url")
    l.add_argument("--canvas", choices=["unique", "native", "random"])
    l.add_argument("--extension", action="append", help="path of an .xpi to install (repeatable)")
    c = sub.add_parser("cookies")
    c.add_argument("action", choices=["export", "import"])
    c.add_argument("--user-data", required=True)
    c.add_argument("--file", required=True)
    c.add_argument("--mode", choices=["domains", "all"], default="domains")
    e = sub.add_parser("extensions")
    e.add_argument("action", choices=["install"])
    e.add_argument("--user-data", required=True)
    e.add_argument("xpi", nargs="*")
    w = sub.add_parser("warm")
    w.add_argument("--ws", required=True)
    w.add_argument("--urls", required=True, help="JSON array of URLs")
    w.add_argument("--dwell", type=float, required=True, help="seconds per URL")
    args = ap.parse_args()
    try:
        if args.cmd == "check":
            cmd_check(args)
        elif args.cmd == "generate":
            from generate import cmd_generate

            cmd_generate(args)
        elif args.cmd == "launch":
            from launch import cmd_launch

            asyncio.run(cmd_launch(args))
        elif args.cmd == "cookies":
            from cookies import cmd_cookies

            cmd_cookies(args)
        elif args.cmd == "warm":
            from warm import cmd_warm

            asyncio.run(cmd_warm(args))
        else:
            from extensions import cmd_extensions

            cmd_extensions(args)
    except Exception as e:
        emit({"error": f"{type(e).__name__}: {e}"})
        sys.exit(1)


if __name__ == "__main__":
    main()
