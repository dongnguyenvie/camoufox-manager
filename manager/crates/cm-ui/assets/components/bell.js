/* Notification bell (phase 19), shown at the right end of every page header (<cm-bell/> in page.js).
   One shared store polls GET /api/notifications every few seconds while the window is visible.
   Rows are written by the server (profile crashed, schedule run failed/finished, proxy check failed,
   import/export finished). Each has a `kind` + `params`; the text is translated here
   (bell.t.<kind>), the English title the server stored is the fallback.
   Actions: click a row = mark read, "Mark all read", "Clear", macOS banner switch (default off). */

I18N.add({
  en: {
    'bell.title': 'Notifications',
    'bell.open': 'Notifications ({n} unread)',
    'bell.empty': 'Nothing new. Crashes, failed runs and finished imports show up here.',
    'bell.markAll': 'Mark all read',
    'bell.clear': 'Clear',
    'bell.system': 'Also show a macOS notification',
    'bell.t.profile.crashed': 'Profile "{profile}" crashed',
    'bell.t.schedule.failed': 'Scheduled run failed: {schedule}',
    'bell.t.schedule.finished': 'Scheduled run finished: {schedule}',
    'bell.t.proxy.check_failed': 'Proxy check failed: {proxy}',
    'bell.t.transfer.export_finished': 'Profile export finished',
    'bell.t.transfer.import_finished': 'Profile import finished',
    'bell.t.sheets.export_finished': 'Spreadsheet export finished',
    'bell.t.sheets.import_finished': 'Spreadsheet import finished',
    'bell.b.schedule.failed': '{profile}: {message}',
    'bell.b.schedule.finished': '{profile}',
    'bell.b.profile.crashed': '{detail}',
    'bell.b.proxy.check_failed': '{error}',
  },
  vi: {
    'bell.title': 'Thông báo',
    'bell.open': 'Thông báo ({n} chưa đọc)',
    'bell.empty': 'Chưa có gì mới. Profile bị sập, lượt chạy lỗi và import/export xong sẽ hiện ở đây.',
    'bell.markAll': 'Đánh dấu đã đọc hết',
    'bell.clear': 'Xoá hết',
    'bell.system': 'Hiện thêm thông báo của macOS',
    'bell.t.profile.crashed': 'Profile "{profile}" bị sập',
    'bell.t.schedule.failed': 'Lượt chạy theo lịch lỗi: {schedule}',
    'bell.t.schedule.finished': 'Lượt chạy theo lịch xong: {schedule}',
    'bell.t.proxy.check_failed': 'Kiểm tra proxy lỗi: {proxy}',
    'bell.t.transfer.export_finished': 'Đã export profile xong',
    'bell.t.transfer.import_finished': 'Đã import profile xong',
    'bell.t.sheets.export_finished': 'Đã export bảng tính xong',
    'bell.t.sheets.import_finished': 'Đã import bảng tính xong',
    'bell.b.schedule.failed': '{profile}: {message}',
    'bell.b.schedule.finished': '{profile}',
    'bell.b.profile.crashed': '{detail}',
    'bell.b.proxy.check_failed': '{error}',
  },
});

{
  const store = Vue.reactive({ items: [], unread: 0, systemNotify: false });
  let timer = null, users = 0;

  const apply = r => { store.items = r.items; store.unread = r.unread; store.systemNotify = r.systemNotify; };
  const refresh = async () => {
    if (document.hidden) return;
    try { apply(await api('/api/notifications?limit=50')); } catch (_) { /* server busy: keep what we have */ }
  };
  const send = async (path, method, body) => {
    try { apply(await api('/api/notifications' + path, { method, body: JSON.stringify(body || {}) })); }
    catch (e) { toast(e.message, true); }
  };
  const start = () => { if (!users++) { refresh(); timer = setInterval(refresh, 6000); } };
  const stop = () => { if (!--users && timer) { clearInterval(timer); timer = null; } };

  const text = (kind, key, p, fallback) => {
    const k = 'bell.' + key + '.' + kind, s = t(k, p || {});
    return s === k ? fallback : s;
  };

  CM.components.CmBell = {
    setup() {
      const open = Vue.ref(false);
      const root = Vue.ref(null);
      const away = e => { if (open.value && root.value && !root.value.contains(e.target)) open.value = false; };
      const key = e => { if (e.key === 'Escape') open.value = false; };
      Vue.onMounted(() => { start(); document.addEventListener('mousedown', away); document.addEventListener('keydown', key); });
      Vue.onUnmounted(() => { stop(); document.removeEventListener('mousedown', away); document.removeEventListener('keydown', key); });
      return {
        store, open, root,
        title: n => text(n.kind, 't', n.params, n.title),
        body: n => text(n.kind, 'b', n.params, n.body),
        ago: n => fmt.ago(n.createdAt),
        read: n => { if (!n.read) send('/read', 'POST', { ids: [n.id] }); },
        markAll: () => send('/read', 'POST', {}),
        clear: () => send('/clear', 'POST'),
        setSystem: on => send('/settings', 'PUT', { systemNotify: on }),
      };
    },
    template: `<div ref="root" class="relative">
      <button type="button" class="icon-btn relative" :aria-label="t('bell.open', { n: store.unread })" :title="t('bell.title')" aria-haspopup="true" :aria-expanded="open" @click="open = !open">
        <cm-icon name="bell" :size="18"/>
        <span v-if="store.unread" class="absolute -top-0.5 -right-0.5 min-w-4 h-4 px-1 rounded-full bg-bad text-white text-[10px] font-semibold leading-4 text-center">{{ store.unread > 99 ? '99+' : store.unread }}</span>
      </button>
      <div v-if="open" class="absolute right-0 top-full mt-2 w-[360px] max-w-[calc(100vw-24px)] z-30 flex flex-col bg-panel border border-line rounded-xl shadow-card overflow-hidden">
        <div class="flex items-center gap-2 px-3.5 py-2.5 border-b border-line">
          <span class="text-[13px] font-semibold flex-1">{{ t('bell.title') }}</span>
          <button class="btn" :disabled="!store.unread" @click="markAll">{{ t('bell.markAll') }}</button>
          <button class="btn" :disabled="!store.items.length" @click="clear">{{ t('bell.clear') }}</button>
        </div>
        <div class="max-h-[380px] overflow-auto">
          <p v-if="!store.items.length" class="m-0 px-4 py-6 text-center text-[13px] text-faint">{{ t('bell.empty') }}</p>
          <button v-for="n in store.items" :key="n.id" type="button" @click="read(n)"
                  class="w-full text-left flex gap-2.5 px-3.5 py-2.5 border-0 border-b border-line bg-transparent cursor-pointer hover:bg-panel2">
            <i class="dot mt-1.5" :class="n.level === 'error' ? 'bg-bad' : n.level === 'warn' ? 'bg-warn' : 'bg-ok'"></i>
            <span class="min-w-0 flex-1 flex flex-col gap-0.5">
              <span class="text-[13px] break-words" :class="n.read ? 'text-muted' : 'font-semibold'">{{ title(n) }}</span>
              <span v-if="body(n)" class="text-xs text-muted break-words">{{ body(n) }}</span>
              <span class="text-[11px] text-faint">{{ ago(n) }}</span>
            </span>
            <i v-if="!n.read" class="dot mt-1.5 bg-accent"></i>
          </button>
        </div>
        <div class="px-3.5 py-2.5 border-t border-line">
          <cm-switch :model-value="store.systemNotify" :label="t('bell.system')" @update:model-value="setSystem"/>
        </div>
      </div>
    </div>`,
  };
}
