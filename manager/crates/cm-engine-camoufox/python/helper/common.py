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
