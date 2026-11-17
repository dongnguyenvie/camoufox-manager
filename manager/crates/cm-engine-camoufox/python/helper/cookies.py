"""`cookies export|import --user-data DIR --file FILE [--mode domains|all]`.

Reads/writes the cookie store of a CLOSED profile directly: Firefox keeps cookies in
`<user-data>/cookies.sqlite` (table moz_cookies). No browser is started (so this works without
the Camoufox browser, in milliseconds, and cannot disturb the pinned fingerprint). The columns
are discovered with PRAGMA table_info, so a Firefox that adds a column does not break it.

FILE is JSON in both directions: a list of
  {name, value, domain, path, expires (unix seconds | null), httpOnly, secure,
   sameSite ("Strict"|"Lax"|"None"|null), hostOnly}
`domain` follows the browser-extension convention: a leading dot unless hostOnly.
The Rust side validates and converts to/from the user-facing formats.
"""

import json
import os
import sqlite3
import time

from common import emit

# Schema written by Camoufox's Firefox (cookies.sqlite user_version 17, read from a real profile),
# used only when the profile has never been started and so has no cookie store yet; Firefox
# migrates other versions on its own.
SCHEMA = """
CREATE TABLE moz_cookies (
  id INTEGER PRIMARY KEY,
  originAttributes TEXT NOT NULL DEFAULT '',
  name TEXT,
  value TEXT,
  host TEXT,
  path TEXT,
  expiry INTEGER,
  lastAccessed INTEGER,
  creationTime INTEGER,
  isSecure INTEGER,
  isHttpOnly INTEGER,
  inBrowserElement INTEGER DEFAULT 0,
  sameSite INTEGER DEFAULT 0,
  schemeMap INTEGER DEFAULT 0,
  isPartitionedAttributeSet INTEGER DEFAULT 0,
  updateTime INTEGER,
  CONSTRAINT moz_uniqueid UNIQUE (name, host, path, originAttributes)
);
"""

# Firefox sameSite values: NONE 0, LAX 1, STRICT 2, UNSET 256 (no attribute).
SAMESITE = {"None": 0, "Lax": 1, "Strict": 2}
SAMESITE_UNSET = 256
SAMESITE_NAME = {0: "None", 1: "Lax", 2: "Strict"}


def _firefox_running(user_data):
    """`lock` is a symlink "127.0.0.1:+PID" while Firefox runs (and stale after a crash)."""
    lock = os.path.join(user_data, "lock")
    try:
        target = os.readlink(lock)
    except OSError:
        return False
    try:
        pid = int(target.rsplit("+", 1)[1])
        os.kill(pid, 0)
    except (IndexError, ValueError, ProcessLookupError):
        return False
    except PermissionError:
        return True
    return True


def _open(user_data, create):
    path = os.path.join(user_data, "cookies.sqlite")
    if not os.path.exists(path):
        if not create:
            return None
        os.makedirs(user_data, exist_ok=True)
    db = sqlite3.connect(path, timeout=10)
    cols = [r[1] for r in db.execute("PRAGMA table_info(moz_cookies)")]
    if not cols:
        if not create:
            db.close()
            return None
        db.executescript(SCHEMA)
        db.execute("PRAGMA user_version = 17")
    return db


def _export(db):
    cols = {r[1] for r in db.execute("PRAGMA table_info(moz_cookies)")}
    same = "sameSite" if "sameSite" in cols else "0"
    out = []
    now = time.time()
    for name, value, host, path, expiry, secure, http_only, ss in db.execute(
        f"SELECT name, value, host, path, expiry, isSecure, isHttpOnly, {same} FROM moz_cookies ORDER BY host, name"
    ):
        # Firefox stores the expiry in milliseconds since 2021; older profiles hold seconds.
        exp = expiry / 1000 if expiry and expiry > 100_000_000_000 else expiry
        if exp and exp < now:
            continue  # expired, Firefox will purge it
        out.append({
            "name": name, "value": value, "domain": host, "path": path or "/",
            "expires": float(exp) if exp else None,
            "httpOnly": bool(http_only), "secure": bool(secure),
            "sameSite": SAMESITE_NAME.get(ss),
            "hostOnly": not host.startswith("."),
        })
    return out


def _import(db, cookies, mode):
    cols = [r[1] for r in db.execute("PRAGMA table_info(moz_cookies)")]
    has = set(cols)
    micros = int(time.time() * 1_000_000)
    # Millisecond expiry is the Firefox >= 2021 convention. Detect it from the existing data and
    # fall back to milliseconds for an empty store (Camoufox ships a recent Firefox).
    ms = True
    row = db.execute("SELECT MAX(expiry) FROM moz_cookies").fetchone()
    if row and row[0] and row[0] < 100_000_000_000:
        ms = False
    with db:
        if mode == "all":
            db.execute("DELETE FROM moz_cookies")
        else:
            for host in {c["domain"].lstrip(".") for c in cookies}:
                db.execute("DELETE FROM moz_cookies WHERE host IN (?, ?)", (host, "." + host))
        n = 0
        for c in cookies:
            host = c["domain"]
            exp = int(c["expires"] or 0)
            secure = 1 if c.get("secure") else 0
            ss = SAMESITE.get(c.get("sameSite") or "", SAMESITE_UNSET)
            vals = {
                "originAttributes": "",
                "name": c["name"], "value": c["value"], "host": host, "path": c.get("path") or "/",
                "expiry": exp * 1000 if ms else exp,
                "lastAccessed": micros, "creationTime": micros, "updateTime": micros,
                "isSecure": secure, "isHttpOnly": 1 if c.get("httpOnly") else 0,
                "inBrowserElement": 0, "sameSite": ss, "rawSameSite": ss,
                "schemeMap": 0,  # what Playwright's add_cookies writes too
                "isPartitionedAttributeSet": 0,
            }
            use = [k for k in vals if k in has]
            db.execute(
                f"INSERT OR REPLACE INTO moz_cookies ({', '.join(use)}) VALUES ({', '.join('?' * len(use))})",
                [vals[k] for k in use],
            )
            n += 1
    return n


def cmd_cookies(args):
    if _firefox_running(args.user_data):
        emit({"error": "the profile is running; stop it first", "code": "running"})
        raise SystemExit(1)
    if args.action == "export":
        db = _open(args.user_data, create=False)
        cookies = _export(db) if db else []
        if db:
            db.close()
        with open(args.file, "w", encoding="utf-8") as f:
            json.dump(cookies, f)
        emit({"ok": True, "count": len(cookies)})
        return
    with open(args.file, encoding="utf-8") as f:
        cookies = json.load(f)
    db = _open(args.user_data, create=True)
    try:
        n = _import(db, cookies, args.mode)
        db.execute("PRAGMA wal_checkpoint(TRUNCATE)")
    finally:
        db.close()
    emit({"ok": True, "count": n})
