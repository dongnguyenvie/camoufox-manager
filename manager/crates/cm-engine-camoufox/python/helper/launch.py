"""`launch`: open Camoufox with the persistent profile, the pinned machine and the app's
proxy/timezone/geo, print the Playwright ws endpoint, run until told to stop.

--profile is a JSON file {"id": ..., "config": {...pinned Camoufox config...}} written by the
app before each launch. Exits when stdin closes or a line "stop" arrives (graceful close),
on SIGTERM/SIGINT/SIGHUP, or when the user closes the last browser window.
"""

import asyncio
import json
import os
import signal
import sys

from common import decode_chunks, emit, encode_chunks


async def cmd_launch(args):
    from camoufox.utils import launch_options
    from playwright.async_api import async_playwright

    with open(args.profile, encoding="utf-8") as f:
        profile = json.load(f)
    config = dict(profile["config"])

    # Identical launch_options inputs every time -> the fingerprint (fonts, GPU, voices...) stays.
    # geoip=False so Camoufox does not look up the IP itself and override the app's geo/timezone.
    opts = launch_options(config=config, headless=args.headless, geoip=False, i_know_what_im_doing=True)

    # Geo/timezone/IP of the proxy are applied AFTER, without touching the fingerprint seed.
    env = opts["env"]
    cfg = decode_chunks(env, "CAMOU_CONFIG_")
    prefs = opts.setdefault("firefox_user_prefs", {})
    for k in [k for k in cfg if k.startswith("geolocation:") or k in ("timezone", "webrtc:ipv4")]:
        del cfg[k]
    if args.timezone:
        cfg["timezone"] = args.timezone
    if args.geo:
        lat, lon = (float(x) for x in args.geo.split(","))
        cfg["geolocation:latitude"] = lat
        cfg["geolocation:longitude"] = lon
        cfg["geolocation:accuracy"] = 100.0
        prefs["permissions.default.geo"] = 1
    else:
        prefs["permissions.default.geo"] = 0
    if args.ip:
        # WebRTC reports the proxy IP instead of the real one.
        cfg["webrtc:ipv4"] = args.ip
    if args.proxy:
        opts["proxy"] = {"server": args.proxy}
        # No separate WebRTC path (direct UDP) when a proxy is set.
        prefs["media.peerconnection.ice.proxy_only_if_behind_proxy"] = True
    else:
        opts.pop("proxy", None)
        prefs["media.peerconnection.ice.proxy_only_if_behind_proxy"] = False
    encode_chunks(env, "CAMOU_CONFIG_", cfg)
    encode_chunks(env, "CAMOU_PREFS_", prefs)

    os.makedirs(args.user_data, exist_ok=True)
    stop = asyncio.Event()
    loop = asyncio.get_running_loop()
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
        loop.add_signal_handler(sig, stop.set)

    async def watch_stdin():
        reader = asyncio.StreamReader()
        await loop.connect_read_pipe(lambda: asyncio.StreamReaderProtocol(reader), sys.stdin)
        while True:
            line = await reader.readline()
            if not line or line.strip() == b"stop":
                stop.set()
                return

    async with async_playwright() as p:
        ctx = await p.firefox.launch_persistent_context(args.user_data, **opts)
        ctx.on("close", lambda _: stop.set())  # user closed the last window -> Firefox exits
        if not ctx.pages:
            await ctx.new_page()
        bound = await ctx.browser.bind(f"profile-{profile.get('id', '')}", host="127.0.0.1", port=0)
        emit({"ws": bound["endpoint"], "pid": os.getpid()})
        stdin_task = asyncio.create_task(watch_stdin())
        await stop.wait()
        stdin_task.cancel()
        try:
            await asyncio.wait_for(ctx.close(), timeout=15)  # graceful: Firefox flushes cookies/session
        except Exception:
            pass
