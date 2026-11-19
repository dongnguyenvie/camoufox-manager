"""Firefox extensions (.xpi) for a persistent profile.

`extensions install --user-data DIR [XPI...]` makes `<DIR>/extensions/` hold exactly the given
add-ons (plus anything the user put there themselves) and prints
{"ok": true, "installed": [ids], "removed": [ids]}. `launch` calls the same `sync()` before the
browser opens, so the assignment of the app is always what Firefox sees.

How it loads: Firefox scans `<profile>/extensions/<addon-id>.xpi` at startup (a "sideload" into the
profile scope). Two things make that work for unsigned / user-uploaded add-ons:
  * the file name must be exactly the add-on id from the manifest (`browser_specific_settings.gecko.id`
    or `applications.gecko.id`), so an add-on without an id cannot be installed;
  * `extensions.autoDisableScopes = 0` (otherwise Firefox installs sideloaded add-ons as disabled and
    waits for a prompt that never comes), `extensions.enabledScopes = 15`, and
    `xpinstall.signatures.required = false` (Camoufox is not a release-signed build, so unsigned
    add-ons are accepted). Measured with Camoufox 0.5.9: without these prefs the add-on is ignored.

What is ours is recorded in `<DIR>/extensions/.cm-managed.json`; only those files are ever removed,
so add-ons installed by hand inside the browser are left alone. Removing the file makes Firefox
uninstall the add-on at the next start.

Limits: only a Manifest V2 content-script add-on was tested (Camoufox 0.5.9); add-ons are
installed at launch, so assignment changes need a restart of the profile; WebExtension APIs
Camoufox does not ship do nothing; an add-on without a Gecko id in its manifest is refused.
"""

import json
import os
import shutil
import zipfile

from common import emit

MANAGED = ".cm-managed.json"
PREFS = {
    "extensions.autoDisableScopes": 0,
    "extensions.enabledScopes": 15,
    "xpinstall.signatures.required": False,
}


def addon_id(xpi):
    """Add-on id from the manifest of an .xpi (zip); ValueError when it cannot be used."""
    try:
        with zipfile.ZipFile(xpi) as z:
            manifest = json.loads(z.read("manifest.json").decode("utf-8-sig"))
    except KeyError:
        raise ValueError(f"{os.path.basename(xpi)}: no manifest.json")
    except (zipfile.BadZipFile, json.JSONDecodeError, UnicodeDecodeError) as e:
        raise ValueError(f"{os.path.basename(xpi)}: not a valid extension ({e})")
    for key in ("browser_specific_settings", "applications"):
        gecko = (manifest.get(key) or {}).get("gecko") or {}
        if isinstance(gecko.get("id"), str) and gecko["id"].strip():
            ident = gecko["id"].strip()
            if "/" in ident or "\\" in ident or ident.startswith("."):
                raise ValueError(f"{os.path.basename(xpi)}: unusable add-on id {ident!r}")
            return ident
    raise ValueError(f"{os.path.basename(xpi)}: manifest has no browser_specific_settings.gecko.id")


def _same(a, b):
    try:
        if os.path.getsize(a) != os.path.getsize(b):
            return False
        with open(a, "rb") as fa, open(b, "rb") as fb:
            return fa.read() == fb.read()
    except OSError:
        return False


def sync(user_data, xpis):
    """Make `<user_data>/extensions/` hold `xpis` (managed copies). Returns (installed, removed) ids."""
    ext_dir = os.path.join(user_data, "extensions")
    record = os.path.join(ext_dir, MANAGED)
    try:
        with open(record, encoding="utf-8") as f:
            managed = [x for x in json.load(f) if isinstance(x, str)]
    except (OSError, ValueError):
        managed = []
    wanted = {}
    for x in xpis:
        wanted[addon_id(x)] = x
    if not wanted and not managed:
        return [], []
    os.makedirs(ext_dir, exist_ok=True)
    removed = []
    for ident in managed:
        if ident not in wanted:
            try:
                os.remove(os.path.join(ext_dir, ident + ".xpi"))
            except FileNotFoundError:
                pass
            removed.append(ident)
    for ident, src in wanted.items():
        dest = os.path.join(ext_dir, ident + ".xpi")
        if not _same(src, dest):  # keep the mtime when unchanged: Firefox re-reads the add-on on change
            tmp = dest + ".tmp"
            shutil.copyfile(src, tmp)
            os.replace(tmp, dest)
    with open(record + ".tmp", "w", encoding="utf-8") as f:
        json.dump(sorted(wanted), f)
    os.replace(record + ".tmp", record)
    return sorted(wanted), removed


def cmd_extensions(args):
    installed, removed = sync(args.user_data, args.xpi)
    emit({"ok": True, "installed": installed, "removed": removed})
