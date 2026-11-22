/* Profiles page: spreadsheet-style table (D8) — click a cell to select, drag or ⇧ to extend,
   ⌘C copies the selection as TSV (pastes into Excel/Sheets), "Copy table" copies everything.
   Start/stop, create/delete, assign proxy per row. Polls /api/overview every 5 s without losing the
   selection (kept by profile id), the scroll position, or an open proxy dropdown.
   Phase 4 adds the profile form (components/profile-form.js); phase 7 the bulk toolbar. */

I18N.add({
  en: {
    'profiles.search': 'Search name, ID, IP, country…',
    'profiles.filter.all': 'All',
    'profiles.filter.running': 'Running',
    'profiles.filter.own': 'Own proxy',
    'profiles.filter.none': 'No proxy',
    'profiles.hint': 'Click a cell to select · drag / {shift} for a range · {cmdc} pastes into Excel',
    'profiles.copyTable': 'Copy table',
    'profiles.copyTableTitle': 'Copy the whole visible table (with headers)',
    'profiles.refreshTitle': 'Reload the profile list',
    'profiles.new': 'New profile',
    'profiles.newTitle': 'Create a profile with its own fingerprint',
    'profiles.running': '{n} running',
    'profiles.proxyCount': 'Proxies: {n}',
    'profiles.engineError': 'Could not read profiles, showing the last data.',
    'profiles.engineNotReadyTitle': '{name} not ready',
    'profiles.engineNotReadyBody': 'creating and starting profiles is disabled.',
    'profiles.engineInstall': 'Install {name}',
    'profiles.engineInstalling': 'Installing {name}…',
    'profiles.engineViewLog': 'View log',
    'profiles.engineDetails': 'Details',
    'profiles.noMatch': 'No profile matches',
    'profiles.selectRow': 'Select the whole row',
    'profiles.stopFirst': 'Stop the profile before changing its proxy',
    'profiles.chooseProxy': 'Choose a proxy for this profile',
    'profiles.own': 'own',
    'profiles.no': 'none',
    'profiles.failClosed': 'fail-closed: if the proxy dies the browser is blocked, the real IP never leaks',
    'profiles.checkProxy': 'Check proxy',
    'profiles.reveal': 'Show in Finder',
    'profiles.start': 'Start',
    'profiles.stop': 'Stop',
    'profiles.delete': 'Delete profile',
    'profiles.edit': 'Edit profile settings',
    'profiles.started': 'Browser opened',
    'profiles.startedVia': 'Browser opened via {ip}',
    'profiles.stopped': 'Stopped',
    'profiles.engineErrorToast': 'Engine error, keeping the old data',
    'profiles.copiedRange': 'Copied the selection (pastes into Excel)',
    'profiles.copiedRows': 'Copied {n} rows with headers',
    'profiles.createTitle': 'New {name} profile',
    'profiles.name': 'Name',
    'profiles.namePlaceholder': 'e.g. Shop 01',
    'profiles.os': 'Fingerprint OS',
    'profiles.fpOnce': 'The fingerprint is generated once and stays the same for this profile.',
    'profiles.creating': 'Creating…',
    'profiles.created': 'Created profile {name}',
    'profiles.deleteTitle': 'Delete profile “{name}”?',
    'profiles.deleteBody': 'Its fingerprint and all browser data (cookies, logins, history) are removed from this machine and cannot be recovered.',
    'profiles.deleteBodyTrash': 'It moves to the Trash and is deleted for good after {days} days. Until then you can restore it with all its data.',
    'profiles.trash': 'Trash',
    'profiles.trashTitle': 'Deleted profiles that can still be restored',
    'profiles.trashCount': 'Trash ({n})',
    'profiles.trashEmpty': 'The trash is empty.',
    'profiles.trashInfo': 'Deleted profiles stay here for {days} days, then are removed for good.',
    'profiles.trashOff': 'The trash is off: deleted profiles are removed at once.',
    'profiles.trashDays': 'Keep for (days, 0 = off)',
    'profiles.trashDeleted': 'deleted {ago}',
    'profiles.trashLeft': '{n} d left',
    'profiles.trashLeftSoon': 'less than a day left',
    'profiles.restore': 'Restore',
    'profiles.restored': 'Restored {name}',
    'profiles.purge': 'Delete forever',
    'profiles.purgeTitle': 'Delete “{name}” forever?',
    'profiles.purgeBody': 'Its fingerprint and all browser data are removed from this machine and cannot be recovered.',
    'profiles.purged': 'Deleted {name} forever',
    'profiles.emptyTrash': 'Empty trash',
    'profiles.emptyTrashTitle': 'Empty the trash?',
    'profiles.emptyTrashBody': 'All {n} profiles in the trash are deleted forever, with their data. This cannot be undone.',
    'profiles.emptied': 'Trash emptied',
    'profiles.deleteRunning': 'The running profile is stopped first.',
    'profiles.deleting': 'Deleting…',
    'profiles.deleted': 'Deleted {name}',
    'profiles.status.running': 'running',
    'profiles.status.crashed': 'crashed',
    'profiles.lease': 'leased by {holder}',
    'profiles.status.stopped': 'stopped',
    'profiles.activity.running': 'running {ago}',
    'profiles.byIp': 'by IP',
    'profiles.notYet': 'not yet',
    'profiles.group.profile': 'Profile',
    'profiles.group.proxy': 'Proxy',
    'profiles.group.region': 'Region',
    'profiles.group.fingerprint': 'Fingerprint',
    'profiles.group.data': 'Local data',
    'profiles.col.name': 'Name',
    'profiles.col.status': 'Status',
    'profiles.col.id': 'Profile ID',
    'profiles.col.activity': 'Activity',
    'profiles.col.os': 'OS',
    'profiles.col.proxy': 'Proxy',
    'profiles.col.type': 'Type',
    'profiles.col.exitIp': 'Exit IP',
    'profiles.col.localPort': 'Local port',
    'profiles.col.country': 'Country',
    'profiles.col.timezone': 'Timezone',
    'profiles.col.language': 'Language',
    'profiles.col.browser': 'Browser',
    'profiles.col.resolution': 'Resolution',
    'profiles.col.path': 'Path',
    'profiles.col.size': 'Size',
  },
  vi: {
    'profiles.search': 'Tìm theo tên, ID, IP, quốc gia…',
    'profiles.filter.all': 'Tất cả',
    'profiles.filter.running': 'Đang chạy',
    'profiles.filter.own': 'Proxy riêng',
    'profiles.filter.none': 'Không proxy',
    'profiles.hint': 'Bấm ô để chọn · kéo / {shift} để chọn vùng · {cmdc} dán được vào Excel',
    'profiles.copyTable': 'Sao chép bảng',
    'profiles.copyTableTitle': 'Sao chép toàn bộ bảng đang hiện (kèm tiêu đề)',
    'profiles.refreshTitle': 'Đọc lại danh sách profile',
    'profiles.new': 'Tạo profile',
    'profiles.newTitle': 'Tạo profile mới với fingerprint riêng',
    'profiles.running': '{n} đang chạy',
    'profiles.proxyCount': '{n} proxy',
    'profiles.engineError': 'Không đọc được danh sách profile, đang hiện dữ liệu gần nhất.',
    'profiles.engineNotReadyTitle': '{name} chưa sẵn sàng',
    'profiles.engineNotReadyBody': 'tạo/mở profile đang tắt.',
    'profiles.engineInstall': 'Cài {name}',
    'profiles.engineInstalling': 'Đang cài {name}…',
    'profiles.engineViewLog': 'Xem nhật ký',
    'profiles.engineDetails': 'Chi tiết',
    'profiles.noMatch': 'Không có profile nào khớp',
    'profiles.selectRow': 'Chọn cả hàng',
    'profiles.stopFirst': 'Stop trước khi đổi proxy',
    'profiles.chooseProxy': 'Chọn proxy cho profile',
    'profiles.own': 'riêng',
    'profiles.no': 'không',
    'profiles.failClosed': 'fail-closed: proxy chết thì chặn, không lộ IP thật',
    'profiles.checkProxy': 'Kiểm tra proxy',
    'profiles.reveal': 'Mở trong Finder',
    'profiles.start': 'Start',
    'profiles.stop': 'Stop',
    'profiles.delete': 'Xoá profile',
    'profiles.edit': 'Sửa cài đặt profile',
    'profiles.started': 'Đã mở browser',
    'profiles.startedVia': 'Đã mở browser qua {ip}',
    'profiles.stopped': 'Đã dừng',
    'profiles.engineErrorToast': 'Engine lỗi, giữ dữ liệu cũ',
    'profiles.copiedRange': 'Đã sao chép vùng chọn (dán được vào Excel)',
    'profiles.copiedRows': 'Đã sao chép {n} dòng kèm tiêu đề',
    'profiles.createTitle': 'Tạo profile {name}',
    'profiles.name': 'Tên',
    'profiles.namePlaceholder': 'Ví dụ: Shop 01',
    'profiles.os': 'Hệ điều hành của fingerprint',
    'profiles.fpOnce': 'Fingerprint được sinh một lần và giữ nguyên cho profile này.',
    'profiles.creating': 'Đang tạo…',
    'profiles.created': 'Đã tạo profile {name}',
    'profiles.deleteTitle': 'Xoá profile “{name}”?',
    'profiles.deleteBodyTrash': 'Profile được chuyển vào Thùng rác và xoá hẳn sau {days} ngày. Trong thời gian đó bạn có thể khôi phục cùng toàn bộ dữ liệu.',
    'profiles.trash': 'Thùng rác',
    'profiles.trashTitle': 'Các profile đã xoá vẫn còn khôi phục được',
    'profiles.trashCount': 'Thùng rác ({n})',
    'profiles.trashEmpty': 'Thùng rác trống.',
    'profiles.trashInfo': 'Profile đã xoá được giữ ở đây {days} ngày rồi xoá hẳn.',
    'profiles.trashOff': 'Thùng rác đang tắt: profile bị xoá ngay khi xoá.',
    'profiles.trashDays': 'Giữ trong (ngày, 0 = tắt)',
    'profiles.trashDeleted': 'xoá {ago}',
    'profiles.trashLeft': 'còn {n} ngày',
    'profiles.trashLeftSoon': 'còn chưa đến 1 ngày',
    'profiles.restore': 'Khôi phục',
    'profiles.restored': 'Đã khôi phục {name}',
    'profiles.purge': 'Xoá vĩnh viễn',
    'profiles.purgeTitle': 'Xoá vĩnh viễn “{name}”?',
    'profiles.purgeBody': 'Fingerprint và toàn bộ dữ liệu browser của profile sẽ bị xoá khỏi máy, không khôi phục được.',
    'profiles.purged': 'Đã xoá vĩnh viễn {name}',
    'profiles.emptyTrash': 'Dọn thùng rác',
    'profiles.emptyTrashTitle': 'Dọn sạch thùng rác?',
    'profiles.emptyTrashBody': 'Cả {n} profile trong thùng rác sẽ bị xoá vĩnh viễn cùng dữ liệu, không hoàn tác được.',
    'profiles.emptied': 'Đã dọn thùng rác',
    'profiles.deleteBody': 'Fingerprint và toàn bộ dữ liệu browser (cookie, đăng nhập, lịch sử) của profile sẽ bị xoá khỏi máy, không khôi phục được.',
    'profiles.deleteRunning': 'Profile đang chạy sẽ được dừng trước.',
    'profiles.deleting': 'Đang xoá…',
    'profiles.deleted': 'Đã xoá {name}',
    'profiles.status.running': 'đang chạy',
    'profiles.status.crashed': 'bị lỗi',
    'profiles.lease': 'agent {holder} đang giữ',
    'profiles.status.stopped': 'dừng',
    'profiles.activity.running': 'chạy {ago}',
    'profiles.byIp': 'theo IP',
    'profiles.notYet': 'chưa có',
    'profiles.group.profile': 'Profile',
    'profiles.group.proxy': 'Proxy',
    'profiles.group.region': 'Vùng',
    'profiles.group.fingerprint': 'Fingerprint',
    'profiles.group.data': 'Dữ liệu ở máy',
    'profiles.col.name': 'Tên',
    'profiles.col.status': 'Trạng thái',
    'profiles.col.id': 'Profile ID',
    'profiles.col.activity': 'Hoạt động',
    'profiles.col.os': 'OS',
    'profiles.col.proxy': 'Proxy',
    'profiles.col.type': 'Loại',
    'profiles.col.exitIp': 'IP ra',
    'profiles.col.localPort': 'Local port',
    'profiles.col.country': 'Quốc gia',
    'profiles.col.timezone': 'Timezone',
    'profiles.col.language': 'Ngôn ngữ',
    'profiles.col.browser': 'Browser',
    'profiles.col.resolution': 'Độ phân giải',
    'profiles.col.path': 'Đường dẫn',
    'profiles.col.size': 'Dung lượng',
  },
});

{ // file-local scope: classic scripts share one global scope

/* ---------- formatting ---------- */
const osName = os => ({ mac: 'macOS', win: 'Windows', lin: 'Linux', android: 'Android' }[os] || os || '—');
const browserOf = ua => {
  const m = /Chrome\/(\d+)/.exec(ua || '') || /Firefox\/(\d+)/.exec(ua || '');
  return m ? (m[0].startsWith('Chrome') ? 'Chrome ' : 'Firefox ') + m[1] : '—';
};
const shortGpu = r => (r || '').replace(/^ANGLE \(/, '').replace(/, Unspecified Version\)$/, '').replace(/ANGLE Metal Renderer: /, '').replace(/\)$/, '');
const langOf = l => (l || '').split(',')[0] || '—';
const yn = v => v === undefined || v === null ? '—' : v ? '✓' : '✗';
const tsvCell = v => /[\t\n"]/.test(v) ? '"' + v.replace(/"/g, '""') + '"' : v;
const proxyLabel = x => '#' + x.id + ' ' + (x.name && x.name !== x.label ? x.name + ' · ' : '') + x.label;

/* ---------- columns ----------
   g: group label key (marks the first column of a group) · h: header key (t()) or literal (raw: true)
   w: width px · cls: cell class · v: raw value (search / copy / title) · k: special rendering (template) */
const tzOf = p => p.timezone || p.assigned?.geo?.timezone || (p.timezoneByIp ? t('profiles.byIp') : '—');
const storageCol = key => ({ h: key, raw: true, k: 'yn', get: p => p.fp?.storage?.[key], v: p => yn(p.fp?.storage?.[key]) });

const ALL = [
  { h: 'profiles.col.name', w: 170, k: 'name', v: p => p.name },
  { g: 'profiles.group.profile', h: 'profiles.col.status', w: 112, k: 'status', v: p => (p.crashed && !p.running ? t('profiles.status.crashed') : t(p.running ? 'profiles.status.running' : 'profiles.status.stopped')) + (p.lease ? ' · ' + t('profiles.lease', { holder: p.lease.holder }) : '') },
  { h: 'org.col.tags', w: 170, k: 'tags', v: p => CM.orgText.tags(p) },
  { h: 'org.col.group', w: 120, k: 'group', v: p => CM.orgText.group(p) },
  { h: 'profiles.col.id', w: 210, cls: 'mono', v: p => p.id },
  { h: 'profiles.col.activity', w: 112, s: p => p.running ? Date.now() : Date.parse(p.lastActivity) || 0, v: p => p.running ? t('profiles.activity.running', { ago: fmt.ago(p.running.startedAt) }) : fmt.ago(p.lastActivity) },
  { h: 'profiles.col.os', w: 80, v: p => osName(p.os) },
  { g: 'profiles.group.proxy', h: 'profiles.col.proxy', w: 250, k: 'proxy', v: p => p.assigned ? proxyLabel(p.assigned) : t('proxies.none') },
  { h: 'profiles.col.type', w: 84, k: 'type', v: p => t(p.assigned ? 'profiles.own' : 'profiles.no') },
  { h: 'profiles.col.exitIp', w: 140, cls: 'mono', v: p => p.assigned ? (p.assigned.geo?.ip || '?') : '—' },
  { h: 'profiles.col.localPort', w: 140, cls: 'mono', v: p => p.assigned?.port ? '127.0.0.1:' + p.assigned.port : '—' },
  { h: '', w: 34, k: 'check', v: () => '' },
  { g: 'profiles.group.region', h: 'profiles.col.country', w: 140, k: 'geo', v: p => p.assigned?.geo ? p.assigned.geo.country : '—' },
  { h: 'profiles.col.timezone', w: 150, cls: 'mono', v: tzOf },
  { h: 'profiles.col.language', w: 80, v: p => p.settings?.locale || langOf(p.fp?.language) },
  { g: 'profiles.group.fingerprint', h: 'profiles.col.browser', w: 100, v: p => browserOf(p.fp?.userAgent) },
  { h: 'profiles.col.resolution', w: 104, cls: 'num', v: p => p.fp?.resolution || '—' },
  { h: 'DPR', raw: true, w: 52, cls: 'num', v: p => p.fp?.dpr ?? '—' },
  { h: 'CPU', raw: true, w: 52, cls: 'num', v: p => p.fp?.cpu ?? '—' },
  { h: 'RAM', raw: true, w: 64, cls: 'num', v: p => p.fp ? (p.fp.ram ?? '—') + ' GB' : '—' },
  { h: 'GPU', raw: true, w: 190, v: p => p.fp ? shortGpu(p.fp.webglRenderer) || p.fp.webglVendor || '—' : '—' },
  { h: 'Canvas', raw: true, w: 70, v: p => p.fp?.noise?.canvas || '—' },
  { h: 'WebGL', raw: true, w: 70, v: p => p.fp?.noise?.webgl || '—' },
  { h: 'Audio', raw: true, w: 70, v: p => p.fp?.noise?.audio || '—' },
  { h: 'Rects', raw: true, w: 70, v: p => p.fp?.noise?.rects || '—' },
  { h: 'WebRTC', raw: true, w: 84, v: p => p.fp?.webrtc || p.settings?.webrtc || '—' },
  { h: 'Fonts', raw: true, w: 60, cls: 'num', v: p => p.fp?.fonts ?? '—' },
  { h: 'User-Agent', raw: true, w: 260, cls: 'mono', v: p => p.fp?.userAgent || '—' },
  { g: 'profiles.group.data', h: 'profiles.col.path', w: 300, cls: 'mono', v: p => p.path.dir },
  { h: '', w: 34, k: 'reveal', v: () => '' },
  { h: 'profiles.col.size', w: 92, cls: 'num', s: p => p.path.sizeBytes, v: p => p.path.exists ? fmt.size(p.path.sizeBytes) : t('profiles.notYet') },
  { w: 66, ...storageCol('session') },
  { w: 56, ...storageCol('local') },
  { w: 64, ...storageCol('history') },
  { w: 84, ...storageCol('passwords') },
  { h: 'ws', raw: true, hf: o => (o?.wsProtocol || 'Playwright') + ' (ws)', w: 280, cls: 'mono', v: p => p.running?.ws || '—' },
];
const LAST = ALL.length - 1;
const OSES = [{ k: 'win', t: 'Windows' }, { k: 'mac', t: 'macOS' }, { k: 'lin', t: 'Linux' }];
const FILTERS = ['all', 'running', 'own', 'none'];

// Group header row: each group spans until the next group (the Name column has none).
const GROUPS = [];
ALL.forEach((c, i) => {
  if (!c.g) return;
  let n = 1; while (ALL[i + n] && !ALL[i + n].g) n++;
  GROUPS.push({ g: c.g, n });
});

CM.pages.profiles = {
  setup() {
    const { reactive, ref, computed, nextTick, onMounted, onUnmounted } = Vue;
    const o = ref(null);
    const busy = reactive({});
    const q = ref(''), filter = ref('all'), refreshing = ref(false), sheet = ref(null);
    // selection: a = anchor cell, f = focus cell, each {id, c}; by id so it survives data refreshes
    const sel = reactive({ a: null, f: null, drag: false });
    // organization (components/org.js): tags/group/OS filters, sort, checkbox selection, saved views, bulk actions
    const manage = ref(false);
    const orgx = CM.useOrg({ q, filter, rows: () => view.value, profiles: () => o.value?.profiles });

    /* ---------- data ---------- */
    let px;
    async function load(forceRefresh) {
      // A proxy dropdown is open: do not re-render under it (it would close).
      if (!forceRefresh && document.activeElement?.tagName === 'SELECT') return;
      try {
        const [ov] = await Promise.all([api('/api/overview'), px.fetchProxies(), orgx.loadOrg()]);
        o.value = ov;
        if (forceRefresh) ov.apiError ? toast(t('profiles.engineErrorToast'), true) : toast(t('common.updated'));
      } catch (e) { toast(t('common.loadFailed', { msg: e.message }), true); }
    }
    px = useProxies({ load });

    const pn = computed(() => o.value?.provider || 'Camoufox');
    const runCount = computed(() => o.value ? o.value.profiles.filter(p => p.running).length : 0);
    const raw = (c, p) => String(c.v(p));
    const head = c => c.hf ? c.hf(o.value) : c.raw ? c.h : (c.h ? t(c.h) : '');
    const hint = computed(() => fmt.engineHint(o.value));
    const hintDetail = computed(() => fmt.engineDetail(o.value));
    // Settings starts the install on arrival (#/settings/install); while one runs, just open its log.
    const goInstall = () => { location.hash = '#/settings/install'; };
    const goLog = () => { location.hash = '#/settings'; };

    const view = computed(() => {
      if (!o.value) return [];
      const needle = q.value.toLowerCase();
      return orgx.sortRows(orgx.filterRows(o.value.profiles), ALL).filter(p => {
        if (filter.value === 'running' && !p.running) return false;
        if (filter.value === 'own' && !p.assigned) return false;
        if (filter.value === 'none' && p.assigned) return false;
        return !needle || ALL.some(c => raw(c, p).toLowerCase().includes(needle));
      });
    });

    /* ---------- cell selection & copy, spreadsheet style ---------- */
    const rowOf = id => view.value.findIndex(p => p.id === id);
    const rect = computed(() => {
      if (!sel.a || !sel.f) return null;
      const r0 = rowOf(sel.a.id), r1 = rowOf(sel.f.id);
      if (r0 < 0 || r1 < 0) return null;
      return { r0: Math.min(r0, r1), r1: Math.max(r0, r1), c0: Math.min(sel.a.c, sel.f.c), c1: Math.max(sel.a.c, sel.f.c) };
    });
    const cur = computed(() => sel.f ? { r: rowOf(sel.f.id), c: sel.f.c } : null);
    const cellState = (r, c) => {
      const R = rect.value;
      return { sel: !!R && r >= R.r0 && r <= R.r1 && c >= R.c0 && c <= R.c1, cur: cur.value?.r === r && cur.value?.c === c };
    };

    function setSel(r, c, extend) {
      const p = view.value[r]; if (!p) return;
      c = Math.max(0, Math.min(LAST, c));
      sel.f = { id: p.id, c };
      if (!extend || !sel.a) sel.a = { ...sel.f };
      nextTick(() => sheet.value?.querySelector(`tbody tr:nth-child(${r + 1}) td:nth-child(${c + 2})`)?.scrollIntoView({ block: 'nearest', inline: 'nearest' }));
    }
    const selectRow = r => {
      const id = view.value[r].id;
      sel.a = { id, c: 0 }; sel.f = { id, c: LAST }; sheet.value.focus();
    };
    const down = (e, r, c) => {
      if (e.target.closest('select,button')) return;
      e.preventDefault(); sheet.value.focus();
      setSel(r, c, e.shiftKey); sel.drag = true;
    };
    const over = (r, c) => { if (sel.drag) setSel(r, c, true); };
    const up = () => { sel.drag = false; };

    function tsv(rows, c0, c1, withHead) {
      const cols = ALL.slice(c0, c1 + 1).filter(c => c.h); // skip button columns
      const lines = rows.map(p => cols.map(c => tsvCell(raw(c, p))).join('\t'));
      if (withHead) lines.unshift(cols.map(head).join('\t'));
      return lines.join('\n');
    }
    function selectedText() {
      const R = rect.value; if (!R) return null;
      if (R.r0 === R.r1 && R.c0 === R.c1) return raw(ALL[R.c0], view.value[R.r0]);
      return tsv(view.value.slice(R.r0, R.r1 + 1), R.c0, R.c1, false);
    }
    function flash() {
      sheet.value.querySelectorAll('td.sel').forEach(td => { td.classList.remove('flash'); void td.offsetWidth; td.classList.add('flash'); });
    }
    const dbl = e => {
      if (e.target.closest('select,button')) return;
      const s = selectedText();
      if (s != null) { navigator.clipboard.writeText(s); flash(); toast(t('common.copied')); }
    };
    function onKey(e) {
      if (e.target !== sheet.value || !sel.f) return;
      const r = rowOf(sel.f.id), c = sel.f.c;
      const mv = { ArrowUp: [-1, 0], ArrowDown: [1, 0], ArrowLeft: [0, -1], ArrowRight: [0, 1], Tab: [0, e.shiftKey ? -1 : 1] }[e.key];
      if (mv) { e.preventDefault(); setSel(Math.max(0, Math.min(view.value.length - 1, r + mv[0])), c + mv[1], e.shiftKey && e.key !== 'Tab'); return; }
      if ((e.metaKey || e.ctrlKey) && e.key === 'a') {
        e.preventDefault();
        if (view.value.length) { sel.a = { id: view.value[0].id, c: 0 }; sel.f = { id: view.value.at(-1).id, c: LAST }; }
        return;
      }
      if (e.key === 'Escape') sel.a = sel.f = null;
    }
    // ⌘C or Edit → Copy: copy the selection as TSV
    function onCopy(e) {
      if (document.activeElement !== sheet.value) return;
      const s = selectedText(); if (s == null) return;
      e.clipboardData.setData('text/plain', s); e.preventDefault(); flash();
      toast(s.includes('\t') || s.includes('\n') ? t('profiles.copiedRange') : t('common.copied'));
    }
    const copyAll = () => {
      navigator.clipboard.writeText(tsv(view.value, 0, LAST, true));
      toast(t('profiles.copiedRows', { n: view.value.length }));
    };

    /* ---------- actions ---------- */
    async function refresh() {
      refreshing.value = true;
      await load(true);
      refreshing.value = false;
    }
    async function act(name, id) {
      busy[id] = true;
      try {
        const r = await api(`/api/profiles/${id}/${name}`, { method: 'POST' });
        toast(name === 'start' ? (r.geo?.ip ? t('profiles.startedVia', { ip: r.geo.ip }) : t('profiles.started')) : t('profiles.stopped'));
      } catch (e) { toast(e.message, true); }
      busy[id] = false; load();
    }
    const reveal = p => api(`/api/profiles/${p.id}/reveal`, { method: 'POST' }).catch(err => toast(err.message, true));

    const dlg = reactive({ open: false, name: '', os: 'win', busy: false });
    const del = reactive({ p: null, busy: false });
    // CM-3.1 trash: list + retention, dialog, confirm (kind 'one' with p, or 'all')
    const trash = reactive({ open: false, days: 7, items: [], busy: false, confirm: null });
    async function loadTrash() {
      try { const r = await api('/api/profiles/trash'); trash.days = r.days; trash.items = r.profiles; } catch (e) { /* keep the last list */ }
    }
    const openDel = p => { del.p = p; loadTrash(); };
    const daysLeft = it => trash.days - Math.floor((Date.now() - Date.parse(it.deletedAt)) / 86400000);
    const leftText = it => { const n = daysLeft(it); return n <= 0 ? t('profiles.trashLeftSoon') : t('profiles.trashLeft', { n }); };
    async function trashAct(fn) {
      trash.busy = true;
      try { await fn(); } catch (e) { toast(e.message, true); }
      trash.busy = false; trash.confirm = null; await loadTrash(); load();
    }
    const restore = it => trashAct(async () => {
      await api(`/api/profiles/${it.id}/restore`, { method: 'POST' });
      toast(t('profiles.restored', { name: it.name }));
    });
    const purge = it => trashAct(async () => {
      await api(`/api/profiles/${it.id}/purge`, { method: 'DELETE' });
      toast(t('profiles.purged', { name: it.name }));
    });
    const emptyTrash = () => trashAct(async () => {
      await api('/api/profiles/trash/empty', { method: 'POST' });
      toast(t('profiles.emptied'));
    });
    async function saveDays(e) {
      const n = Math.max(0, Math.min(365, parseInt(e.target.value, 10) || 0));
      try { await api('/api/settings', { method: 'PUT', body: JSON.stringify({ trashDays: n }) }); trash.days = n; } catch (err) { toast(err.message, true); }
      e.target.value = trash.days;
    }
    const edit = reactive({ id: null });
    const editing = computed(() => (edit.id && o.value?.profiles.find(p => p.id === edit.id)) || null);
    const cookies = reactive({ id: null }); // cookies dialog (components/cookies.js)
    const cookiesFor = computed(() => (cookies.id && o.value?.profiles.find(p => p.id === cookies.id)) || null);
    const nameInput = ref(null);
    const openCreate = () => { dlg.open = true; dlg.name = ''; dlg.os = 'win'; nextTick(() => nameInput.value?.focus()); };
    async function create() {
      if (!dlg.name.trim() || dlg.busy) return;
      dlg.busy = true;
      try {
        const p = await api('/api/profiles', { method: 'POST', body: JSON.stringify({ name: dlg.name.trim(), os: dlg.os }) });
        toast(t('profiles.created', { name: p.name }));
        dlg.open = false;
      } catch (e) { toast(e.message, true); }
      dlg.busy = false; load();
    }
    async function remove() {
      const p = del.p; if (!p) return;
      del.busy = true; busy[p.id] = true;
      try {
        await api(`/api/profiles/${p.id}`, { method: 'DELETE' });
        toast(t('profiles.deleted', { name: p.name }));
        del.p = null;
      } catch (e) { toast(e.message, true); }
      del.busy = false; busy[p.id] = false; load(); loadTrash();
    }

    const xfer = reactive({ exp: null, clone: null }); // components/transfer.js dialogs
    const tplx = reactive({ ext: null, save: null });  // pages/templates.js: profile extensions / save as template

    let timer;
    onMounted(() => {
      load(); loadTrash();
      timer = setInterval(() => load(), 5000); // local state only
      document.addEventListener('mouseup', up);
      document.addEventListener('copy', onCopy);
    });
    onUnmounted(() => {
      clearInterval(timer);
      document.removeEventListener('mouseup', up);
      document.removeEventListener('copy', onCopy);
    });

    return {
      xfer, tplx, ...px, ALL, GROUPS, FILTERS, OSES, o, pn, hint, hintDetail, goInstall, goLog, dlg, del, edit, editing, cookies, cookiesFor, nameInput, openCreate, create, remove, trash, openDel, leftText, restore, purge, emptyTrash, saveDays, head, busy, q, filter, refreshing, sheet,
      runCount, view, raw, orgx, manage, load, cellState, selectRow, down, over, dbl, onKey, copyAll, refresh, act, reveal, fmt,
    };
  },
  template: `<cm-page :title="t('nav.profiles')" :count="o ? o.profiles.length : null">
    <template #tools>
      <div class="search"><cm-icon name="search" :size="14"/><input v-model="q" :placeholder="t('profiles.search')" autocomplete="off"></div>
      <div class="inline-flex bg-panel2 rounded-[10px] p-[3px]">
        <button v-for="f in FILTERS" :key="f" @click="filter = f"
                class="border-0 px-3 py-[5px] rounded-lg cursor-pointer text-[12.5px] whitespace-nowrap"
                :class="filter === f ? 'bg-panel text-ink font-semibold shadow-sm' : 'bg-transparent text-muted'">{{ t('profiles.filter.' + f) }}</button>
      </div>
    </template>
    <template #actions>
      <span class="chip max-2xl:hidden"><span class="dot" :class="o ? (o.providerAvailable ? 'bg-ok' : 'bg-bad') : ''"></span>{{ o ? t(o.providerAvailable ? 'engine.ready' : 'engine.notReady', { name: pn }) : pn }}</span>
      <span class="chip max-2xl:hidden"><span class="dot" :class="o ? (o.proxyCount ? 'bg-ok' : 'bg-bad') : ''"></span>{{ t('profiles.proxyCount', { n: o ? o.proxyCount : 0 }) }}</span>
      <span class="chip"><span class="dot" :class="runCount ? 'bg-ok animate-pulse-ring' : ''"></span>{{ t('profiles.running', { n: runCount }) }}</span>
      <button class="btn" @click="copyAll" :title="t('profiles.copyTableTitle')"><cm-icon name="copy" :size="14"/> {{ t('profiles.copyTable') }}</button>
      <button class="btn" :disabled="refreshing" @click="refresh" :title="t('profiles.refreshTitle')">
        <span v-if="refreshing" class="spin"></span><cm-icon v-else name="refresh" :size="14"/> {{ t('common.refresh') }}
      </button>
      <button class="btn" @click="trash.open = true" :title="t('profiles.trashTitle')"><cm-icon name="trash" /> {{ trash.items.length ? t('profiles.trashCount', { n: trash.items.length }) : t('profiles.trash') }}</button>
      <cm-transfer :export-profile="xfer.exp" :clone-profile="xfer.clone" @close-export="xfer.exp = null" @close-clone="xfer.clone = null" @changed="load()"/>
      <cm-sheets @changed="load()"/>
      <button v-if="o" class="btn btn-start" :disabled="!o.providerAvailable" :title="o.providerAvailable ? t('profiles.newTitle') : hint" @click="openCreate">
        <cm-icon name="plus" :size="14"/> {{ t('profiles.new') }}
      </button>
    </template>

    <!-- Engine not installed (Camoufox is optional) -->
    <div v-if="o && !o.providerAvailable && o.providerHint" class="banner">
      <cm-icon name="alert"/><div class="flex-1 min-w-0">
        <b>{{ t('profiles.engineNotReadyTitle', { name: pn }) }}</b> — {{ t('profiles.engineNotReadyBody') }}<br><span class="opacity-85">{{ hint }}</span>
        <details v-if="hintDetail" class="mt-1"><summary class="cursor-pointer text-xs">{{ t('profiles.engineDetails') }}</summary><code class="text-xs break-all">{{ hintDetail }}</code></details>
      </div>
      <button v-if="o.providerInstalling" class="btn" @click="goLog"><i class="spin"></i>{{ t('profiles.engineInstalling', { name: pn }) }} · {{ t('profiles.engineViewLog') }}</button>
      <button v-else class="btn btn-start" @click="goInstall">{{ t('profiles.engineInstall', { name: pn }) }}</button>
    </div>
    <div v-if="o && o.apiError" class="banner">
      <cm-icon name="alert"/><div><b>{{ t('profiles.engineError') }}</b><br><span class="opacity-85">{{ o.apiError }}</span></div>
    </div>

    <cm-org-bar :orgx="orgx" @manage="manage = true"/>
    <cm-org-bulk v-if="orgx.checkedIds.value.length" :orgx="orgx" :profiles="o ? o.profiles : []" :proxies="proxies" :provider-available="!!o?.providerAvailable" @done="load()"/>
    <cm-org-manage :open="manage" :orgx="orgx" @close="manage = false" @changed="load()"/>

    <div class="hidden min-[1100px]:block text-[11.5px] text-faint mb-2">
      <span v-html="t('profiles.hint', { shift: '<kbd>⇧</kbd>', cmdc: '<kbd>⌘C</kbd>' })"></span>
    </div>

    <!-- ===== spreadsheet ===== -->
    <div ref="sheet" tabindex="0" @keydown="onKey" class="min-h-0 overflow-auto bg-panel border border-line rounded-xl shadow-card outline-none">
      <table class="grid-tbl">
        <colgroup>
          <col style="width:44px"><col v-for="c in ALL" :style="{ width: c.w + 'px' }"><col style="width:232px">
        </colgroup>
        <thead>
          <tr class="g">
            <th class="s0"></th><th class="s1"></th>
            <th v-for="g in GROUPS" :colspan="g.n" class="gs">{{ t(g.g) }}</th>
            <th class="sr"></th>
          </tr>
          <tr class="h">
            <th class="s0 !px-1 !text-left"><input type="checkbox" class="align-middle m-0 mr-1" :checked="orgx.allChecked.value" :indeterminate.prop="orgx.someChecked.value && !orgx.allChecked.value" @change="orgx.toggleAll()" :title="t('org.selectAll')">#</th>
            <th v-for="(c, i) in ALL" :class="[i === 0 ? 's1' : c.g ? 'gs' : '', c.h && c.k !== 'check' && c.k !== 'reveal' ? 'cursor-pointer select-none' : '']" :title="c.h && c.k !== 'check' && c.k !== 'reveal' ? head(c) + ' · ' + t('org.sort') : head(c)" @click="orgx.setSort(c)">{{ head(c) }}{{ orgx.sortMark(c) }}</th>
            <th class="sr"></th>
          </tr>
        </thead>
        <tbody>
          <tr v-if="!o"><td :colspan="ALL.length + 2" class="!p-12 text-center text-muted"><div class="skel w-[30%]"></div></td></tr>
          <tr v-else-if="!view.length"><td :colspan="ALL.length + 2" class="!p-12 text-center text-muted">{{ t('profiles.noMatch') }}</td></tr>
          <tr v-for="(p, r) in view" :key="p.id" :class="{ live: p.running }">
            <td class="s0 rn !px-1 !text-left" :title="t('profiles.selectRow')" @mousedown.prevent="selectRow(r)">
              <input type="checkbox" class="align-middle m-0 mr-1" :checked="!!orgx.checked[p.id]" :title="t('org.selectRow')" @mousedown.stop @click.stop="orgx.toggle(p.id, $event)">{{ r + 1 }}
            </td>

            <td v-for="(c, i) in ALL" :key="i"
                :class="[i === 0 ? 's1 c-name' : c.g ? 'gs' : '', c.cls, cellState(r, i)]"
                :title="raw(c, p)"
                @mousedown="down($event, r, i)" @mouseover="over(r, i)" @dblclick="dbl($event)">

              <span v-if="c.k === 'name'" class="font-bold">{{ p.name }}</span>

 <cm-org-tags v-else-if="c.k === 'tags'" :id="p.id"/>

              <span v-else-if="c.k === 'group'" :class="orgx.groupOf(p.id) ? '' : 'text-faint'">{{ raw(c, p) }}</span>

              <span v-else-if="c.k === 'status'" class="inline-flex items-center gap-1.5" :class="p.running ? '' : p.crashed ? 'text-bad' : 'text-faint'" :title="p.crashed && !p.running ? p.crashed.detail : ''">
                <span class="dot" :class="p.running ? 'bg-ok animate-pulse-ring' : p.crashed ? 'bg-bad' : ''"></span>{{ raw(c, p) }}
              </span>

              <select v-else-if="c.k === 'proxy'" :value="p.assigned ? p.assigned.id : ''" :disabled="!!p.running || !!busy[p.id]"
                      :title="p.running ? t('profiles.stopFirst') : t('profiles.chooseProxy')" @change="assign(p, $event)">
                <option value="">{{ noProxy() }}</option>
                <option v-for="x in proxies" :key="x.id" :value="x.id">#{{ x.id }} {{ x.label }}{{ x.geo ? ' · ' + x.geo.countryCode : '' }}</option>
              </select>

              <span v-else-if="c.k === 'type'">
                <span v-if="p.assigned" class="text-ok font-semibold" :title="t('profiles.failClosed')">{{ t('profiles.own') }}</span>
                <span v-else class="text-warn">{{ t('profiles.no') }}</span>
              </span>

              <button v-else-if="c.k === 'check'" class="icon-btn" :class="{ invisible: !p.assigned }" @click="check(p.assigned.id)" :title="t('profiles.checkProxy')">
                <span v-if="checking === p.assigned?.id" class="spin"></span><template v-else>⟳</template>
              </button>

              <template v-else-if="c.k === 'geo'">
                <template v-if="p.assigned?.geo">{{ fmt.flag(p.assigned.geo.countryCode) }} {{ p.assigned.geo.country }}</template>
                <template v-else>—</template>
              </template>

              <button v-else-if="c.k === 'reveal'" class="icon-btn" :disabled="!p.path.exists" @click="reveal(p)" :title="t('profiles.reveal')"><cm-icon name="folder" :size="15" /></button>

              <template v-else-if="c.k === 'yn'">
                <span v-if="c.get(p) === undefined" class="text-faint">—</span>
                <span v-else-if="c.get(p)" class="text-ok font-bold">✓</span>
                <span v-else class="text-faint">✗</span>
              </template>

              <template v-else>{{ raw(c, p) }}</template>
            </td>

            <td class="sr">
              <button v-if="p.running" class="btn btn-stop" :disabled="!!busy[p.id]" @click="act('stop', p.id)">
                <span v-if="busy[p.id]" class="spin"></span><template v-else>■</template> {{ t('profiles.stop') }}
              </button>
              <button v-else class="btn btn-start" :disabled="!!busy[p.id] || !o.providerAvailable" :title="o.providerAvailable ? '' : hint" @click="act('start', p.id)">
                <span v-if="busy[p.id]" class="spin"></span><template v-else><cm-icon name="play" :size="12" /> {{ t('profiles.start') }}</template>
              </button>
              <button class="icon-btn ml-1" :disabled="!!busy[p.id]" :title="t('profiles.edit')" @click="edit.id = p.id"><cm-icon name="pencil" :size="15" /></button>
              <button class="icon-btn" :disabled="!!busy[p.id]" :title="t('transfer.export')" @click="xfer.exp = p"><cm-icon name="download" :size="15" /></button>
              <button class="icon-btn" :disabled="!!busy[p.id]" :title="t('transfer.clone')" @click="xfer.clone = p"><cm-icon name="duplicate" :size="15" /></button>
              <button class="icon-btn" :disabled="!!busy[p.id] || !!p.running" :title="p.running ? t('cookies.running') : t('cookies.title', { name: p.name })" @click="cookies.id = p.id"><cm-icon name="cookie" :size="15" /></button>
              <button class="icon-btn" :disabled="!!busy[p.id]" :title="t('tpl.prof.extBtn')" @click="tplx.ext = p"><cm-icon name="puzzle" :size="15" /></button>
              <button class="icon-btn" :disabled="!!busy[p.id]" :title="t('tpl.prof.starTitle')" @click="tplx.save = p"><cm-icon name="star" :size="15" /></button>
              <button class="icon-btn" :disabled="!!busy[p.id]" :title="t('profiles.delete')" @click="openDel(p)"><cm-icon name="trash" :size="15" /></button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- ===== create ===== -->
    <cm-dialog :open="dlg.open" :title="t('profiles.createTitle', { name: pn })" :busy="dlg.busy" @close="dlg.open = false">
      <form id="create-profile" class="flex flex-col gap-3" @submit.prevent="create">
        <cm-field :label="t('profiles.name')">
          <input ref="nameInput" class="input" v-model="dlg.name" maxlength="80" autocomplete="off" :placeholder="t('profiles.namePlaceholder')">
        </cm-field>
        <cm-field :label="t('profiles.os')" :hint="t('profiles.fpOnce')">
          <select class="input" v-model="dlg.os"><option v-for="x in OSES" :key="x.k" :value="x.k">{{ x.t }}</option></select>
        </cm-field>
      </form>
      <template #footer>
        <button type="button" class="btn" :disabled="dlg.busy" @click="dlg.open = false">{{ t('common.cancel') }}</button>
        <button type="submit" form="create-profile" class="btn btn-start" :disabled="dlg.busy || !dlg.name.trim()">
          <template v-if="dlg.busy"><span class="spin"></span> {{ t('profiles.creating') }}</template><template v-else>{{ t('common.create') }}</template>
        </button>
      </template>
    </cm-dialog>

    <!-- ===== edit (components/profile-form.js) ===== -->
    <cm-profile-form :open="!!editing" :profile="editing" @close="edit.id = null" @saved="load()"/>
    <cm-cookies :profile="cookiesFor" @close="cookies.id = null"/>
    <cm-tpl-dialogs :ext-profile="tplx.ext" :save-profile="tplx.save" @close-ext="tplx.ext = null" @close-save="tplx.save = null"/>

    <!-- ===== delete ===== -->
    <cm-dialog :open="!!del.p" :title="del.p ? t('profiles.deleteTitle', { name: del.p.name }) : ''" :busy="del.busy" @close="del.p = null">
      <p class="m-0 text-[13px] text-muted">{{ trash.days ? t('profiles.deleteBodyTrash', { days: trash.days }) : t('profiles.deleteBody') }}<template v-if="del.p?.running"> {{ t('profiles.deleteRunning') }}</template></p>
      <template #footer>
        <button class="btn" :disabled="del.busy" @click="del.p = null">{{ t('common.cancel') }}</button>
        <button class="btn btn-stop" :disabled="del.busy" @click="remove">
          <template v-if="del.busy"><span class="spin"></span> {{ t('profiles.deleting') }}</template><template v-else>{{ t('common.delete') }}</template>
        </button>
      </template>
    </cm-dialog>

    <!-- ===== trash (CM-3.1) ===== -->
    <cm-dialog :open="trash.open" :title="t('profiles.trash')" :busy="trash.busy" :width="520" @close="trash.open = false">
      <p class="m-0 text-[13px] text-muted">{{ trash.days ? t('profiles.trashInfo', { days: trash.days }) : t('profiles.trashOff') }}</p>
      <p v-if="!trash.items.length" class="m-0 py-4 text-center text-[13px] text-muted">{{ t('profiles.trashEmpty') }}</p>
      <ul v-else class="m-0 p-0 list-none flex flex-col gap-1 max-h-[320px] overflow-auto">
        <li v-for="it in trash.items" :key="it.id" class="flex items-center gap-2 px-2 py-[6px] rounded-lg bg-panel2">
          <div class="flex-1 min-w-0">
            <div class="text-[13px] font-semibold truncate">{{ it.name }}</div>
            <div class="text-[11.5px] text-faint">{{ t('profiles.trashDeleted', { ago: fmt.ago(it.deletedAt) }) }} · {{ leftText(it) }}</div>
          </div>
          <button class="btn" :disabled="trash.busy" @click="restore(it)">{{ t('profiles.restore') }}</button>
          <button class="btn btn-stop" :disabled="trash.busy" @click="trash.confirm = { p: it }">{{ t('profiles.purge') }}</button>
        </li>
      </ul>
      <label class="flex items-center gap-2 text-[12.5px] text-muted">{{ t('profiles.trashDays') }}
        <input type="number" min="0" max="365" class="w-[72px]" :value="trash.days" @change="saveDays">
      </label>
      <template #footer>
        <button class="btn btn-stop" :disabled="trash.busy || !trash.items.length" @click="trash.confirm = { all: true }">{{ t('profiles.emptyTrash') }}</button>
        <button class="btn" @click="trash.open = false">{{ t('common.close') }}</button>
      </template>
    </cm-dialog>
    <cm-dialog :open="!!trash.confirm" :busy="trash.busy" :title="trash.confirm?.all ? t('profiles.emptyTrashTitle') : trash.confirm ? t('profiles.purgeTitle', { name: trash.confirm.p.name }) : ''" @close="trash.confirm = null">
      <p class="m-0 text-[13px] text-muted">{{ trash.confirm?.all ? t('profiles.emptyTrashBody', { n: trash.items.length }) : t('profiles.purgeBody') }}</p>
      <template #footer>
        <button class="btn" :disabled="trash.busy" @click="trash.confirm = null">{{ t('common.cancel') }}</button>
        <button class="btn btn-stop" :disabled="trash.busy" @click="trash.confirm?.all ? emptyTrash() : purge(trash.confirm.p)">
          <span v-if="trash.busy" class="spin"></span><template v-else>{{ t('common.delete') }}</template>
        </button>
      </template>
    </cm-dialog>
  </cm-page>`,
};

} // end file-local scope
