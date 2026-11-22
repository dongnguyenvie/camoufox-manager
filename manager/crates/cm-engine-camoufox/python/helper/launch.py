"""`launch`: open Camoufox with the persistent profile, the pinned machine, the profile's setting
overrides and the app's proxy/timezone/geo, print the Playwright ws endpoint, run until told to stop.

--profile is a JSON file {"id": ..., "config": {...pinned Camoufox config...}} written by the
app before each launch. Exits when stdin closes or a line "stop" arrives (graceful close),
on SIGTERM/SIGINT/SIGHUP, or when the user closes the last browser window.
"""

import asyncio
import json
import os
import signal
import sys
import threading

from common import CANVAS_INIT_JS, ensure_search_engine, apply_screen, canvas_seed, check_timezone, decode_chunks, emit, encode_chunks, parse_size


async def cmd_launch(args):
    from camoufox.utils import launch_options
    from playwright.async_api import async_playwright

    with open(args.profile, encoding="utf-8") as f:
        profile = json.load(f)
    config = dict(profile["config"])
    # `cm:*` keys are the manager's own pins (not Camoufox config): read here, never sent to the browser.
    seed = canvas_seed(config, profile.get("id", ""))
    for k in [k for k in config if k.startswith("cm:")]:
        del config[k]

    # ---- overrides from the profile settings, applied on top of the pinned machine ----
    # Hardware stays pinned; only what the user explicitly set changes, so the identity holds.
    if args.screen:
        apply_screen(config, parse_size(args.screen, "screen"))
    if args.window:
        w, h = parse_size(args.window, "window")
        config["window.outerWidth"], config["window.outerHeight"] = w, h
    if args.fonts:
        extra = json.loads(args.fonts)
        config["fonts"] = list(dict.fromkeys([*(config.get("fonts") or []), *extra]))
    if args.timezone:
        check_timezone(args.timezone)
    kwargs = {}
    # Always pass a locale: Firefox keeps intl.* prefs in the persistent profile, so a locale set
    # once would stick after the override is cleared. Camoufox's own default is en-US.
    kwargs["locale"] = (args.locale or "en-US").replace("_", "-")
    if args.humanize:
        kwargs["humanize"] = True
    if args.webrtc == "none":
        kwargs["block_webrtc"] = True
    if args.canvas in ("native", "unique"):
        # Camoufox >= 0.5.9 covers canvas/WebGL readback with random data by default (a new value
        # on every read and launch; `canvas:seed` is gone upstream). canvas_noise=False keeps the
        # host's native rendering, identical on every launch (measured in the phase 4 report).
        # "native" stops here (identical on every profile of this machine); "unique" adds the
        # per-profile seeded readback below (CM-1.20).
        kwargs["canvas_noise"] = False
    if args.canvas == "unique":
        kwargs["main_world_eval"] = True  # lets the init script below run in the page's own world ("mw:")

    # Identical launch_options inputs every time -> the fingerprint (fonts, GPU, voices...) stays.
    # geoip=False so Camoufox does not look up the IP itself and override the app's geo/timezone.
    opts = launch_options(config=config, headless=args.headless, geoip=False, i_know_what_im_doing=True, **kwargs)
    ensure_search_engine(opts["executable_path"])

    # Geo/timezone/IP of the proxy are applied AFTER, without touching the fingerprint seed.
    env = opts["env"]
    cfg = decode_chunks(env, "CAMOU_CONFIG_")
    prefs = opts.setdefault("firefox_user_prefs", {})
    if args.extension_prefs:
        # The app sideloaded add-ons into <profile>/extensions. Without these Firefox installs them
        # disabled (autoDisableScopes) or ignores unsigned ones (Camoufox is not release-signed).
        prefs.update({"extensions.autoDisableScopes": 0, "extensions.enabledScopes": 15, "xpinstall.signatures.required": False})
    for k in [k for k in cfg if k.startswith("geolocation:") or k in ("timezone", "webrtc:ipv4")]:
        del cfg[k]
    if args.timezone:
        cfg["timezone"] = args.timezone
    # D4: never set geolocation without a timezone (the host timezone would leak next to it).
    if args.geo and args.timezone:
        lat, lon = (float(x) for x in args.geo.split(","))
        cfg["geolocation:latitude"] = lat
        cfg["geolocation:longitude"] = lon
        cfg["geolocation:accuracy"] = 100.0
        prefs["permissions.default.geo"] = 1
    else:
        prefs["permissions.default.geo"] = 0
    mode = args.webrtc or "replace"
    if args.ip and mode == "replace":
        # WebRTC reports the proxy IP instead of the real one.
        cfg["webrtc:ipv4"] = args.ip
    if args.proxy:
        opts["proxy"] = {"server": args.proxy}
    else:
        opts.pop("proxy", None)
    if args.proxy and not args.direct_proxy:
        # replace / none: no separate WebRTC path (direct UDP) when a proxy is set.
        # real: leave UDP direct so the real IP shows. forward: Firefox defaults untouched.
        if mode in ("replace", "none"):
            prefs["media.peerconnection.ice.proxy_only_if_behind_proxy"] = True
        elif mode == "real":
            prefs["media.peerconnection.ice.proxy_only_if_behind_proxy"] = False
    else:
        # No proxy, or only the app's direct relay (local-network blocking): WebRTC behaves as
        # without a proxy and must not be tied to the relay.
        if mode != "forward":
            prefs["media.peerconnection.ice.proxy_only_if_behind_proxy"] = False
    if args.prefs:
        # Data-saving toggles and the profile's custom prefs (validated by the app): last word.
        extra = json.loads(args.prefs)
        prefs.update({k: v for k, v in extra.items() if isinstance(v, (bool, int, float, str))})
    encode_chunks(env, "CAMOU_CONFIG_", cfg)
    encode_chunks(env, "CAMOU_PREFS_", prefs)

    os.makedirs(args.user_data, exist_ok=True)
    stop = asyncio.Event()
    loop = asyncio.get_running_loop()
    if sys.platform != "win32":
        for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
            loop.add_signal_handler(sig, stop.set)

    async def watch_stdin_thread():
        # Windows asyncio cannot connect_read_pipe(stdin): read lines in a thread instead.
        def read_lines():
            try:
                for raw in sys.stdin.buffer:
                    if raw.strip() == b"stop":
                        break
            except Exception:
                pass
            loop.call_soon_threadsafe(stop.set)

        threading.Thread(target=read_lines, daemon=True).start()

    async def watch_stdin():
        if sys.platform == "win32":
            return await watch_stdin_thread()
        reader = asyncio.StreamReader()
        await loop.connect_read_pipe(lambda: asyncio.StreamReaderProtocol(reader), sys.stdin)
        while True:
            line = await reader.readline()
            if not line or line.strip() == b"stop":
                stop.set()
                return

    async with async_playwright() as p:
        ctx = await p.firefox.launch_persistent_context(args.user_data, **opts)
        if args.canvas == "unique":
            # Per-profile, launch-stable canvas/WebGL/audio readback (see common.CANVAS_INIT_JS).
            await ctx.add_init_script("mw:" + CANVAS_INIT_JS.replace("__SEED__", str(seed)))
        ctx.on("close", lambda _: stop.set())  # user closed the last window -> Firefox exits
        page = ctx.pages[0] if ctx.pages else await ctx.new_page()
        if args.start_url:
            try:
                await page.goto(args.start_url, wait_until="commit", timeout=20000)
            except Exception:  # an unreachable start page must not stop the browser
                pass
        bound = await ctx.browser.bind(f"profile-{profile.get('id', '')}", host="127.0.0.1", port=0)
        emit({"ws": bound["endpoint"], "pid": os.getpid()})
        stdin_task = asyncio.create_task(watch_stdin())
        await stop.wait()
        stdin_task.cancel()
        try:
            await asyncio.wait_for(ctx.close(), timeout=15)  # graceful: Firefox flushes cookies/session
        except Exception:
            pass
