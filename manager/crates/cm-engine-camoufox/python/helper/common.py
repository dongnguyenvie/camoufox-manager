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


# ---- unique stable canvas / WebGL / audio (CM-1.20) ----
# Camoufox 0.5.9 / Firefox 158 reads no per-profile canvas, WebGL-pixel or audio seed: `canvas:seed`,
# `canvas:aaOffset`, `canvas:aaCapOffset` and `audio:seed` have no consumer in the browser binary
# (daijro/camoufox#721). Its own protection (baselineFingerprintingProtection placeholder) is keyed
# by a random per-session key, so it changes on every launch. The only way to get a value that is
# per profile AND stable is to perturb the READBACK deterministically from a pinned seed, which is
# what this init script does (runs in every frame of the profile's context, before page scripts).
# Noise depends only on (seed, pixel/sample position), never on time or call count, so repeated
# reads agree; it touches copies returned to the page and never the canvas itself.
CANVAS_INIT_JS = r"""(() => {
  const SEED = (__SEED__) >>> 0;
  const mix = (a, b) => {
    let h = Math.imul((a ^ SEED) >>> 0, 0x45d9f3b) ^ Math.imul((b + 0x9e3779b9) | 0, 0x2c1b3c6d);
    h = Math.imul(h ^ (h >>> 16), 0x45d9f3b); h ^= h >>> 16;
    return h >>> 0;
  };
  const natives = new Map();
  const defs = (proto, name, make) => {
    const d = Object.getOwnPropertyDescriptor(proto, name);
    if (!d || typeof d.value !== 'function') return;
    const orig = d.value;
    const wrapped = make(orig);
    try { Object.defineProperty(wrapped, 'name', { value: orig.name, configurable: true }); } catch (_) {}
    try { Object.defineProperty(wrapped, 'length', { value: orig.length, configurable: true }); } catch (_) {}
    natives.set(wrapped, orig);
    Object.defineProperty(proto, name, { value: wrapped, writable: d.writable, enumerable: d.enumerable, configurable: d.configurable });
  };
  // Pixels (RGBA, row-major): about 1 in 4 visible pixels gets one channel moved by +-1.
  const noisePixels = (data, w, h, chans, ox, oy) => {
    const n = w * h;
    for (let i = 0; i < n; i++) {
      const x = (i % w) + ox, y = ((i / w) | 0) + oy;
      const k = mix(x, y * 65537);
      if ((k & 3) !== 0) continue;
      const p = i * chans;
      if (chans === 4 && data[p + 3] === 0) continue; // keep fully transparent pixels untouched
      const c = (k >>> 2) % 3, v = data[p + c];
      data[p + c] = (k & 0x80) ? (v < 255 ? v + 1 : v - 1) : (v > 0 ? v - 1 : v + 1);
    }
  };
  const origGetContext = HTMLCanvasElement.prototype.getContext;
  const origGetImageData = CanvasRenderingContext2D.prototype.getImageData;
  const tmpCanvas = (src) => {
    const w = src.width, h = src.height;
    if (!(w > 0 && h > 0) || w * h > 64e6) return null;
    const t = document.createElement('canvas'); t.width = w; t.height = h;
    const cx = origGetContext.call(t, '2d');
    cx.drawImage(src, 0, 0);
    const img = origGetImageData.call(cx, 0, 0, w, h);
    noisePixels(img.data, w, h, 4, 0, 0);
    cx.putImageData(img, 0, 0);
    return t;
  };
  defs(CanvasRenderingContext2D.prototype, 'getImageData', (orig) => ({ getImageData(...a) {
    const img = orig.apply(this, a);
    try { noisePixels(img.data, img.width, img.height, 4, a[0] | 0, a[1] | 0); } catch (_) {}
    return img;
  } }).getImageData);
  for (const m of ['toDataURL', 'toBlob']) {
    defs(HTMLCanvasElement.prototype, m, (orig) => ({ [m](...a) {
      let t = null;
      try { t = tmpCanvas(this); } catch (_) {}
      return orig.apply(t || this, a);
    } })[m]);
  }
  if (typeof OffscreenCanvas !== 'undefined') {
    defs(OffscreenCanvas.prototype, 'convertToBlob', (orig) => ({ convertToBlob(...a) {
      let t = null;
      try { t = tmpCanvas(this); } catch (_) {}
      return orig.apply(t || this, a);
    } }).convertToBlob);
  }
  for (const C of [window.WebGLRenderingContext, window.WebGL2RenderingContext]) {
    if (!C) continue;
    defs(C.prototype, 'readPixels', (orig) => ({ readPixels(...a) {
      const r = orig.apply(this, a);
      try {
        const [x, y, w, h, fmt, type, buf] = a;
        const chans = fmt === 0x1908 ? 4 : fmt === 0x1907 ? 3 : 0;
        if (chans && type === 0x1401 && buf && buf.length >= w * h * chans && w > 0 && h > 0) noisePixels(buf, w, h, chans, x, y);
      } catch (_) {}
      return r;
    } }).readPixels);
  }
  if (typeof AudioBuffer !== 'undefined') {
    const done = new WeakSet();
    const ensure = (buf) => {
      if (done.has(buf)) return;
      done.add(buf);
      for (let c = 0; c < buf.numberOfChannels; c++) {
        const d = rawGet.call(buf, c);
        for (let i = 0; i < d.length; i++) {
          const v = d[i];
          if (v !== 0) d[i] = v + ((mix(i, c + 77) & 1) ? 1 : -1) * 1e-7 * (1 + (mix(c, i) % 7));
        }
      }
    };
    const rawGet = Object.getOwnPropertyDescriptor(AudioBuffer.prototype, 'getChannelData').value;
    defs(AudioBuffer.prototype, 'getChannelData', (orig) => ({ getChannelData(...a) { const r = orig.apply(this, a); try { ensure(this); } catch (_) {} return r; } }).getChannelData);
    defs(AudioBuffer.prototype, 'copyFromChannel', (orig) => ({ copyFromChannel(...a) { try { ensure(this); } catch (_) {} return orig.apply(this, a); } }).copyFromChannel);
  }
  if (typeof AnalyserNode !== 'undefined') {
    for (const m of ['getFloatFrequencyData', 'getFloatTimeDomainData']) {
      defs(AnalyserNode.prototype, m, (orig) => ({ [m](a) {
        const r = orig.call(this, a);
        try { for (let i = 0; i < a.length; i++) if (Number.isFinite(a[i]) && a[i] !== 0) a[i] += ((mix(i, 5) & 1) ? 1 : -1) * 1e-4 * (1 + (mix(i, 9) % 5)); } catch (_) {}
        return r;
      } })[m]);
    }
  }
  const ts = Function.prototype.toString;
  const fake = { toString() { return ts.call(natives.get(this) || this); } }.toString;
  natives.set(fake, ts);
  Object.defineProperty(Function.prototype, 'toString', { value: fake, writable: true, enumerable: false, configurable: true });
})();"""


def canvas_seed(config, profile_id):
    """The profile's canvas seed: pinned in the machine (`cm:canvasSeed`), else derived from the id."""
    import hashlib

    s = config.get("cm:canvasSeed")
    if isinstance(s, int) and 0 <= s < 2**32:
        return s
    return int.from_bytes(hashlib.sha256(("cm-canvas:" + str(profile_id)).encode()).digest()[:4], "big")
