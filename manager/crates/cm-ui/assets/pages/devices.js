/* Devices page (phase 16, D9): pair this install with another one over the user's Tailscale.
   Sections: Tailscale status · make this device reachable (exact `tailscale serve` command, run only
   after confirm; plain-HTTP 100.x fallback) · my pairing link + QR + code countdown + regenerate ·
   add a device by pasting its link · paired devices (last seen, ping, revoke, remove).
   API: /api/devices/… (see cm-api devices.rs). The QR encoder below is self-contained (byte mode,
   ECC level M, versions 1–10, enough for a cmsync:// link), written for this file. */

I18N.add({
  en: {
    'nav.devices': 'Devices',
    'devices.intro': 'Pair this app with Camoufox Manager on your other Mac, over your own Tailscale. Nothing goes through a third-party server.',
    'devices.status': 'Tailscale',
    'devices.state': 'Status',
    'devices.ts.not_installed': 'Not installed',
    'devices.ts.not_running': 'Not running',
    'devices.ts.needs_login': 'Needs login',
    'devices.ts.stopped': 'Switched off',
    'devices.ts.starting': 'Starting…',
    'devices.ts.running': 'Running',
    'devices.ts.error': 'Error',
    'devices.hint.not_installed': 'Install Tailscale from tailscale.com and log in on both Macs. Device sync stays off until then; the rest of the app works as usual.',
    'devices.hint.not_running': 'Open the Tailscale app on this Mac.',
    'devices.hint.needs_login': 'Log in to Tailscale (or ask your admin to approve this machine).',
    'devices.hint.stopped': 'Connect Tailscale from its menu bar icon.',
    'devices.hint.starting': 'Wait a moment, then refresh.',
    'devices.hint.error': 'Tailscale reported a problem. Check the Tailscale app.',
    'devices.version': 'Version',
    'devices.thisDevice': 'This device',
    'devices.fingerprint': 'Key fingerprint',
    'devices.magicDns': 'Tailnet name',
    'devices.ips': 'Tailscale IPs',
    'devices.https': 'HTTPS certificates',
    'devices.on': 'on',
    'devices.off': 'off',
    'devices.listeners': 'Sync listener',
    'devices.reach': 'Make this device reachable',
    'devices.reachIntro': 'The other device connects to this app\'s sync port through Tailscale. Only devices on your tailnet can reach it, never the public internet.',
    'devices.reachNow': 'Reachable at',
    'devices.mode.serve': 'HTTPS via tailscale serve',
    'devices.mode.tailnetIp': 'plain HTTP inside WireGuard (100.x)',
    'devices.mode.advertise': 'development address (CM_SYNC_ADVERTISE)',
    'devices.notReachable': 'Not reachable yet.',
    'devices.serveCmd': 'This command will run on this Mac:',
    'devices.serveRun': 'Make reachable',
    'devices.serveTitle': 'Change your Tailscale settings?',
    'devices.serveBody': 'Camoufox Manager will run the command below. It publishes the sync port to devices on your tailnet over HTTPS (not to the internet). It stays until you undo it.',
    'devices.serveUndo': 'To undo later, run:',
    'devices.serveConfirm': 'Run command',
    'devices.serveDone': 'This device is now reachable on your tailnet',
    'devices.noCerts': 'HTTPS certificates are off for your tailnet, so tailscale serve cannot be used. Turn them on in the Tailscale admin console (DNS → HTTPS Certificates), then refresh — or use the fallback below.',
    'devices.fallback': 'Fallback: listen on {ip}',
    'devices.fallbackTitle': 'Use plain HTTP on the Tailscale address?',
    'devices.fallbackBody': 'The sync port will also listen on {addr}. Traffic is plain HTTP, but it only travels inside Tailscale\'s encrypted WireGuard tunnel, and every request is still signed by the paired device. Nothing in your Tailscale settings changes.',
    'devices.fallbackConfirm': 'Listen on {addr}',
    'devices.fallbackDone': 'Listening on the Tailscale address',
    'devices.plainBadge': 'plain HTTP in WireGuard',
    'devices.myLink': 'My pairing link',
    'devices.myLinkIntro': 'On the other Mac open Devices → Add a device and paste this link (or scan the QR code). The code works once, for 10 minutes; the link holds no lasting secret.',
    'devices.code': 'Code',
    'devices.expiresIn': 'expires in {t}',
    'devices.expired': 'expired',
    'devices.regenerate': 'New code',
    'devices.copyLink': 'Copy link',
    'devices.linkNeedsReach': 'Make this device reachable first to get a pairing link.',
    'devices.add': 'Add a device',
    'devices.addIntro': 'Paste the cmsync:// link shown on the other device.',
    'devices.addPh': 'cmsync://…/pair?c=…&fp=…',
    'devices.pair': 'Pair',
    'devices.paired': 'Paired with {name}',
    'devices.list': 'Paired devices',
    'devices.none': 'No paired devices yet.',
    'devices.colName': 'Device',
    'devices.colAddress': 'Address',
    'devices.colLastSeen': 'Last seen',
    'devices.revoked': 'revoked',
    'devices.ping': 'Ping',
    'devices.pingOk': '{name} answered',
    'devices.revoke': 'Revoke',
    'devices.remove': 'Remove',
    'devices.revokeTitle': 'Revoke this device?',
    'devices.revokeBody': '"{name}" will be refused from its next request. To use it again, pair it again with a new link.',
    'devices.noAddress': 'no address',
  },
  vi: {
    'nav.devices': 'Thiết bị',
    'devices.intro': 'Ghép app này với Camoufox Manager trên máy Mac khác, qua Tailscale của bạn. Không đi qua máy chủ bên thứ ba nào.',
    'devices.status': 'Tailscale',
    'devices.state': 'Trạng thái',
    'devices.ts.not_installed': 'Chưa cài',
    'devices.ts.not_running': 'Chưa chạy',
    'devices.ts.needs_login': 'Cần đăng nhập',
    'devices.ts.stopped': 'Đang tắt',
    'devices.ts.starting': 'Đang khởi động…',
    'devices.ts.running': 'Đang chạy',
    'devices.ts.error': 'Lỗi',
    'devices.hint.not_installed': 'Cài Tailscale từ tailscale.com và đăng nhập trên cả hai máy. Đồng bộ thiết bị tắt cho tới lúc đó; phần còn lại của app vẫn chạy bình thường.',
    'devices.hint.not_running': 'Mở app Tailscale trên máy Mac này.',
    'devices.hint.needs_login': 'Đăng nhập Tailscale (hoặc nhờ admin duyệt máy này).',
    'devices.hint.stopped': 'Bật kết nối Tailscale từ biểu tượng trên thanh menu.',
    'devices.hint.starting': 'Chờ một chút rồi làm mới.',
    'devices.hint.error': 'Tailscale báo lỗi. Kiểm tra app Tailscale.',
    'devices.version': 'Phiên bản',
    'devices.thisDevice': 'Thiết bị này',
    'devices.fingerprint': 'Dấu vân tay khoá',
    'devices.magicDns': 'Tên trong tailnet',
    'devices.ips': 'IP Tailscale',
    'devices.https': 'Chứng chỉ HTTPS',
    'devices.on': 'bật',
    'devices.off': 'tắt',
    'devices.listeners': 'Cổng đồng bộ',
    'devices.reach': 'Cho thiết bị khác kết nối tới máy này',
    'devices.reachIntro': 'Thiết bị kia kết nối tới cổng đồng bộ của app qua Tailscale. Chỉ thiết bị trong tailnet của bạn mới tới được, không bao giờ mở ra internet.',
    'devices.reachNow': 'Địa chỉ kết nối',
    'devices.mode.serve': 'HTTPS qua tailscale serve',
    'devices.mode.tailnetIp': 'HTTP thường bên trong WireGuard (100.x)',
    'devices.mode.advertise': 'địa chỉ phát triển (CM_SYNC_ADVERTISE)',
    'devices.notReachable': 'Chưa kết nối được.',
    'devices.serveCmd': 'Lệnh sau sẽ chạy trên máy Mac này:',
    'devices.serveRun': 'Cho phép kết nối',
    'devices.serveTitle': 'Thay đổi cài đặt Tailscale?',
    'devices.serveBody': 'Camoufox Manager sẽ chạy lệnh bên dưới. Lệnh này mở cổng đồng bộ cho các thiết bị trong tailnet qua HTTPS (không ra internet) và giữ nguyên cho tới khi bạn gỡ.',
    'devices.serveUndo': 'Muốn gỡ sau này, chạy:',
    'devices.serveConfirm': 'Chạy lệnh',
    'devices.serveDone': 'Thiết bị này đã kết nối được trong tailnet',
    'devices.noCerts': 'Tailnet của bạn đang tắt chứng chỉ HTTPS nên không dùng được tailscale serve. Bật trong trang quản trị Tailscale (DNS → HTTPS Certificates) rồi làm mới — hoặc dùng cách dự phòng bên dưới.',
    'devices.fallback': 'Dự phòng: lắng nghe trên {ip}',
    'devices.fallbackTitle': 'Dùng HTTP thường trên địa chỉ Tailscale?',
    'devices.fallbackBody': 'Cổng đồng bộ sẽ lắng nghe thêm trên {addr}. Dữ liệu là HTTP thường nhưng chỉ đi bên trong đường hầm WireGuard đã mã hoá của Tailscale, và mọi yêu cầu vẫn được thiết bị đã ghép ký. Cài đặt Tailscale không thay đổi.',
    'devices.fallbackConfirm': 'Lắng nghe trên {addr}',
    'devices.fallbackDone': 'Đang lắng nghe trên địa chỉ Tailscale',
    'devices.plainBadge': 'HTTP thường trong WireGuard',
    'devices.myLink': 'Liên kết ghép của tôi',
    'devices.myLinkIntro': 'Trên máy Mac kia mở Thiết bị → Thêm thiết bị và dán liên kết này (hoặc quét mã QR). Mã chỉ dùng một lần, trong 10 phút; liên kết không chứa bí mật lâu dài nào.',
    'devices.code': 'Mã',
    'devices.expiresIn': 'hết hạn sau {t}',
    'devices.expired': 'đã hết hạn',
    'devices.regenerate': 'Mã mới',
    'devices.copyLink': 'Sao chép liên kết',
    'devices.linkNeedsReach': 'Hãy cho phép kết nối tới máy này trước để có liên kết ghép.',
    'devices.add': 'Thêm thiết bị',
    'devices.addIntro': 'Dán liên kết cmsync:// hiện trên thiết bị kia.',
    'devices.addPh': 'cmsync://…/pair?c=…&fp=…',
    'devices.pair': 'Ghép',
    'devices.paired': 'Đã ghép với {name}',
    'devices.list': 'Thiết bị đã ghép',
    'devices.none': 'Chưa ghép thiết bị nào.',
    'devices.colName': 'Thiết bị',
    'devices.colAddress': 'Địa chỉ',
    'devices.colLastSeen': 'Lần cuối thấy',
    'devices.revoked': 'đã thu hồi',
    'devices.ping': 'Ping',
    'devices.pingOk': '{name} đã trả lời',
    'devices.revoke': 'Thu hồi',
    'devices.remove': 'Xoá',
    'devices.revokeTitle': 'Thu hồi thiết bị này?',
    'devices.revokeBody': '"{name}" sẽ bị từ chối từ yêu cầu tiếp theo. Muốn dùng lại phải ghép lại bằng liên kết mới.',
    'devices.noAddress': 'không có địa chỉ',
  },
});

/* Sidebar icon (icons.js keeps ICONS in the shared global scope). */
ICONS.devices = ['M3 5h12v9H3z', 'M1 18h16', 'M18 8h4v11h-4z', 'M20 16h.01'];

{
  /* ---- QR code: byte mode, ECC M, versions 1–10 ---- */
  const ECC_M = [0, 10, 16, 26, 18, 24, 16, 18, 22, 22, 26]; // EC codewords per block
  const BLOCKS_M = [0, 1, 1, 1, 2, 2, 4, 4, 4, 5, 5]; // number of blocks

  const rawModules = v => {
    let r = (16 * v + 128) * v + 64;
    if (v >= 2) {
      const n = Math.floor(v / 7) + 2;
      r -= (25 * n - 10) * n - 55;
      if (v >= 7) r -= 36;
    }
    return r;
  };
  const dataCodewords = v => Math.floor(rawModules(v) / 8) - ECC_M[v] * BLOCKS_M[v];

  const gfMul = (x, y) => {
    let z = 0;
    for (let i = 7; i >= 0; i--) { z = (z << 1) ^ ((z >>> 7) * 0x11d); z ^= ((y >>> i) & 1) * x; }
    return z;
  };
  const rsDivisor = deg => {
    const r = new Array(deg - 1).fill(0); r.push(1);
    let root = 1;
    for (let i = 0; i < deg; i++) {
      for (let j = 0; j < r.length; j++) { r[j] = gfMul(r[j], root); if (j + 1 < r.length) r[j] ^= r[j + 1]; }
      root = gfMul(root, 2);
    }
    return r;
  };
  const rsRemainder = (data, div) => {
    const r = div.map(() => 0);
    for (const b of data) {
      const f = b ^ r.shift(); r.push(0);
      div.forEach((c, i) => { r[i] ^= gfMul(c, f); });
    }
    return r;
  };
  const alignPositions = (v, size) => {
    if (v === 1) return [];
    const n = Math.floor(v / 7) + 2;
    const step = Math.ceil((v * 4 + 4) / (n * 2 - 2)) * 2;
    const r = [6];
    for (let p = size - 7; r.length < n; p -= step) r.splice(1, 0, p);
    return r;
  };
  const bit = (x, i) => ((x >>> i) & 1) !== 0;

  function encode(text) {
    const bytes = Array.from(new TextEncoder().encode(text));
    let v = 1;
    while (v <= 10 && 4 + (v < 10 ? 8 : 16) + bytes.length * 8 > dataCodewords(v) * 8) v++;
    if (v > 10) throw new Error('text too long for QR');
    const cap = dataCodewords(v) * 8;
    const bits = [];
    const put = (val, n) => { for (let i = n - 1; i >= 0; i--) bits.push((val >>> i) & 1); };
    put(4, 4); put(bytes.length, v < 10 ? 8 : 16); bytes.forEach(b => put(b, 8));
    put(0, Math.min(4, cap - bits.length));
    put(0, (8 - bits.length % 8) % 8);
    for (let pad = 0xec; bits.length < cap; pad ^= 0xec ^ 0x11) put(pad, 8);
    const data = [];
    for (let i = 0; i < bits.length; i += 8) data.push(bits.slice(i, i + 8).reduce((a, b) => (a << 1) | b, 0));

    // Split into blocks, add Reed–Solomon codewords, interleave.
    const nb = BLOCKS_M[v], ecl = ECC_M[v], raw = Math.floor(rawModules(v) / 8);
    const nShort = nb - raw % nb, shortLen = Math.floor(raw / nb);
    const div = rsDivisor(ecl);
    const blocks = [];
    for (let i = 0, k = 0; i < nb; i++) {
      const d = data.slice(k, k + shortLen - ecl + (i < nShort ? 0 : 1));
      k += d.length;
      const ec = rsRemainder(d, div);
      if (i < nShort) d.push(0);
      blocks.push(d.concat(ec));
    }
    const all = [];
    for (let i = 0; i < blocks[0].length; i++) blocks.forEach((b, j) => { if (i !== shortLen - ecl || j >= nShort) all.push(b[i]); });

    // Matrix with function patterns.
    const size = v * 4 + 17;
    const m = Array.from({ length: size }, () => new Array(size).fill(false));
    const fn = Array.from({ length: size }, () => new Array(size).fill(false));
    const set = (x, y, dark) => { m[y][x] = dark; fn[y][x] = true; };
    for (let i = 0; i < size; i++) { set(6, i, i % 2 === 0); set(i, 6, i % 2 === 0); }
    for (const [cx, cy] of [[3, 3], [size - 4, 3], [3, size - 4]]) {
      for (let dy = -4; dy <= 4; dy++) for (let dx = -4; dx <= 4; dx++) {
        const d = Math.max(Math.abs(dx), Math.abs(dy)), x = cx + dx, y = cy + dy;
        if (x >= 0 && x < size && y >= 0 && y < size) set(x, y, d !== 2 && d !== 4);
      }
    }
    const al = alignPositions(v, size);
    al.forEach((ax, i) => al.forEach((ay, j) => {
      if ((i === 0 && j === 0) || (i === 0 && j === al.length - 1) || (i === al.length - 1 && j === 0)) return;
      for (let dy = -2; dy <= 2; dy++) for (let dx = -2; dx <= 2; dx++) set(ax + dx, ay + dy, Math.max(Math.abs(dx), Math.abs(dy)) !== 1);
    }));
    const drawFormat = mask => {
      const d = (0 << 3) | mask; // ECC level M = 00
      let r = d;
      for (let i = 0; i < 10; i++) r = (r << 1) ^ ((r >>> 9) * 0x537);
      const b = ((d << 10) | r) ^ 0x5412;
      for (let i = 0; i <= 5; i++) set(8, i, bit(b, i));
      set(8, 7, bit(b, 6)); set(8, 8, bit(b, 7)); set(7, 8, bit(b, 8));
      for (let i = 9; i < 15; i++) set(14 - i, 8, bit(b, i));
      for (let i = 0; i < 8; i++) set(size - 1 - i, 8, bit(b, i));
      for (let i = 8; i < 15; i++) set(8, size - 15 + i, bit(b, i));
      set(8, size - 8, true);
    };
    drawFormat(0); // reserve the format areas
    if (v >= 7) {
      let r = v;
      for (let i = 0; i < 12; i++) r = (r << 1) ^ ((r >>> 11) * 0x1f25);
      const b = (v << 12) | r;
      for (let i = 0; i < 18; i++) { const a = size - 11 + i % 3, c = Math.floor(i / 3); set(a, c, bit(b, i)); set(c, a, bit(b, i)); }
    }
    // Data in the zigzag order.
    let i = 0;
    for (let right = size - 1; right >= 1; right -= 2) {
      if (right === 6) right = 5;
      for (let vert = 0; vert < size; vert++) for (let j = 0; j < 2; j++) {
        const x = right - j, up = ((right + 1) & 2) === 0, y = up ? size - 1 - vert : vert;
        if (!fn[y][x] && i < all.length * 8) { m[y][x] = bit(all[i >>> 3], 7 - (i & 7)); i++; }
      }
    }
    // Pick the mask with the lowest (simplified) penalty.
    const MASKS = [
      (x, y) => (x + y) % 2 === 0, (x, y) => y % 2 === 0, x => x % 3 === 0, (x, y) => (x + y) % 3 === 0,
      (x, y) => (Math.floor(x / 3) + Math.floor(y / 2)) % 2 === 0, (x, y) => (x * y) % 2 + (x * y) % 3 === 0,
      (x, y) => ((x * y) % 2 + (x * y) % 3) % 2 === 0, (x, y) => ((x + y) % 2 + (x * y) % 3) % 2 === 0,
    ];
    const applyMask = k => { for (let y = 0; y < size; y++) for (let x = 0; x < size; x++) if (!fn[y][x] && MASKS[k](x, y)) m[y][x] = !m[y][x]; };
    const penalty = () => {
      let p = 0, dark = 0;
      for (let y = 0; y < size; y++) for (let x = 0; x < size; x++) {
        if (m[y][x]) dark++;
        if (x < size - 1 && y < size - 1 && m[y][x] === m[y][x + 1] && m[y][x] === m[y + 1][x] && m[y][x] === m[y + 1][x + 1]) p += 3;
      }
      for (let a = 0; a < size; a++) {
        let rx = 1, ry = 1;
        for (let b = 1; b < size; b++) {
          rx = m[a][b] === m[a][b - 1] ? rx + 1 : 1; if (rx === 5) p += 3; else if (rx > 5) p++;
          ry = m[b][a] === m[b - 1][a] ? ry + 1 : 1; if (ry === 5) p += 3; else if (ry > 5) p++;
        }
      }
      return p + Math.floor(Math.abs(dark * 20 - size * size * 10) / (size * size)) * 10;
    };
    let best = 0, bestP = Infinity;
    for (let k = 0; k < 8; k++) {
      applyMask(k); drawFormat(k);
      const p = penalty();
      if (p < bestP) { bestP = p; best = k; }
      applyMask(k); // undo (XOR)
    }
    applyMask(best); drawFormat(best);
    return { size, modules: m, version: v, mask: best };
  }

  /* SVG markup (black on white, 4-module quiet zone). */
  function svg(text, px = 200) {
    const q = encode(text), n = q.size + 8;
    let d = '';
    q.modules.forEach((row, y) => row.forEach((dark, x) => { if (dark) d += `M${x + 4} ${y + 4}h1v1h-1z`; }));
    return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${n} ${n}" width="${px}" height="${px}" shape-rendering="crispEdges" role="img"><rect width="${n}" height="${n}" fill="#fff"/><path d="${d}" fill="#000"/></svg>`;
  }

  CM.qr = { encode, svg };

  const mmss = s => Math.floor(s / 60) + ':' + String(s % 60).padStart(2, '0');

  CM.pages.devices = {
    setup() {
      const st = Vue.ref(null);
      const devices = Vue.ref([]);
      const offer = Vue.ref(null);
      const left = Vue.ref(0);
      const add = Vue.reactive({ url: '', busy: false });
      const serve = Vue.reactive({ open: false, busy: false });
      const fallback = Vue.reactive({ open: false, busy: false });
      const revoke = Vue.reactive({ open: false, dev: null, busy: false });
      const busy = Vue.reactive({ refresh: false, regen: false, ping: '' });
      let tick = null, offerAt = 0;

      const loadStatus = async fresh => {
        try { st.value = await api('/api/devices/status' + (fresh ? '?fresh=1' : '')); }
        catch (e) { toast(t('common.loadFailed', { msg: e.message }), true); }
      };
      const loadDevices = async () => {
        try { devices.value = (await api('/api/devices')).devices; } catch (e) { toast(t('common.loadFailed', { msg: e.message }), true); }
      };
      const setOffer = o => { offer.value = o; offerAt = Date.now(); left.value = o ? o.expiresIn : 0; };
      const loadOffer = async () => {
        if (!st.value || !st.value.reach) { setOffer(null); return; }
        try { setOffer(await api('/api/devices/pairing')); } catch (e) { setOffer(null); toast(e.message, true); }
      };
      const refresh = async () => {
        busy.refresh = true;
        await loadStatus(true);
        await Promise.all([loadOffer(), loadDevices()]);
        busy.refresh = false;
      };
      const regenerate = async () => {
        busy.regen = true;
        try { setOffer(await api('/api/devices/pairing/regenerate', { method: 'POST', body: '{}' })); } catch (e) { toast(e.message, true); }
        busy.regen = false;
      };
      const runServe = async () => {
        serve.busy = true;
        try {
          await api('/api/devices/serve', { method: 'POST', body: JSON.stringify({ command: st.value.servePlan.command, confirm: true }) });
          serve.open = false; toast(t('devices.serveDone')); await refresh();
        } catch (e) { toast(e.message, true); }
        serve.busy = false;
      };
      const runFallback = async () => {
        fallback.busy = true;
        try {
          await api('/api/devices/tailnet-ip', { method: 'POST', body: JSON.stringify({ confirm: true }) });
          fallback.open = false; toast(t('devices.fallbackDone')); await refresh();
        } catch (e) { toast(e.message, true); }
        fallback.busy = false;
      };
      const pair = async () => {
        add.busy = true;
        try {
          const d = await api('/api/devices/pair', { method: 'POST', body: JSON.stringify({ url: add.url.trim() }) });
          add.url = ''; toast(t('devices.paired', { name: d.name })); await loadDevices();
        } catch (e) { toast(e.message, true); }
        add.busy = false;
      };
      const ping = async d => {
        busy.ping = d.id;
        try { await api('/api/devices/' + d.id + '/hello', { method: 'POST', body: '{}' }); toast(t('devices.pingOk', { name: d.name })); }
        catch (e) { toast(e.message, true); }
        busy.ping = '';
        await loadDevices();
      };
      const doRevoke = async () => {
        revoke.busy = true;
        try { await api('/api/devices/' + revoke.dev.id, { method: 'DELETE' }); revoke.open = false; await loadDevices(); }
        catch (e) { toast(e.message, true); }
        revoke.busy = false;
      };
      const remove = async d => {
        try { await api('/api/devices/' + d.id + '?forget=1', { method: 'DELETE' }); await loadDevices(); } catch (e) { toast(e.message, true); }
      };
      const copy = async text => {
        try { await navigator.clipboard.writeText(text); toast(t('common.copied')); } catch (_) { /* selectable anyway */ }
      };

      Vue.onMounted(async () => {
        await loadStatus(false);
        await Promise.all([loadOffer(), loadDevices()]);
        tick = setInterval(() => {
          if (!offer.value) return;
          left.value = Math.max(0, offer.value.expiresIn - Math.floor((Date.now() - offerAt) / 1000));
          if (left.value === 0 && !busy.regen) { busy.regen = true; loadOffer().finally(() => { busy.regen = false; }); }
        }, 1000);
      });
      Vue.onUnmounted(() => { if (tick) clearInterval(tick); });

      const ts = Vue.computed(() => st.value && st.value.tailscale);
      const dot = Vue.computed(() => !ts.value ? '' : ts.value.state === 'running' ? 'bg-ok' : ts.value.state === 'not_installed' ? 'bg-faint' : 'bg-bad');
      const qr = Vue.computed(() => { try { return offer.value ? svg(offer.value.url, 196) : ''; } catch (_) { return ''; } });
      const short = fp => fp ? fp.slice(0, 16).replace(/(.{4})/g, '$1 ').trim() + '…' : '';

      return {
        st, ts, dot, devices, offer, left, qr, add, serve, fallback, revoke, busy,
        refresh, regenerate, runServe, runFallback, pair, ping, doRevoke, remove, copy, mmss, short, ago: fmt.ago,
      };
    },
    template: `<cm-page :title="t('nav.devices')">
    <template #actions>
      <button class="btn" :disabled="busy.refresh" @click="refresh"><cm-icon name="refresh" :size="14"/> {{ t('common.refresh') }}</button>
    </template>
    <div class="w-full max-w-[720px] mx-auto flex flex-col gap-6 py-2 overflow-auto">
      <p class="m-0 text-[13px] text-muted">{{ t('devices.intro') }}</p>

      <section>
        <h2 class="section-title">{{ t('devices.status') }}</h2>
        <div class="kv">
          <div class="kv-row"><span>{{ t('devices.state') }}</span>
            <div class="flex flex-col gap-1">
              <span class="flex items-center gap-2"><i class="dot" :class="dot"></i>{{ ts ? t('devices.ts.' + ts.state) : '…' }}</span>
              <span v-if="ts && ts.state !== 'running'" class="text-xs text-faint">{{ t('devices.hint.' + ts.state) }}<template v-if="ts.detail"> ({{ ts.detail }})</template></span>
            </div>
          </div>
          <div v-if="ts && ts.version" class="kv-row"><span>{{ t('devices.version') }}</span><code>{{ ts.version }}</code></div>
          <div v-if="ts && ts.dnsName" class="kv-row"><span>{{ t('devices.magicDns') }}</span><code class="break-all">{{ ts.dnsName }}</code></div>
          <div v-if="ts && ts.ips.length" class="kv-row"><span>{{ t('devices.ips') }}</span><code class="break-all">{{ ts.ips.join(', ') }}</code></div>
          <div v-if="ts && ts.state === 'running'" class="kv-row"><span>{{ t('devices.https') }}</span><span>{{ ts.httpsCerts ? t('devices.on') : t('devices.off') }}</span></div>
          <div v-if="st" class="kv-row"><span>{{ t('devices.thisDevice') }}</span>
            <div class="flex flex-col gap-1"><span>{{ st.me.name }}</span><code class="text-xs text-faint" :title="st.me.fingerprint">{{ t('devices.fingerprint') }}: {{ short(st.me.fingerprint) }}</code></div>
          </div>
          <div v-if="st" class="kv-row"><span>{{ t('devices.listeners') }}</span><code class="break-all">{{ st.listeners.join(', ') || '—' }}</code></div>
        </div>
      </section>

      <section v-if="st && st.enabled">
        <h2 class="section-title">{{ t('devices.reach') }}</h2>
        <p class="m-0 text-[13px] text-muted" style="margin-bottom:8px">{{ t('devices.reachIntro') }}</p>
        <div class="kv">
          <div class="kv-row"><span>{{ t('devices.reachNow') }}</span>
            <div v-if="st.reach" class="flex flex-col gap-1">
              <code class="break-all">{{ st.reach.endpoint }}</code>
              <span class="text-xs text-faint">{{ t('devices.mode.' + st.reach.mode) }} <span v-if="st.reach.plain" class="chip">{{ t('devices.plainBadge') }}</span></span>
            </div>
            <span v-else class="text-muted">{{ t('devices.notReachable') }}</span>
          </div>
          <div v-if="!st.reach || st.reach.mode === 'tailnetIp'" class="kv-row"><span></span>
            <div class="flex flex-col gap-2 min-w-0">
              <template v-if="st.serveAvailable">
                <span class="text-xs text-faint">{{ t('devices.serveCmd') }}</span>
                <code class="break-all p-2 rounded-lg border border-line bg-panel2 text-[12px]" style="user-select:all">{{ st.servePlan.command }}</code>
                <div><button class="btn btn-start" @click="serve.open = true">{{ t('devices.serveRun') }}</button></div>
              </template>
              <span v-else-if="ts && ts.state === 'running'" class="text-xs text-muted">{{ t('devices.noCerts') }}</span>
              <div v-if="!st.reach && st.tailnetIp"><button class="btn" @click="fallback.open = true">{{ t('devices.fallback', { ip: st.tailnetIp }) }}</button></div>
            </div>
          </div>
        </div>
      </section>

      <section v-if="st && st.enabled">
        <h2 class="section-title">{{ t('devices.myLink') }}</h2>
        <p class="m-0 text-[13px] text-muted" style="margin-bottom:8px">{{ offer ? t('devices.myLinkIntro') : t('devices.linkNeedsReach') }}</p>
        <div v-if="offer" class="kv">
          <div class="flex gap-4 flex-wrap items-start p-3">
            <div class="rounded-lg border border-line bg-white" style="line-height:0;padding:4px;background:#fff" v-html="qr"></div>
            <div class="flex flex-col gap-2 flex-1 min-w-0" style="min-width:220px">
              <code class="break-all p-2 rounded-lg border border-line bg-panel2 text-[12px]" style="user-select:all">{{ offer.url }}</code>
              <div class="flex items-center gap-2 flex-wrap">
                <span class="text-xs text-faint">{{ t('devices.code') }}</span><code>{{ offer.code }}</code>
              </div>
              <span class="text-xs" :class="left > 60 ? 'text-faint' : 'text-bad'">{{ left > 0 ? t('devices.expiresIn', { t: mmss(left) }) : t('devices.expired') }}</span>
              <div class="flex gap-2 flex-wrap">
                <button class="btn" @click="copy(offer.url)"><cm-icon name="copy" :size="14"/> {{ t('devices.copyLink') }}</button>
                <button class="btn" :disabled="busy.regen" @click="regenerate"><cm-icon name="refresh" :size="14"/> {{ t('devices.regenerate') }}</button>
              </div>
            </div>
          </div>
        </div>
      </section>

      <section v-if="st && st.enabled">
        <h2 class="section-title">{{ t('devices.add') }}</h2>
        <form class="kv" @submit.prevent="add.url.trim() && pair()">
          <div class="kv-row"><span>{{ t('devices.addIntro') }}</span>
            <div class="flex gap-2 flex-wrap">
              <input class="input flex-1 min-w-0" v-model="add.url" autocomplete="off" spellcheck="false" :placeholder="t('devices.addPh')">
              <button class="btn btn-start" type="submit" :disabled="add.busy || !add.url.trim()"><i v-if="add.busy" class="spin"></i>{{ t('devices.pair') }}</button>
            </div>
          </div>
        </form>
      </section>

      <section>
        <h2 class="section-title">{{ t('devices.list') }}</h2>
        <p v-if="!devices.length" class="m-0 text-[13px] text-muted">{{ t('devices.none') }}</p>
        <div v-else class="kv">
          <table class="list-tbl">
            <thead><tr><th>{{ t('devices.colName') }}</th><th>{{ t('devices.colAddress') }}</th><th>{{ t('devices.colLastSeen') }}</th><th></th></tr></thead>
            <tbody>
              <tr v-for="d in devices" :key="d.id" :class="d.revokedAt ? 'opacity-50' : ''">
                <td>{{ d.name }} <span v-if="d.revokedAt" class="chip">{{ t('devices.revoked') }}</span><br><code class="text-xs text-faint">{{ d.id }}</code></td>
                <td><code class="text-xs break-all">{{ d.url || t('devices.noAddress') }}</code></td>
                <td>{{ ago(d.lastSeenAt) }}<br><code v-if="d.lastSeenAddr" class="text-xs text-faint">{{ d.lastSeenAddr }}</code></td>
                <td>
                  <div class="flex gap-2 flex-wrap justify-end">
                    <button v-if="!d.revokedAt && d.url" class="btn" :disabled="busy.ping === d.id" @click="ping(d)">{{ t('devices.ping') }}</button>
                    <button v-if="!d.revokedAt" class="btn" @click="revoke.dev = d; revoke.open = true">{{ t('devices.revoke') }}</button>
                    <button v-else class="btn" @click="remove(d)">{{ t('devices.remove') }}</button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </div>

    <cm-dialog :open="serve.open" :title="t('devices.serveTitle')" :busy="serve.busy" :width="560" @close="serve.open = false">
      <p class="m-0 text-[13px] text-muted">{{ t('devices.serveBody') }}</p>
      <code v-if="st" class="break-all p-3 rounded-lg border border-line bg-panel2 text-[12.5px]">{{ st.servePlan.command }}</code>
      <p class="m-0 text-xs text-faint">{{ t('devices.serveUndo') }} <code v-if="st">{{ st.servePlan.undo }}</code></p>
      <template #footer>
        <button class="btn" :disabled="serve.busy" @click="serve.open = false">{{ t('common.cancel') }}</button>
        <button class="btn btn-start" :disabled="serve.busy" @click="runServe">{{ t('devices.serveConfirm') }}</button>
      </template>
    </cm-dialog>

    <cm-dialog :open="fallback.open" :title="t('devices.fallbackTitle')" :busy="fallback.busy" :width="520" @close="fallback.open = false">
      <p class="m-0 text-[13px] text-muted">{{ t('devices.fallbackBody', { addr: st && (st.tailnetIp + ':' + st.syncPort) }) }}</p>
      <template #footer>
        <button class="btn" :disabled="fallback.busy" @click="fallback.open = false">{{ t('common.cancel') }}</button>
        <button class="btn btn-start" :disabled="fallback.busy" @click="runFallback">{{ t('devices.fallbackConfirm', { addr: st && (st.tailnetIp + ':' + st.syncPort) }) }}</button>
      </template>
    </cm-dialog>

    <cm-dialog :open="revoke.open" :title="t('devices.revokeTitle')" :busy="revoke.busy" @close="revoke.open = false">
      <p class="m-0 text-[13px] text-muted">{{ t('devices.revokeBody', { name: revoke.dev && revoke.dev.name }) }}</p>
      <template #footer>
        <button class="btn" :disabled="revoke.busy" @click="revoke.open = false">{{ t('common.cancel') }}</button>
        <button class="btn btn-stop" :disabled="revoke.busy" @click="doRevoke">{{ t('devices.revoke') }}</button>
      </template>
    </cm-dialog>
  </cm-page>`,
  };
}
