"""`warm`: browse like a person on an already running profile (scheduled warming launches).

  warm --ws <playwright ws endpoint> --urls <JSON array> --dwell <seconds per URL>

Connects to the browser `launch` started (Playwright protocol), opens a tab, visits each URL,
stays `--dwell` seconds on it scrolling lightly (down, now and then back up, with pauses),
closes its tab and disconnects. The browser itself is left running: the app closes it.
Prints {"ok": true, "visited": N, "failed": [...]}; errors only when no URL could be visited.
"""

import asyncio
import json
import random
import time

from common import emit


async def _dwell(page, seconds):
    end = time.monotonic() + seconds
    while True:
        left = end - time.monotonic()
        if left <= 0:
            return
        try:
            up = random.random() < 0.2
            await page.mouse.wheel(0, -random.randint(150, 400) if up else random.randint(200, 650))
        except Exception:
            pass  # a page that is still loading or has no scrollable body is fine
        await asyncio.sleep(min(left, random.uniform(1.5, 4.0)))


async def cmd_warm(args):
    from playwright.async_api import async_playwright

    urls = json.loads(args.urls)
    visited, failed = 0, []
    async with async_playwright() as p:
        browser = await p.firefox.connect(args.ws, timeout=30000)
        ctx = browser.contexts[0] if browser.contexts else await browser.new_context()
        page = await ctx.new_page()
        for url in urls:
            started = time.monotonic()
            try:
                await page.goto(url, wait_until="domcontentloaded", timeout=30000)
                visited += 1
            except Exception as e:
                failed.append({"url": url, "error": f"{type(e).__name__}: {str(e)[:160]}"})
            # The dwell time counts from the start of the visit, even if the page was slow.
            await _dwell(page, max(0.0, args.dwell - (time.monotonic() - started)))
        try:
            await page.close()
        except Exception:
            pass
    if not visited:
        raise RuntimeError(f"no URL could be opened: {failed[0]['error'] if failed else 'none given'}")
    emit({"ok": True, "visited": visited, "failed": failed})
