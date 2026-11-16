"""Shared helpers: JSON output and Camoufox's chunked env encoding."""

import json
import sys

OS_NAMES = {"win": "windows", "mac": "macos", "lin": "linux"}


def emit(obj):
    sys.stdout.write(json.dumps(obj) + "\n")
    sys.stdout.flush()


def decode_chunks(env, prefix):
    keys = sorted((k for k in env if k.startswith(prefix)), key=lambda k: int(k[len(prefix):]))
    return json.loads("".join(env[k] for k in keys)) if keys else {}


def encode_chunks(env, prefix, value):
    for k in [k for k in env if k.startswith(prefix)]:
        del env[k]
    data = json.dumps(value, ensure_ascii=True, separators=(",", ":"))
    size = 32767
    for i in range(0, len(data), size):
        env[f"{prefix}{i // size + 1}"] = data[i:i + size]


def parse_size(value, what):
    """`WxH` -> (w, h) ints, within sane bounds."""
    try:
        w, h = (int(x) for x in str(value).lower().split("x"))
    except ValueError:
        raise ValueError(f"{what} must look like 1920x1080")
    if not (200 <= w <= 10000 and 200 <= h <= 10000):
        raise ValueError(f"{what} {w}x{h} is out of range")
    return w, h


def apply_screen(config, size):
    """Override the pinned screen size, keeping the taskbar/dock height and fitting the window."""
    w, h = size
    taskbar = max(0, int(config.get("screen.height", h)) - int(config.get("screen.availHeight", h)))
    config["screen.width"], config["screen.height"] = w, h
    config["screen.availWidth"], config["screen.availHeight"] = w, h - taskbar
    config["window.outerWidth"], config["window.outerHeight"] = w, h - taskbar


def check_timezone(tz):
    from zoneinfo import ZoneInfo, ZoneInfoNotFoundError

    try:
        ZoneInfo(tz)
    except (ZoneInfoNotFoundError, ValueError):
        raise ValueError(f"unknown timezone {tz!r}")
