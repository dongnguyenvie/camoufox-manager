/* Proxies: the `useProxies(ctx)` composable used by the profile table (list, check, assign) and the
   Proxy library page. Phase 2 owns this file: CRUD, import, bulk / sequential assignment.
   Passwords never reach the UI (the API only says hasPassword). */

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
    'proxies.noUser': 'No username',
    'proxies.passwordStored': 'Password stored',
    'proxies.noPassword': 'No password',
    'proxies.notChecked': 'Not checked',
    'proxies.check': 'Check',
    'proxies.empty': 'No proxies yet.',
    'proxies.soon': 'Adding, editing, importing and bulk-assigning proxies is coming in a later update.',
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
    'proxies.noUser': 'Không username',
    'proxies.passwordStored': 'Đã lưu mật khẩu',
    'proxies.noPassword': 'Không mật khẩu',
    'proxies.notChecked': 'Chưa kiểm tra',
    'proxies.check': 'Kiểm tra',
    'proxies.empty': 'Chưa có proxy nào.',
    'proxies.soon': 'Thêm, sửa, nhập danh sách và gán proxy hàng loạt sẽ có ở bản cập nhật sau.',
  },
});

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

  return { proxies, checking, fetchProxies, check, assign, noProxy: () => t('proxies.none') };
}

CM.pages.proxies = {
  setup() {
    const loaded = Vue.ref(false), q = Vue.ref('');
    const px = useProxies({ load: () => load() });
    async function load() {
      try { await px.fetchProxies(); } catch (e) { toast(t('common.loadFailed', { msg: e.message }), true); }
      loaded.value = true;
    }
    Vue.onMounted(load);
    const columns = Vue.computed(() => [
      { key: 'name', label: t('proxies.col.name') },
      { key: 'connection', label: t('proxies.col.connection'), cls: 'font-mono text-[12.5px]' },
      { key: 'credentials', label: t('proxies.col.credentials') },
      { key: 'lastCheck', label: t('proxies.col.lastCheck') },
      { key: 'relay', label: t('proxies.col.relay'), cls: 'font-mono text-[12.5px]' },
      { key: 'actions', label: '', cls: 'text-right', hcls: 'text-right' },
    ]);
    const rows = Vue.computed(() => {
      const n = q.value.toLowerCase();
      return px.proxies.value.filter(p => !n || [p.name, p.label, p.geo?.country, p.geo?.countryCode].some(v => (v || '').toLowerCase().includes(n)));
    });
    return { ...px, loaded, q, columns, rows, fmt };
  },
  template: `<cm-page :title="t('nav.proxies')" :count="proxies.length">
    <template #tools>
      <div class="search"><cm-icon name="search" :size="14"/><input v-model="q" :placeholder="t('proxies.search')" autocomplete="off"></div>
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
        <template #cell-relay="{ row }">{{ row.port ? '127.0.0.1:' + row.port : '—' }}</template>
        <template #cell-actions="{ row }">
          <button class="btn" :disabled="checking === row.id" @click="check(row.id)">
            <span v-if="checking === row.id" class="spin"></span><cm-icon v-else name="check" :size="14"/> {{ t('proxies.check') }}
          </button>
        </template>
      </cm-table>
    </div>
    <p class="m-0 mt-3 text-[12.5px] text-faint">{{ t('proxies.soon') }}</p>
  </cm-page>`,
};
