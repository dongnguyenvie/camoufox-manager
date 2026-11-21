"""In-page probes run through Playwright against a started profile."""

PROBE_JS = r"""async () => {
  const fnv = (s) => { let h = 0x811c9dc5; for (let i = 0; i < s.length; i++) { h ^= s.charCodeAt(i); h = Math.imul(h, 16777619) >>> 0; } return h.toString(16); };
  const o = {
    ua: navigator.userAgent, platform: navigator.platform, lang: navigator.language, langs: [...navigator.languages],
    tz: Intl.DateTimeFormat().resolvedOptions().timeZone, tzoff: new Date().getTimezoneOffset(),
    sw: screen.width, sh: screen.height, aw: screen.availWidth, ah: screen.availHeight,
    ow: window.outerWidth, oh: window.outerHeight, iw: innerWidth, ih: innerHeight,
    cpu: navigator.hardwareConcurrency, dpr: devicePixelRatio, secure: isSecureContext,
    hasRTC: typeof RTCPeerConnection !== 'undefined',
  };
  try {
    const c = document.createElement('canvas'); c.width = 240; c.height = 60;
    const x = c.getContext('2d');
    x.fillStyle = '#f60'; x.fillRect(10, 5, 100, 40); x.fillStyle = '#069'; x.font = '18px Arial';
    x.fillText('cm e2e canvas \u{1F600} 123', 4, 30); x.strokeStyle = 'rgba(30,120,40,0.6)'; x.beginPath(); x.arc(150, 30, 20, 0, 6.28); x.stroke();
    o.canvasURL = fnv(c.toDataURL());
    const c2 = document.createElement('canvas'); c2.width = 120; c2.height = 40;
    const x2 = c2.getContext('2d'); const gr = x2.createLinearGradient(0, 0, 120, 0); gr.addColorStop(0, '#e33'); gr.addColorStop(1, '#33e');
    x2.fillStyle = gr; x2.fillRect(0, 0, 120, 40); x2.fillStyle = 'rgba(0,0,0,0.5)'; x2.fillRect(20, 10, 60, 20);
    o.canvasSimpleURL = fnv(c2.toDataURL());
    o.canvasSimpleURL2 = fnv(c2.toDataURL());
    o.canvasBlob = await Promise.race([new Promise((res) => c2.toBlob(async (b) => res(fnv(Array.from(new Uint8Array(await b.arrayBuffer())).join(','))), 'image/png')), new Promise((res) => setTimeout(() => res('timeout'), 4000))]);
    o.canvasData = fnv(Array.from(x.getImageData(0, 0, 240, 60).data).join(','));
  } catch (e) { o.canvasErr = String(e); }
  try {
    const c = document.createElement('canvas'); c.width = 64; c.height = 64;
    const gl = c.getContext('webgl');
    const vs = gl.createShader(gl.VERTEX_SHADER); gl.shaderSource(vs, 'attribute vec2 p;varying vec2 v;void main(){v=p;gl_Position=vec4(p,0.,1.);}'); gl.compileShader(vs);
    const fs = gl.createShader(gl.FRAGMENT_SHADER); gl.shaderSource(fs, 'precision mediump float;varying vec2 v;void main(){gl_FragColor=vec4(v*0.5+0.5,0.37,1.);}'); gl.compileShader(fs);
    const pr = gl.createProgram(); gl.attachShader(pr, vs); gl.attachShader(pr, fs); gl.linkProgram(pr); gl.useProgram(pr);
    const b = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, b); gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, 0, 1]), gl.STATIC_DRAW);
    const l = gl.getAttribLocation(pr, 'p'); gl.enableVertexAttribArray(l); gl.vertexAttribPointer(l, 2, gl.FLOAT, false, 0, 0);
    gl.clearColor(0.1, 0.2, 0.3, 1); gl.clear(gl.COLOR_BUFFER_BIT); gl.drawArrays(gl.TRIANGLES, 0, 3);
    const px = new Uint8Array(64 * 64 * 4); gl.readPixels(0, 0, 64, 64, gl.RGBA, gl.UNSIGNED_BYTE, px);
    o.webgl = fnv(Array.from(px).join(','));
    const d = gl.getExtension('WEBGL_debug_renderer_info');
    o.glRenderer = d ? gl.getParameter(d.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER);
    o.glVendor = d ? gl.getParameter(d.UNMASKED_VENDOR_WEBGL) : gl.getParameter(gl.VENDOR);
  } catch (e) { o.webglErr = String(e); }
  try {
    const ac = new OfflineAudioContext(1, 44100, 44100);
    const osc = ac.createOscillator(); osc.type = 'triangle'; osc.frequency.value = 10000;
    const cmp = ac.createDynamicsCompressor();
    osc.connect(cmp); cmp.connect(ac.destination); osc.start(0);
    const buf = await Promise.race([ac.startRendering(), new Promise((_, rej) => setTimeout(() => rej(new Error('audio timeout')), 6000))]);
    const d = buf.getChannelData(0);
    let s = 0; for (let i = 4500; i < 5000; i++) s += Math.abs(d[i]);
    o.audio = fnv(Array.from(d.slice(4000, 6000)).join(','));
    o.audioSum = s;
  } catch (e) { o.audioErr = String(e); }
  return o;
}"""

# Collect ICE candidates (no STUN servers: only host / mDNS / replaced candidates can appear).
ICE_JS = r"""async () => {
  if (typeof RTCPeerConnection === 'undefined') return { hasRTC: false, candidates: [] };
  const out = [];
  const pc = new RTCPeerConnection({ iceServers: [] });
  pc.createDataChannel('x');
  pc.onicecandidate = (e) => { if (e.candidate) out.push(e.candidate.candidate); };
  await pc.setLocalDescription(await pc.createOffer());
  await new Promise((res) => { const t = setTimeout(res, 6000); pc.onicegatheringstatechange = () => { if (pc.iceGatheringState === 'complete') { clearTimeout(t); res(); } }; });
  const sdp = pc.localDescription ? pc.localDescription.sdp : '';
  pc.close();
  return { hasRTC: true, candidates: out, sdp };
}"""

GEO_JS = r"""() => new Promise((res) => {
  setTimeout(() => res({ err: 'timeout (no answer/prompt pending)' }), 9000);
  if (!navigator.geolocation) return res({ err: 'no geolocation' });
  navigator.geolocation.getCurrentPosition(
    (p) => res({ lat: p.coords.latitude, lon: p.coords.longitude }),
    (e) => res({ err: e.code + ':' + e.message }),
    { timeout: 8000 });
})"""

PERM_JS = r"""async () => { try { return (await navigator.permissions.query({ name: 'geolocation' })).state; } catch (e) { return 'err ' + e; } }"""


def ev(page, js):
    """Evaluate in the page's MAIN world when the profile allows it (canvas mode "stable"/unique turns
    on main_world_eval so the init script's patches are visible); otherwise the isolated world, where
    typed-array reads over Xrays (audio) fail and are reported as audioErr."""
    try:
        return page.evaluate("mw:" + js)
    except Exception as e:
        if "Main world evaluation is disabled" not in str(e):
            raise
        return page.evaluate(js)


def probe(page, url):
    page.goto(url, wait_until="load", timeout=30000)
    return ev(page, PROBE_JS)


def host_ips():
    """Non-loopback IPv4 addresses of this machine (what a WebRTC leak would show)."""
    import re
    import subprocess

    out = subprocess.run(["ifconfig"], capture_output=True, text=True).stdout
    ips = set(re.findall(r"inet (\d+\.\d+\.\d+\.\d+)", out))
    return sorted(i for i in ips if not i.startswith("127."))


def host_timezone():
    import time

    return time.tzname, __import__("os").path.realpath("/etc/localtime")
