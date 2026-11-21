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
  // Deterministic PNG encoder (stored deflate blocks): the native encoder re-applies Camoufox's
  // per-session noise, so for PNG we encode the seeded-noised pixels ourselves (B3).
  const crcT = new Uint32Array(256);
  for (let n = 0; n < 256; n++) { let c = n; for (let k = 0; k < 8; k++) c = (c & 1) ? (0xedb88320 ^ (c >>> 1)) : (c >>> 1); crcT[n] = c >>> 0; }
  const crc32 = (b, s, e) => { let c = 0xffffffff; for (let i = s; i < e; i++) c = crcT[(c ^ b[i]) & 255] ^ (c >>> 8); return (c ^ 0xffffffff) >>> 0; };
  const encodePng = (img) => {
    const w = img.width, h = img.height, px = img.data, stride = w * 4 + 1;
    const raw = new Uint8Array(stride * h);
    for (let y = 0; y < h; y++) raw.set(px.subarray(y * w * 4, (y + 1) * w * 4), y * stride + 1);
    const nb = Math.max(1, Math.ceil(raw.length / 65535));
    const z = new Uint8Array(2 + raw.length + nb * 5 + 4);
    let p = 0; z[p++] = 0x78; z[p++] = 0x01;
    for (let o = 0, i = 0; i < nb; i++, o += 65535) {
      const n = Math.min(65535, raw.length - o);
      z[p++] = i === nb - 1 ? 1 : 0; z[p++] = n & 255; z[p++] = n >>> 8; z[p++] = ~n & 255; z[p++] = (~n >>> 8) & 255;
      z.set(raw.subarray(o, o + n), p); p += n;
    }
    let a = 1, b = 0;
    for (let i = 0; i < raw.length; i++) { a = (a + raw[i]) % 65521; b = (b + a) % 65521; }
    const ad = ((b << 16) | a) >>> 0;
    z[p++] = ad >>> 24; z[p++] = (ad >>> 16) & 255; z[p++] = (ad >>> 8) & 255; z[p++] = ad & 255;
    const out = new Uint8Array(8 + 25 + 12 + z.length + 12);
    const dv = new DataView(out.buffer);
    out.set([137, 80, 78, 71, 13, 10, 26, 10]);
    let q = 8;
    const chunk = (type, data) => {
      dv.setUint32(q, data.length); q += 4;
      for (let i = 0; i < 4; i++) out[q + i] = type.charCodeAt(i);
      out.set(data, q + 4);
      dv.setUint32(q + 4 + data.length, crc32(out, q, q + 4 + data.length)); q += 8 + data.length;
    };
    const ihdr = new Uint8Array(13), iv = new DataView(ihdr.buffer);
    iv.setUint32(0, w); iv.setUint32(4, h); ihdr[8] = 8; ihdr[9] = 6;
    chunk('IHDR', ihdr); chunk('IDAT', z.subarray(0, p)); chunk('IEND', new Uint8Array(0));
    return out;
  };
  const isPng = (t) => t === undefined || t === null || String(t).toLowerCase() === 'image/png' || !/^image\/(jpeg|webp)$/i.test(String(t));
  const pngBytes = (src) => {
    const w = src.width, h = src.height;
    if (!(w > 0 && h > 0) || w * h > 64e6) return null;
    const t = document.createElement('canvas'); t.width = w; t.height = h;
    const cx = origGetContext.call(t, '2d');
    cx.drawImage(src, 0, 0);
    const img = origGetImageData.call(cx, 0, 0, w, h);
    noisePixels(img.data, w, h, 4, 0, 0);
    return encodePng(img);
  };
  const b64 = (u8) => { let s = ''; for (let i = 0; i < u8.length; i += 32768) s += String.fromCharCode.apply(null, u8.subarray(i, i + 32768)); return btoa(s); };
  defs(HTMLCanvasElement.prototype, 'toDataURL', (orig) => ({ toDataURL(...a) {
    let u = null;
    try { if (isPng(a[0])) { const r = pngBytes(this); if (r) u = 'data:image/png;base64,' + b64(r); } } catch (_) {}
    if (u !== null) return u;
    let t = null;
    try { t = tmpCanvas(this); } catch (_) {}
    return orig.apply(t || this, a);
  } }).toDataURL);
  defs(HTMLCanvasElement.prototype, 'toBlob', (orig) => ({ toBlob(...a) {
    let r = null;
    try { if (typeof a[0] === 'function' && isPng(a[1])) r = pngBytes(this); } catch (_) {}
    if (r) { const cb = a[0]; setTimeout(() => cb(new Blob([r], { type: 'image/png' })), 0); return; }
    let t = null;
    try { t = tmpCanvas(this); } catch (_) {}
    return orig.apply(t || this, a);
  } }).toBlob);
  if (typeof OffscreenCanvas !== 'undefined') {
    defs(OffscreenCanvas.prototype, 'convertToBlob', (orig) => ({ convertToBlob(...a) {
      try { if (isPng(a[0] && a[0].type)) { const r = pngBytes(this); if (r) return Promise.resolve(new Blob([r], { type: 'image/png' })); } } catch (_) {}
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
