"""Shared pieces of the e2e harness: app process control, API client, fake upstream proxies,
result recording. Python stdlib only (+ playwright from the real Camoufox venv, imported lazily).
"""

import asyncio
import base64
import json
import os
import shutil
import signal
import subprocess
import tempfile
import threading
import time
import traceback
import urllib.error
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
MANAGER = os.path.dirname(HERE)
BIN = os.path.join(MANAGER, "target", "debug", "camoufox-manager")
REAL_HOME = os.path.expanduser("~")


def real_venv_python():
    for n in ("camoufox-manager",):
        p = os.path.join(REAL_HOME, "Library/Application Support", n, "camoufox/venv/bin/python")
        if os.path.exists(p):
            return p
    raise SystemExit("no Camoufox venv found")


def make_home(prefix="cme2e."):
    """Private HOME with the (huge) browser cache symlinked from the real one. Caller owns it."""
    home = tempfile.mkdtemp(prefix=prefix)
    os.makedirs(os.path.join(home, "Library/Caches"))
    os.symlink(os.path.join(REAL_HOME, "Library/Caches/camoufox"), os.path.join(home, "Library/Caches/camoufox"))
    return home


def rm_home(home):
    """Delete only a HOME we created (guard: name + not the real home). Unlinks the cache symlink first."""
    if home and os.path.basename(home).startswith("cme2e.") and os.path.realpath(home) != os.path.realpath(REAL_HOME):
        link = os.path.join(home, "Library/Caches/camoufox")
        if os.path.islink(link):
            os.unlink(link)
        shutil.rmtree(home, ignore_errors=True)


class ApiError(Exception):
    def __init__(self, status, body):
        super().__init__(f"HTTP {status}: {body}")
        self.status, self.body = status, body


class App:
    """One camoufox-manager `serve` process with its own HOME and ports. Stops by PID only."""

    def __init__(self, home, port=7921, sync_port=47921, proxy_base=51000, env=None, name="app"):
        self.home, self.port, self.sync_port, self.proxy_base = home, port, sync_port, proxy_base
        self.extra_env, self.name = env or {}, name
        self.proc = None
        self.api_key = None
        self.log_path = os.path.join(home, f"{name}.log")
        self.data_dir = os.path.join(home, "Library/Application Support/camoufox-manager")

    @property
    def base(self):
        return f"http://127.0.0.1:{self.port}"

    def env(self):
        e = dict(os.environ)
        e.update(
            HOME=self.home,
            CAMOUFOX_PYTHON=real_venv_python(),
            CAMOUFOX_HEADLESS="1",
            CM_PORT=str(self.port),
            CM_SYNC_PORT=str(self.sync_port),
            CM_PROXY_BASE_PORT=str(self.proxy_base),
        )
        e.pop("CM_API_KEY", None)
        e.update(self.extra_env)
        return e

    def start(self, wait=True):
        log = open(self.log_path, "ab")
        self.proc = subprocess.Popen([BIN, "serve"], env=self.env(), stdout=log, stderr=subprocess.STDOUT, cwd=self.home)
        if wait:
            self.wait_ready()
        return self

    def wait_ready(self, timeout=30):
        t0 = time.time()
        while time.time() - t0 < timeout:
            if self.proc.poll() is not None:
                raise RuntimeError(f"{self.name} exited early ({self.proc.returncode}); see {self.log_path}")
            try:
                urllib.request.urlopen(self.base + "/api/overview", timeout=2).read()
                return
            except urllib.error.HTTPError:
                return  # 401 with require-key: still up
            except Exception:
                time.sleep(0.2)
        raise RuntimeError(f"{self.name} not ready in {timeout}s")

    def stop(self, timeout=25):
        """SIGTERM (graceful) to our own PID, SIGKILL after the timeout."""
        if not self.proc or self.proc.poll() is not None:
            return
        self.proc.send_signal(signal.SIGTERM)
        try:
            self.proc.wait(timeout)
        except subprocess.TimeoutExpired:
            self.proc.kill()
            self.proc.wait()

    def kill9(self):
        if self.proc and self.proc.poll() is None:
            self.proc.kill()
            self.proc.wait()

    def alive(self):
        return self.proc is not None and self.proc.poll() is None

    def log(self):
        try:
            return open(self.log_path, errors="replace").read()
        except OSError:
            return ""

    # ---- API ----
    def req(self, method, path, body=None, headers=None, raw=None, timeout=120, key=True):
        """Returns (status, parsed-json-or-text, response headers)."""
        h = dict(headers or {})
        data = raw
        if body is not None:
            data = json.dumps(body).encode()
            h.setdefault("content-type", "application/json")
        if key and self.api_key:
            h.setdefault("X-API-Key", self.api_key)
        r = urllib.request.Request(self.base + path, data=data, method=method, headers=h)
        try:
            resp = urllib.request.urlopen(r, timeout=timeout)
            status, txt, hd = resp.status, resp.read(), resp.headers
        except urllib.error.HTTPError as e:
            status, txt, hd = e.code, e.read(), e.headers
        try:
            return status, json.loads(txt), hd
        except Exception:
            return status, txt.decode(errors="replace"), hd

    def api(self, method, path, body=None, **kw):
        """Like req but raises ApiError on non-2xx and returns just the body."""
        s, b, _ = self.req(method, path, body, **kw)
        if s >= 300:
            raise ApiError(s, b)
        return b

    def get(self, path, **kw):
        return self.api("GET", path, **kw)

    def post(self, path, body=None, **kw):
        return self.api("POST", path, {} if body is None else body, **kw)

    def put(self, path, body, **kw):
        return self.api("PUT", path, body, **kw)

    def delete(self, path, **kw):
        return self.api("DELETE", path, **kw)

    def cli(self, *args, key=None, env=None, timeout=120):
        e = self.env()
        e.pop("CAMOUFOX_HEADLESS", None)
        if env:
            e.update(env)
        a = [BIN, *args, "--port", str(self.port)]
        if key:
            a += ["--api-key", key]
        p = subprocess.run(a, env=e, capture_output=True, text=True, timeout=timeout)
        return p.returncode, p.stdout, p.stderr

    # ---- helpers ----
    def create(self, name, os_="win", **settings):
        p = self.post("/api/profiles", {"name": name, "os": os_})
        if settings:
            self.edit(p["id"], **settings)
        return p["id"]

    def profile(self, pid):
        return self.get(f"/api/profiles/{pid}")

    def edit(self, pid, **settings):
        """Merge `settings` (+ name/notes) into the profile's current settings and PUT."""
        cur = self.profile(pid)
        s = dict(cur.get("settings", {}))
        body = {"name": settings.pop("name", cur["name"]), "notes": settings.pop("notes", cur.get("notes")), "settings": s}
        for k in ("os", "osAction"):
            if k in settings:
                body[k] = settings.pop(k)
        s.update(settings)
        return self.put(f"/api/profiles/{pid}", body)

    def overview(self):
        return self.get("/api/overview")

    def prof_row(self, pid):
        for p in self.overview()["profiles"]:
            if p["id"] == pid:
                return p

    def start_profile(self, pid, **kw):
        return self.post(f"/api/profiles/{pid}/start", **kw)

    def stop_profile(self, pid):
        return self.post(f"/api/profiles/{pid}/stop")


# --------------------------------------------------------------------------------------------
# Fake upstream infrastructure (asyncio in a background thread)
# --------------------------------------------------------------------------------------------

SAO_PAULO = {
    "status": "success",
    "country": "Brazil",
    "countryCode": "BR",
    "city": "Sao Paulo",
    "lat": -23.5505,
    "lon": -46.6333,
    "timezone": "America/Sao_Paulo",
    "query": "203.0.113.77",
}
TOKYO = dict(SAO_PAULO, country="Japan", countryCode="JP", city="Tokyo", lat=35.68, lon=139.69, timezone="Asia/Tokyo", query="203.0.113.88")


class Fakes:
    """Event loop thread hosting fake web + HTTP proxy + SOCKS5 proxy servers."""

    def __init__(self):
        self.loop = asyncio.new_event_loop()
        self.t = threading.Thread(target=self.loop.run_forever, daemon=True)
        self.t.start()
        self.servers = []

    def run(self, coro):
        return asyncio.run_coroutine_threadsafe(coro, self.loop).result(15)

    def close(self):
        async def _c():
            for s in self.servers:
                s.close()

        try:
            self.run(_c())
        except Exception:
            pass
        self.loop.call_soon_threadsafe(self.loop.stop)

    async def listen(self, handle, port):
        """Listen on 127.0.0.1; ephemeral ports are picked from 53000+ so they never collide with
        the app's relay ports (CM_PROXY_BASE_PORT 51000/52000 ranges)."""
        if port:
            s = await asyncio.start_server(handle, "127.0.0.1", port)
        else:
            for _ in range(200):
                self._next = getattr(self, "_next", 53000) + 1
                try:
                    s = await asyncio.start_server(handle, "127.0.0.1", self._next)
                    break
                except OSError:
                    continue
        self.servers.append(s)
        return s

    # -- target web server --
    def web(self, geo):
        """Local web server: ip-api.com/json -> geo; everything else -> a tiny HTML page."""
        log = []

        async def handle(r, w):
            try:
                head = await r.readuntil(b"\r\n\r\n")
                lines = head.decode(errors="replace").split("\r\n")
                host = next((x.split(":", 1)[1].strip() for x in lines if x.lower().startswith("host:")), "")
                path = lines[0].split(" ")[1]
                log.append((host, path))
                if host.startswith("ip-api.com"):
                    body, ct = json.dumps(geo).encode(), "application/json"
                else:
                    body, ct = f"<html><head><title>e2e</title></head><body>host={host} path={path}</body></html>".encode(), "text/html"
                w.write(
                    b"HTTP/1.1 200 OK\r\nContent-Type: "
                    + ct.encode()
                    + b"\r\nCache-Control: no-store\r\nConnection: close\r\nContent-Length: "
                    + str(len(body)).encode()
                    + b"\r\n\r\n"
                    + body
                )
                await w.drain()
            except Exception:
                pass
            finally:
                w.close()

        async def mk():
            s = await self.listen(handle, 0)
            return s.sockets[0].getsockname()[1]

        port = self.run(mk())
        return port, log

    async def _pipe(self, r, w):
        try:
            while True:
                d = await r.read(65536)
                if not d:
                    break
                w.write(d)
                await w.drain()
        except Exception:
            pass
        finally:
            try:
                w.close()
            except Exception:
                pass

    async def _tunnel(self, r1, w1, web_port):
        r2, w2 = await asyncio.open_connection("127.0.0.1", web_port)
        await asyncio.gather(self._pipe(r1, w2), self._pipe(r2, w1))

    def http_proxy(self, geo=SAO_PAULO, user="pu", password="pp", port=0):
        """HTTP proxy with Basic auth. Returns obj with .port .log (list of (kind, target, authed))."""
        web_port, web_log = self.web(geo)
        log = []
        want = "Basic " + base64.b64encode(f"{user}:{password}".encode()).decode() if user else None

        async def handle(r, w):
            try:
                head = await r.readuntil(b"\r\n\r\n")
                lines = head.decode(errors="replace").split("\r\n")
                hdr = {x.split(":", 1)[0].lower(): x.split(":", 1)[1].strip() for x in lines[1:] if ":" in x}
                method, target, ver = lines[0].split(" ", 2)
                if want and hdr.get("proxy-authorization") != want:
                    log.append(("407", target, False))
                    w.write(b'HTTP/1.1 407 Proxy Authentication Required\r\nProxy-Authenticate: Basic realm="e2e"\r\nContent-Length: 0\r\n\r\n')
                    await w.drain()
                    w.close()
                    return
                log.append((method, target, True))
                if method == "CONNECT":
                    w.write(b"HTTP/1.1 200 Connection established\r\n\r\n")
                    await w.drain()
                    if not target.endswith(":80"):  # TLS targets: fail fast instead of hanging the browser
                        w.close()
                        return
                    await self._tunnel(r, w, web_port)
                else:
                    r2, w2 = await asyncio.open_connection("127.0.0.1", web_port)
                    tail = target.split("//", 1)[1] if "//" in target else target
                    path = "/" + tail.split("/", 1)[1] if "/" in tail else "/"
                    rest = "\r\n".join(x for x in lines[1:] if x and not x.lower().startswith(("proxy-", "connection:")))
                    w2.write(f"{method} {path} {ver}\r\n{rest}\r\nConnection: close\r\n\r\n".encode())
                    await w2.drain()
                    await self._pipe(r2, w)
            except Exception:
                try:
                    w.close()
                except Exception:
                    pass

        async def mk():
            s = await self.listen(handle, port)
            return s.sockets[0].getsockname()[1]

        o = type("HttpProxy", (), {})()
        o.port, o.log, o.web_log, o.geo, o.user, o.password = self.run(mk()), log, web_log, geo, user, password
        o.server = self.servers[-1]
        o.close = o.server.close
        return o

    def socks5_proxy(self, geo=SAO_PAULO, user="su", password="sp", port=0):
        """SOCKS5 with user/pass. .log = list of ("domain"|"ipv4"|"ipv6"|"authfail", host, port)."""
        web_port, _ = self.web(geo)
        log = []

        async def handle(r, w):
            try:
                _ver, n = await r.readexactly(2)
                methods = await r.readexactly(n)
                if user:
                    if 2 not in methods:
                        w.write(b"\x05\xff")
                        return
                    w.write(b"\x05\x02")
                    await r.readexactly(1)
                    ul = (await r.readexactly(1))[0]
                    u = (await r.readexactly(ul)).decode()
                    pl = (await r.readexactly(1))[0]
                    p = (await r.readexactly(pl)).decode()
                    if (u, p) != (user, password):
                        w.write(b"\x01\x01")
                        await w.drain()
                        log.append(("authfail", u, 0))
                        return
                    w.write(b"\x01\x00")
                else:
                    w.write(b"\x05\x00")
                _v, _cmd, _rsv, atyp = await r.readexactly(4)
                if atyp == 1:
                    host = ".".join(str(b) for b in await r.readexactly(4))
                    kind = "ipv4"
                elif atyp == 3:
                    ln = (await r.readexactly(1))[0]
                    host = (await r.readexactly(ln)).decode()
                    kind = "domain"
                else:
                    host = (await r.readexactly(16)).hex()
                    kind = "ipv6"
                tport = int.from_bytes(await r.readexactly(2), "big")
                log.append((kind, host, tport))
                w.write(b"\x05\x00\x00\x01\x7f\x00\x00\x01\x00\x00")
                await w.drain()
                if tport != 80:
                    return
                await self._tunnel(r, w, web_port)
            except Exception:
                pass
            finally:
                try:
                    w.close()
                except Exception:
                    pass

        async def mk():
            s = await self.listen(handle, port)
            return s.sockets[0].getsockname()[1]

        o = type("Socks5Proxy", (), {})()
        o.port, o.log, o.geo, o.user, o.password = self.run(mk()), log, geo, user, password
        o.server = self.servers[-1]
        o.close = o.server.close
        return o


def dead_port():
    import socket

    import random

    for _ in range(100):
        s = socket.socket()
        p = random.randint(54000, 59000)
        try:
            s.bind(("127.0.0.1", p))
        except OSError:
            continue
        finally:
            s.close()
        return p
    raise RuntimeError("no free port")


# --------------------------------------------------------------------------------------------
# Results
# --------------------------------------------------------------------------------------------


class Results:
    def __init__(self):
        self.rows = []  # (scenario, check, ok, detail)
        self.cur = "?"

    def scenario(self, name):
        self.cur = name

    def check(self, name, ok, detail=""):
        ok = bool(ok)
        self.rows.append((self.cur, name, ok, str(detail)[:400]))
        print(("  ok   " if ok else "  FAIL ") + name + ("" if ok else f"  -> {str(detail)[:300]}"), flush=True)
        return ok

    def note(self, msg):
        print("  note " + msg, flush=True)

    def run(self, name, fn, *a):
        self.scenario(name)
        print(f"\n== {name}", flush=True)
        try:
            fn(*a)
        except Exception as e:
            self.rows.append((name, "scenario crashed", False, f"{type(e).__name__}: {e}"))
            print(f"  FAIL scenario crashed: {type(e).__name__}: {e}\n{traceback.format_exc()[-1500:]}", flush=True)

    def summary(self):
        by = {}
        for s, c, ok, d in self.rows:
            by.setdefault(s, []).append((c, ok, d))
        print("\n===== SUMMARY =====")
        bad = 0
        for s, rs in by.items():
            f = [r for r in rs if not r[1]]
            bad += len(f)
            print(f"{'PASS' if not f else 'FAIL'}  {s}  ({len(rs) - len(f)}/{len(rs)})")
            for c, _, d in f:
                print(f"        x {c}: {d[:200]}")
        return bad


def connect(pw, ws):
    """Playwright firefox.connect to the bound browser; returns (browser, persistent context)."""
    b = pw.firefox.connect(ws, timeout=30000)
    return b, b.contexts[0]
