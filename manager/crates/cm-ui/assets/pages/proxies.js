/* Proxies: the `useProxies(ctx)` composable used by the profile table (list, check, assign) and the
   Proxy library page (add / edit / delete, import a list, check, assign to profiles, assign a list in
   order). Passwords never reach the UI (the API only says hasPassword; the field is write-only). */

I18N.add({
  en: {
    'proxies.none': 'No proxy (machine IP)',
    'proxies.alive': 'Proxy OK: {ip} · {country} · {tz}',
    'proxies.failed': 'Proxy failed: {msg}',
    'proxies.assigned': 'Proxy assigned — applies at next Start',
    'proxies.cleared': 'Proxy removed — uses the machine IP',
    'proxies.intro': 'Proxies are stored on this machine and checked only when you ask. Stored passwords stay hidden.',
    'proxies.search': 'Search name, server or country',
    'proxies.col.name': 'Name',
    'proxies.col.connection': 'Connection',
    'proxies.col.credentials': 'Credentials',
    'proxies.col.lastCheck': 'Last check',
    'proxies.col.relay': 'Local port',
    'proxies.col.profiles': 'Profiles',
    'proxies.noUser': 'No username',
    'proxies.passwordStored': 'Password stored',
    'proxies.noPassword': 'No password',
    'proxies.notChecked': 'Not checked',
    'proxies.check': 'Check',
    'proxies.empty': 'No proxies yet.',
    'proxies.add': 'Add proxy',
    'proxies.edit': 'Edit',
    'proxies.editTitle': 'Edit proxy',
    'proxies.addTitle': 'Add proxy',
    'proxies.newOption': '+ New proxy…',
    'proxies.newLine': 'New proxy',
    'proxies.newLineHint': 'host:port, host:port:user:pass or scheme://user:pass@host:port. It is saved to the Proxy library.',
    'proxies.newDuplicate': 'This proxy is already in the library; choose it from the list.',
    'proxies.import': 'Import list',
    'proxies.importTitle': 'Import a list',
    'proxies.importHint': 'One proxy per line (up to 100): host:port, scheme://user:pass@host:port, socks5://[::1]:1080 or host:port:user:pass. Percent-encode special characters in credentials. The whole list is checked before anything is saved.',
    'proxies.importDo': 'Import',
    'proxies.imported': 'Imported {n} proxies, skipped {s} duplicates',
    'proxies.importRejected': 'Nothing was imported — fix these lines:',
    'proxies.line': 'Line {n}',
    'proxies.f.name': 'Name',
    'proxies.f.nameHint': 'Optional — defaults to host:port',
    'proxies.f.scheme': 'Protocol',
    'proxies.f.schemeHint': 'How the manager talks to the proxy, not to the websites. Most providers: HTTP or SOCKS5.',
    'proxies.f.host': 'Server',
    'proxies.f.hostHint': 'Only host or [IPv6] here — full URLs belong in Import',
    'proxies.f.port': 'Port',
    'proxies.f.user': 'Username',
    'proxies.f.pass': 'Password',
    'proxies.f.passKeep': 'Unchanged (stored)',
    'proxies.f.passClear': 'Remove the stored password',
    'proxies.f.rotateUrl': 'Change-IP link',
    'proxies.f.rotateUrlHint': 'Optional. http(s) link of a rotating provider; opened directly (not through the proxy). It may contain a token — keep it private.',
    'proxies.f.rotateOnStart': 'Change IP before every start',
    'proxies.rotate': 'Change IP',
    'proxies.rotated': 'IP changed: {ip} · {country} · {tz}',
    'proxies.rotatedNoGeo': 'IP change requested, but the re-check failed: {msg}',
    'proxies.rotateFailed': 'Change IP failed: {msg}',
    'proxies.saved': 'Proxy saved',
    'proxies.deleteTitle': 'Delete proxy?',
    'proxies.deleteBody': '{name} will be removed and unassigned from the profiles that use it.',
    'proxies.deleted': 'Proxy deleted',
    'proxies.assign': 'Assign',
    'proxies.assignTitle': 'Assign {name} to profiles',
    'proxies.assignHint': 'Closed profiles only. The proxy applies at the next Start.',
    'proxies.assignDo': 'Assign to {n}',
    'proxies.assignDone': 'Assigned to {n} profiles',
    'proxies.assignSkipped': '{n} skipped: {why}',
    'proxies.noProfiles': 'No profiles yet.',
    'proxies.running': 'running',
    'proxies.selectAll': 'Select all closed',
    'proxies.seq': 'Assign proxy list',
    'proxies.seqTitle': 'Assign a proxy list in order',
    'proxies.seqHint': 'Click profiles and proxies in the order you want: the first profile gets the first proxy, and so on. No proxies selected = the whole library in order. Proxies whose exit IP or server another profile already uses are skipped.',
    'proxies.seqProfiles': 'Profiles',
    'proxies.seqProxies': 'Proxies',
    'proxies.seqPreview': 'Preview mapping',
    'proxies.seqApply': 'Apply',
    'proxies.seqNeed': 'Select at least one profile',
    'proxies.seqSkippedIp': '{n} proxies skipped (IP already in use)',
    'proxies.seqDone': 'Assigned {n} profiles',
    'proxies.seqClear': 'Clear selection',
    'proxies.why.not-found': 'not found',
    'proxies.why.running': 'running — stop it first',
    'proxies.why.no-proxy': 'no proxy left',
    'proxies.why.error': 'error',
    'proxies.err.bad-scheme': 'Unknown protocol (use http, https, socks4, socks4a, socks5 or socks5h)',
    'proxies.err.bad-port': 'Invalid port',
    'proxies.err.bad-host': 'Invalid server',
    'proxies.err.bad-format': 'Not host:port',
    'proxies.err.bad-credentials': 'Invalid credentials (percent-encoding, or SOCKS5 needs both, 1–255 bytes)',
    'proxies.err.creds-not-allowed': 'SOCKS4 does not take credentials',
    'proxies.err.has-path': 'Paths, queries and fragments are not accepted',
    'proxies.err.bad-name': 'Name too long',
  },
  vi: {
    'proxies.none': 'Không proxy (IP máy)',
    'proxies.alive': 'Proxy sống: {ip} · {country} · {tz}',
    'proxies.failed': 'Proxy lỗi: {msg}',
    'proxies.assigned': 'Đã gán proxy — áp dụng khi Start',
    'proxies.cleared': 'Đã bỏ proxy riêng — dùng IP máy',
    'proxies.intro': 'Proxy lưu ở máy này và chỉ được kiểm tra khi bạn yêu cầu. Mật khẩu đã lưu luôn được ẩn.',
    'proxies.search': 'Tìm theo tên, server, quốc gia',
    'proxies.col.name': 'Tên',
    'proxies.col.connection': 'Kết nối',
    'proxies.col.credentials': 'Đăng nhập',
    'proxies.col.lastCheck': 'Kiểm tra gần nhất',
    'proxies.col.relay': 'Cổng local',
    'proxies.col.profiles': 'Profile',
    'proxies.noUser': 'Không username',
    'proxies.passwordStored': 'Đã lưu mật khẩu',
    'proxies.noPassword': 'Không mật khẩu',
    'proxies.notChecked': 'Chưa kiểm tra',
    'proxies.check': 'Kiểm tra',
    'proxies.empty': 'Chưa có proxy nào.',
    'proxies.add': 'Thêm proxy',
    'proxies.edit': 'Sửa',
    'proxies.editTitle': 'Sửa proxy',
    'proxies.addTitle': 'Thêm proxy',
    'proxies.newOption': '+ Proxy mới…',
    'proxies.newLine': 'Proxy mới',
    'proxies.newLineHint': 'host:port, host:port:user:pass hoặc scheme://user:pass@host:port. Proxy được lưu vào thư viện Proxy.',
    'proxies.newDuplicate': 'Proxy này đã có trong thư viện, hãy chọn nó trong danh sách.',
    'proxies.import': 'Nhập danh sách',
    'proxies.importTitle': 'Nhập danh sách proxy',
    'proxies.importHint': 'Mỗi dòng một proxy (tối đa 100): host:port, scheme://user:pass@host:port, socks5://[::1]:1080 hoặc host:port:user:pass. Ký tự đặc biệt trong tài khoản cần mã hoá phần trăm. Cả danh sách được kiểm tra trước khi lưu.',
    'proxies.importDo': 'Nhập',
    'proxies.imported': 'Đã nhập {n} proxy, bỏ qua {s} dòng trùng',
    'proxies.importRejected': 'Chưa nhập gì — hãy sửa các dòng sau:',
    'proxies.line': 'Dòng {n}',
    'proxies.f.name': 'Tên',
    'proxies.f.nameHint': 'Tuỳ chọn — mặc định là host:port',
    'proxies.f.scheme': 'Giao thức',
    'proxies.f.schemeHint': 'Cách ứng dụng nói chuyện với proxy, không phải với website. Đa số nhà cung cấp: HTTP hoặc SOCKS5.',
    'proxies.f.host': 'Server',
    'proxies.f.hostHint': 'Chỉ host hoặc [IPv6] — URL đầy đủ dùng ở phần Nhập danh sách',
    'proxies.f.port': 'Cổng',
    'proxies.f.user': 'Username',
    'proxies.f.pass': 'Mật khẩu',
    'proxies.f.passKeep': 'Giữ nguyên (đã lưu)',
    'proxies.f.passClear': 'Xoá mật khẩu đã lưu',
    'proxies.f.rotateUrl': 'Link đổi IP',
    'proxies.f.rotateUrlHint': 'Tuỳ chọn. Link http(s) của nhà cung cấp proxy xoay; được gọi trực tiếp (không qua proxy). Link có thể chứa token — hãy giữ kín.',
    'proxies.f.rotateOnStart': 'Đổi IP trước mỗi lần Start',
    'proxies.rotate': 'Đổi IP',
    'proxies.rotated': 'Đã đổi IP: {ip} · {country} · {tz}',
    'proxies.rotatedNoGeo': 'Đã gọi đổi IP nhưng kiểm tra lại lỗi: {msg}',
    'proxies.rotateFailed': 'Đổi IP lỗi: {msg}',
    'proxies.saved': 'Đã lưu proxy',
    'proxies.deleteTitle': 'Xoá proxy?',
    'proxies.deleteBody': '{name} sẽ bị xoá và gỡ khỏi các profile đang dùng.',
    'proxies.deleted': 'Đã xoá proxy',
    'proxies.assign': 'Gán',
    'proxies.assignTitle': 'Gán {name} cho profile',
    'proxies.assignHint': 'Chỉ profile đang đóng. Proxy áp dụng ở lần Start sau.',
    'proxies.assignDo': 'Gán cho {n}',
    'proxies.assignDone': 'Đã gán cho {n} profile',
    'proxies.assignSkipped': 'Bỏ qua {n}: {why}',
    'proxies.noProfiles': 'Chưa có profile nào.',
    'proxies.running': 'đang chạy',
    'proxies.selectAll': 'Chọn tất cả profile đang đóng',
    'proxies.seq': 'Gán danh sách proxy',
    'proxies.seqTitle': 'Gán danh sách proxy theo thứ tự',
    'proxies.seqHint': 'Bấm chọn profile và proxy theo thứ tự mong muốn: profile đầu nhận proxy đầu, và cứ thế. Không chọn proxy = cả thư viện theo thứ tự. Proxy có IP ra hoặc server mà profile khác đã dùng sẽ bị bỏ qua.',
    'proxies.seqProfiles': 'Profile',
    'proxies.seqProxies': 'Proxy',
    'proxies.seqPreview': 'Xem trước',
    'proxies.seqApply': 'Áp dụng',
    'proxies.seqNeed': 'Chọn ít nhất một profile',
    'proxies.seqSkippedIp': 'Bỏ qua {n} proxy (IP đã được dùng)',
    'proxies.seqDone': 'Đã gán cho {n} profile',
    'proxies.seqClear': 'Bỏ chọn',
    'proxies.why.not-found': 'không tìm thấy',
    'proxies.why.running': 'đang chạy — hãy dừng trước',
    'proxies.why.no-proxy': 'hết proxy',
    'proxies.why.error': 'lỗi',
    'proxies.err.bad-scheme': 'Giao thức lạ (dùng http, https, socks4, socks4a, socks5 hoặc socks5h)',
    'proxies.err.bad-port': 'Cổng không hợp lệ',
    'proxies.err.bad-host': 'Server không hợp lệ',
    'proxies.err.bad-format': 'Không phải host:port',
    'proxies.err.bad-credentials': 'Tài khoản không hợp lệ (mã hoá phần trăm, hoặc SOCKS5 cần cả hai, 1–255 byte)',
    'proxies.err.creds-not-allowed': 'SOCKS4 không dùng tài khoản',
    'proxies.err.has-path': 'Không nhận đường dẫn, query hay fragment',
    'proxies.err.bad-name': 'Tên quá dài',
  },
});

/* One pasted proxy line -> saved library proxy (via /api/proxies/import, same formats as Import list).
   Returns the proxy id; throws a translated message for a bad line. An exact duplicate already in the
   library is reused when it can be matched by host and port. */
async function proxyFromLine(line) {
  const text = line.trim();
  if (!text) throw new Error(t('proxies.err.bad-format'));
  if (text.includes('\n')) throw new Error(t('proxies.err.bad-format'));
  const r = await fetch('/api/proxies/import', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ text }) });
  const j = await r.json().catch(() => ({}));
  if (j.rejected && j.rejected.length) throw new Error(t('proxies.err.' + j.rejected[0].code));
  if (!r.ok) throw new Error(j.error || r.statusText);
  if (j.created.length) return j.created[0].id;
  const all = (await api('/api/proxies')).proxies;
  const hit = all.filter(x => text.includes(x.host) && text.includes(':' + x.serverPort));
  if (hit.length === 1) return hit[0].id;
  throw new Error(t('proxies.newDuplicate'));
}

/* ctx = { load } — reload callback of the page that uses it. */
function useProxies(ctx) {
  const proxies = Vue.ref([]), checking = Vue.ref(null);

  async function fetchProxies() {
    proxies.value = (await api('/api/proxies')).proxies;
  }

  async function check(id) {
    checking.value = id;
    try {
      const g = await api(`/api/proxies/${id}/check`, { method: 'POST' });
      toast(t('proxies.alive', { ip: g.ip, country: g.country, tz: g.timezone }));
    } catch (err) { toast(t('proxies.failed', { msg: err.message }), true); }
    checking.value = null; ctx.load && ctx.load();
  }

  async function rotate(id) {
    checking.value = id;
    try {
      const r = await api(`/api/proxies/${id}/rotate`, { method: 'POST' });
      if (!r.ok) toast(t('proxies.rotateFailed', { msg: r.error }), true);
      else if (r.geo) toast(t('proxies.rotated', { ip: r.geo.ip, country: r.geo.country, tz: r.geo.timezone }));
      else toast(t('proxies.rotatedNoGeo', { msg: r.error }), true);
    } catch (err) { toast(t('proxies.rotateFailed', { msg: err.message }), true); }
    checking.value = null; ctx.load && ctx.load();
  }

  async function assign(p, e) {
    const proxyId = e.target.value === '' ? null : Number(e.target.value);
    e.target.blur();
    try {
      await api(`/api/profiles/${p.id}/assign`, { method: 'POST', body: JSON.stringify({ proxyId }) });
      toast(t(proxyId === null ? 'proxies.cleared' : 'proxies.assigned'));
      if (proxyId !== null) await api(`/api/proxies/${proxyId}/check`, { method: 'POST' }).catch(err => toast(t('proxies.failed', { msg: err.message }), true));
    } catch (err) { toast(err.message, true); }
    ctx.load && ctx.load();
  }

  return { proxies, checking, fetchProxies, check, rotate, assign, noProxy: () => t('proxies.none') };
}

CM.pages.proxies = {
  setup() {
    const loaded = Vue.ref(false), q = Vue.ref(''), profiles = Vue.ref([]);
    const px = useProxies({ load: () => load() });
    async function load() {
      try {
        const [ov] = await Promise.all([api('/api/overview'), px.fetchProxies()]);
        profiles.value = ov.profiles.map(p => ({ id: p.id, name: p.name, running: !!p.running }));
      } catch (e) { toast(t('common.loadFailed', { msg: e.message }), true); }
      loaded.value = true;
    }
    Vue.onMounted(load);

    const columns = Vue.computed(() => [
      { key: 'name', label: t('proxies.col.name') },
      { key: 'connection', label: t('proxies.col.connection'), cls: 'font-mono text-[12.5px]' },
      { key: 'credentials', label: t('proxies.col.credentials') },
      { key: 'lastCheck', label: t('proxies.col.lastCheck') },
      { key: 'usedBy', label: t('proxies.col.profiles') },
      { key: 'actions', label: '', cls: 'text-right whitespace-nowrap', hcls: 'text-right' },
    ]);
    const rows = Vue.computed(() => {
      const n = q.value.toLowerCase();
      return px.proxies.value.filter(p => !n || [p.name, p.label, p.geo?.country, p.geo?.countryCode].some(v => (v || '').toLowerCase().includes(n)));
    });
    const schemes = ['http', 'https', 'socks4', 'socks4a', 'socks5', 'socks5h'];
    const why = r => t('proxies.why.' + r);

    /* ---- add / edit ---- */
    const form = Vue.reactive({ open: false, id: null, busy: false, name: '', scheme: 'http', host: '', port: '', username: '', password: '', hasPassword: false, clearPassword: false, rotateUrl: '', rotateOnStart: false });
    function openForm(p) {
      Object.assign(form, p
        ? { id: p.id, name: p.name, scheme: p.scheme, host: p.host, port: String(p.serverPort), username: p.username || '', password: '', hasPassword: p.hasPassword, rotateUrl: p.rotateUrl || '', rotateOnStart: !!p.rotateOnStart }
        : { id: null, name: '', scheme: 'http', host: '', port: '', username: '', password: '', hasPassword: false, rotateUrl: '', rotateOnStart: false },
        { open: true, clearPassword: false });
    }
    async function saveForm() {
      form.busy = true;
      try {
        const body = { name: form.name, scheme: form.scheme, host: form.host, port: Number(form.port), username: form.username || null, rotateUrl: form.rotateUrl.trim() || null, rotateOnStart: form.rotateOnStart && !!form.rotateUrl.trim() };
        if (form.password) body.password = form.password;
        if (form.clearPassword) body.clearPassword = true;
        await api(form.id ? `/api/proxies/${form.id}` : '/api/proxies', { method: form.id ? 'PUT' : 'POST', body: JSON.stringify(body) });
        form.open = false; form.password = '';
        toast(t('proxies.saved')); load();
      } catch (e) { toast(e.message, true); }
      form.busy = false;
    }

    /* ---- delete ---- */
    const del = Vue.reactive({ open: false, busy: false, p: null });
    async function doDelete() {
      del.busy = true;
      try { await api(`/api/proxies/${del.p.id}`, { method: 'DELETE' }); del.open = false; toast(t('proxies.deleted')); load(); }
      catch (e) { toast(e.message, true); }
      del.busy = false;
    }

    /* ---- import ---- */
    const imp = Vue.reactive({ open: false, busy: false, text: '', rejected: [] });
    async function doImport() {
      imp.busy = true; imp.rejected = [];
      try {
        const r = await fetch('/api/proxies/import', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ text: imp.text }) });
        const j = await r.json().catch(() => ({}));
        if (j.rejected && j.rejected.length) imp.rejected = j.rejected;
        else if (!r.ok) throw new Error(j.error || r.statusText);
        else { imp.open = false; imp.text = ''; toast(t('proxies.imported', { n: j.created.length, s: j.skipped.length })); load(); }
      } catch (e) { toast(e.message, true); }
      imp.busy = false;
    }

    /* ---- assign one proxy to many profiles ---- */
    const asg = Vue.reactive({ open: false, busy: false, p: null, sel: [] });
    function openAssign(p) { Object.assign(asg, { open: true, p, sel: [] }); }
    const closed = Vue.computed(() => profiles.value.filter(p => !p.running));
    function toggleAll() { asg.sel = asg.sel.length === closed.value.length ? [] : closed.value.map(p => p.id); }
    async function doAssign() {
      asg.busy = true;
      try {
        const r = (await api(`/api/proxies/${asg.p.id}/assign`, { method: 'POST', body: JSON.stringify({ profileIds: asg.sel }) })).results;
        const bad = r.filter(x => !x.ok);
        toast(t('proxies.assignDone', { n: r.length - bad.length }) + (bad.length ? '\n' + t('proxies.assignSkipped', { n: bad.length, why: why(bad[0].reason) }) : ''), bad.length > 0);
        asg.open = false; load();
      } catch (e) { toast(e.message, true); }
      asg.busy = false;
    }

    /* ---- assign a list in order ---- */
    const seq = Vue.reactive({ open: false, busy: false, profiles: [], proxies: [], result: null });
    function openSeq() { Object.assign(seq, { open: true, profiles: [], proxies: [], result: null }); }
    function pick(list, id) {
      const i = list.indexOf(id);
      if (i >= 0) list.splice(i, 1); else list.push(id);
      seq.result = null;
    }
    const nameOf = id => profiles.value.find(p => p.id === id)?.name || id;
    const proxyName = id => px.proxies.value.find(p => p.id === id)?.name || '#' + id;
    async function runSeq(dryRun) {
      seq.busy = true;
      try {
        const body = { profileIds: seq.profiles, proxyIds: seq.proxies.length ? seq.proxies : null, dryRun };
        const r = await api('/api/proxies/assign-sequence', { method: 'POST', body: JSON.stringify(body) });
        if (dryRun) seq.result = r;
        else { toast(t('proxies.seqDone', { n: r.pairs.filter(p => p.applied).length })); seq.open = false; load(); }
      } catch (e) { toast(e.message, true); }
      seq.busy = false;
    }

    return { ...px, loaded, q, columns, rows, fmt, schemes, why, profiles, closed,
      form, openForm, saveForm, del, doDelete, imp, doImport, asg, openAssign, toggleAll, doAssign,
      seq, openSeq, pick, nameOf, proxyName, runSeq };
  },
  template: `<cm-page :title="t('nav.proxies')" :count="proxies.length">
    <template #tools>
      <div class="search"><cm-icon name="search" :size="14"/><input v-model="q" :placeholder="t('proxies.search')" autocomplete="off"></div>
    </template>
    <template #actions>
      <button class="btn" @click="openSeq()" :disabled="!proxies.length || !profiles.length">{{ t('proxies.seq') }}</button>
      <button class="btn" @click="imp.open = true">{{ t('proxies.import') }}</button>
      <button class="btn btn-start" @click="openForm()"><cm-icon name="plus" :size="14"/> {{ t('proxies.add') }}</button>
    </template>
    <p class="m-0 mb-3 text-[13px] text-muted">{{ t('proxies.intro') }}</p>
    <div class="min-h-0 flex flex-col bg-panel border border-line rounded-xl shadow-card">
      <cm-table :columns="columns" :rows="rows" :loading="!loaded" :empty="t('proxies.empty')">
        <template #cell-name="{ row }"><div class="font-semibold">{{ row.name }}</div><div class="text-[11.5px] text-faint">{{ row.geo?.countryCode || '' }}</div></template>
        <template #cell-connection="{ row }">{{ row.scheme }}://{{ row.label }}</template>
        <template #cell-credentials="{ row }">
          <div>{{ row.username || t('proxies.noUser') }}</div>
          <div class="text-[11.5px] text-faint">{{ row.hasPassword ? t('proxies.passwordStored') : t('proxies.noPassword') }}</div>
        </template>
        <template #cell-lastCheck="{ row }">
          <template v-if="row.geo"><div class="text-ok">{{ fmt.flag(row.geo.countryCode) }} {{ row.geo.ip }}<span v-if="row.geo.latencyMs" class="text-faint"> · {{ row.geo.latencyMs }} ms</span></div>
            <div class="text-[11.5px] text-faint">{{ row.geo.city }} · {{ row.geo.timezone }} · {{ fmt.ago(row.geo.checkedAt) }}</div></template>
          <span v-else class="text-muted">{{ t('proxies.notChecked') }}</span>
        </template>
        <template #cell-usedBy="{ row }">{{ row.usedBy ?? 0 }}</template>
        <template #cell-actions="{ row }">
          <div class="inline-flex gap-1.5">
            <button class="btn" :disabled="checking === row.id" @click="check(row.id)">
              <span v-if="checking === row.id" class="spin"></span><cm-icon v-else name="check" :size="14"/> {{ t('proxies.check') }}
            </button>
            <button v-if="row.rotateUrl" class="btn" :disabled="checking === row.id" @click="rotate(row.id)">{{ t('proxies.rotate') }}</button>
            <button class="btn" @click="openAssign(row)">{{ t('proxies.assign') }}</button>
            <button class="btn" @click="openForm(row)">{{ t('proxies.edit') }}</button>
            <button class="btn" @click="del.p = row; del.open = true">{{ t('common.delete') }}</button>
          </div>
        </template>
      </cm-table>
    </div>

    <cm-dialog :open="form.open" :title="form.id ? t('proxies.editTitle') : t('proxies.addTitle')" :busy="form.busy" @close="form.open = false">
      <cm-field :label="t('proxies.f.name')" :hint="t('proxies.f.nameHint')"><input class="input" v-model="form.name" autocomplete="off"></cm-field>
      <cm-field :label="t('proxies.f.scheme')" :hint="t('proxies.f.schemeHint')">
        <select class="input" v-model="form.scheme"><option v-for="s in schemes" :key="s" :value="s">{{ s.toUpperCase() }}</option></select>
      </cm-field>
      <div class="grid grid-cols-[1fr_96px] gap-2">
        <cm-field :label="t('proxies.f.host')" :hint="t('proxies.f.hostHint')"><input class="input" v-model="form.host" autocomplete="off" spellcheck="false"></cm-field>
        <cm-field :label="t('proxies.f.port')"><input class="input" v-model="form.port" inputmode="numeric" autocomplete="off"></cm-field>
      </div>
      <cm-field :label="t('proxies.f.user')"><input class="input" v-model="form.username" autocomplete="off" spellcheck="false"></cm-field>
      <cm-field :label="t('proxies.f.pass')">
        <input class="input" type="password" v-model="form.password" autocomplete="new-password" :placeholder="form.hasPassword ? t('proxies.f.passKeep') : ''" :disabled="form.clearPassword">
      </cm-field>
      <cm-switch v-if="form.id && form.hasPassword" v-model="form.clearPassword" :label="t('proxies.f.passClear')"/>
      <cm-field :label="t('proxies.f.rotateUrl')" :hint="t('proxies.f.rotateUrlHint')"><input class="input font-mono text-[12.5px]" v-model="form.rotateUrl" autocomplete="off" spellcheck="false" placeholder="https://"></cm-field>
      <cm-switch v-if="form.rotateUrl.trim()" v-model="form.rotateOnStart" :label="t('proxies.f.rotateOnStart')"/>
      <template #footer>
        <button class="btn" :disabled="form.busy" @click="form.open = false">{{ t('common.cancel') }}</button>
        <button class="btn btn-start" :disabled="form.busy || !form.host || !form.port" @click="saveForm">{{ t('common.save') }}</button>
      </template>
    </cm-dialog>

    <cm-dialog :open="del.open" :title="t('proxies.deleteTitle')" :busy="del.busy" @close="del.open = false">
      <p class="m-0 text-[13px] text-muted">{{ t('proxies.deleteBody', { name: del.p?.name }) }}</p>
      <template #footer>
        <button class="btn" :disabled="del.busy" @click="del.open = false">{{ t('common.cancel') }}</button>
        <button class="btn btn-stop" :disabled="del.busy" @click="doDelete">{{ t('common.delete') }}</button>
      </template>
    </cm-dialog>

    <cm-dialog :open="imp.open" :title="t('proxies.importTitle')" :busy="imp.busy" :width="560" @close="imp.open = false">
      <p class="m-0 text-[12.5px] text-muted">{{ t('proxies.importHint') }}</p>
      <textarea class="input font-mono text-[12.5px] min-h-[150px]" v-model="imp.text" spellcheck="false" autocomplete="off" placeholder="proxy.example:8080&#10;socks5://user:password@proxy.example:1080"></textarea>
      <div v-if="imp.rejected.length" class="text-[12.5px] text-bad">
        <div class="font-semibold">{{ t('proxies.importRejected') }}</div>
        <div v-for="r in imp.rejected" :key="r.line">{{ t('proxies.line', { n: r.line }) }}: {{ t('proxies.err.' + r.code) }}</div>
      </div>
      <template #footer>
        <button class="btn" :disabled="imp.busy" @click="imp.open = false">{{ t('common.cancel') }}</button>
        <button class="btn btn-start" :disabled="imp.busy || !imp.text.trim()" @click="doImport">{{ t('proxies.importDo') }}</button>
      </template>
    </cm-dialog>

    <cm-dialog :open="asg.open" :title="t('proxies.assignTitle', { name: asg.p?.name })" :busy="asg.busy" :width="440" @close="asg.open = false">
      <p class="m-0 text-[12.5px] text-muted">{{ t('proxies.assignHint') }}</p>
      <div v-if="!profiles.length" class="text-muted text-[13px]">{{ t('proxies.noProfiles') }}</div>
      <template v-else>
        <button class="btn self-start" @click="toggleAll">{{ t('proxies.selectAll') }}</button>
        <div class="max-h-[260px] overflow-auto border border-line rounded-lg">
          <label v-for="p in profiles" :key="p.id" class="flex items-center gap-2 px-3 py-1.5 text-[13px] border-b border-line last:border-0" :class="p.running ? 'opacity-50' : 'cursor-pointer'">
            <input type="checkbox" :value="p.id" v-model="asg.sel" :disabled="p.running">
            <span class="flex-1">{{ p.name }}</span><span v-if="p.running" class="text-faint text-[11.5px]">{{ t('proxies.running') }}</span>
          </label>
        </div>
      </template>
      <template #footer>
        <button class="btn" :disabled="asg.busy" @click="asg.open = false">{{ t('common.cancel') }}</button>
        <button class="btn btn-start" :disabled="asg.busy || !asg.sel.length" @click="doAssign">{{ t('proxies.assignDo', { n: asg.sel.length }) }}</button>
      </template>
    </cm-dialog>

    <cm-dialog :open="seq.open" :title="t('proxies.seqTitle')" :busy="seq.busy" :width="720" @close="seq.open = false">
      <p class="m-0 text-[12.5px] text-muted">{{ t('proxies.seqHint') }}</p>
      <div class="grid grid-cols-2 gap-3 max-sm:grid-cols-1">
        <div>
          <div class="text-xs text-muted mb-1">{{ t('proxies.seqProfiles') }} ({{ seq.profiles.length }})</div>
          <div class="max-h-[220px] overflow-auto border border-line rounded-lg">
            <button v-for="p in closed" :key="p.id" type="button" @click="pick(seq.profiles, p.id)" class="w-full flex items-center gap-2 px-3 py-1.5 text-[13px] text-left bg-transparent border-0 border-b border-line last:border-0 cursor-pointer hover:bg-panel2" :class="seq.profiles.includes(p.id) ? 'font-semibold' : ''">
              <span class="inline-block min-w-[22px] text-center rounded-full text-[11px]" :class="seq.profiles.includes(p.id) ? 'bg-accent text-white' : 'text-faint'">{{ seq.profiles.includes(p.id) ? seq.profiles.indexOf(p.id) + 1 : '·' }}</span>
              {{ p.name }}
            </button>
          </div>
        </div>
        <div>
          <div class="text-xs text-muted mb-1">{{ t('proxies.seqProxies') }} ({{ seq.proxies.length || proxies.length }})</div>
          <div class="max-h-[220px] overflow-auto border border-line rounded-lg">
            <button v-for="p in proxies" :key="p.id" type="button" @click="pick(seq.proxies, p.id)" class="w-full flex items-center gap-2 px-3 py-1.5 text-[13px] text-left bg-transparent border-0 border-b border-line last:border-0 cursor-pointer hover:bg-panel2" :class="seq.proxies.includes(p.id) ? 'font-semibold' : ''">
              <span class="inline-block min-w-[22px] text-center rounded-full text-[11px]" :class="seq.proxies.includes(p.id) ? 'bg-accent text-white' : 'text-faint'">{{ seq.proxies.includes(p.id) ? seq.proxies.indexOf(p.id) + 1 : '·' }}</span>
              <span class="flex-1 truncate">{{ p.name }}</span><span class="text-faint text-[11.5px]">{{ p.geo?.ip || '' }}</span>
            </button>
          </div>
        </div>
      </div>
      <div v-if="seq.result" class="text-[12.5px] border border-line rounded-lg p-2.5 flex flex-col gap-1">
        <div v-for="p in seq.result.pairs" :key="p.profileId">
          {{ nameOf(p.profileId) }} → <b v-if="p.proxyId">{{ proxyName(p.proxyId) }}</b><span v-else class="text-bad">{{ why(p.reason) }}</span>
        </div>
        <div v-if="seq.result.skippedProxies.length" class="text-faint">{{ t('proxies.seqSkippedIp', { n: seq.result.skippedProxies.length }) }}</div>
      </div>
      <template #footer>
        <button class="btn" :disabled="seq.busy" @click="seq.open = false">{{ t('common.cancel') }}</button>
        <button class="btn" :disabled="seq.busy || !seq.profiles.length" :title="seq.profiles.length ? '' : t('proxies.seqNeed')" @click="runSeq(true)">{{ t('proxies.seqPreview') }}</button>
        <button class="btn btn-start" :disabled="seq.busy || !seq.result" @click="runSeq(false)">{{ t('proxies.seqApply') }}</button>
      </template>
    </cm-dialog>
  </cm-page>`,
};
