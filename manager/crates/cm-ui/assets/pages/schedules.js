/* Schedules (phase 10): warming launches and browser refresh for one profile or a tag / group,
   recurrence with timezone, missed-run grace, preconditions, enable/disable, "run now" and the
   durable run history. API: /api/schedules (see cm-api schedules.rs). */

I18N.add({
  en: {
    'schedules.intro': 'Scheduled runs open a profile on its own, browse for a while and close it again. They only run while this app is open; a run that was missed while it was closed starts late inside its grace window, otherwise it is recorded as skipped.',
    'schedules.add': 'New schedule',
    'schedules.edit': 'Edit',
    'schedules.addTitle': 'New schedule',
    'schedules.editTitle': 'Edit schedule',
    'schedules.empty': 'No schedules yet.',
    'schedules.col.name': 'Schedule',
    'schedules.col.target': 'Target',
    'schedules.col.when': 'When',
    'schedules.col.next': 'Next run',
    'schedules.col.last': 'Last run',
    'schedules.col.on': 'On',
    'schedules.runNow': 'Run now',
    'schedules.runStarted': 'Started — progress shows in the history below',
    'schedules.saved': 'Schedule saved',
    'schedules.deleted': 'Schedule deleted',
    'schedules.deleteTitle': 'Delete schedule?',
    'schedules.deleteBody': '{name} and its run history will be removed.',
    'schedules.running': 'running',
    'schedules.disabled': 'disabled',
    'schedules.never': 'never',
    'schedules.f.name': 'Name',
    'schedules.f.target': 'Applies to',
    'schedules.target.profile': 'One profile',
    'schedules.target.tag': 'Every profile with a tag',
    'schedules.target.group': 'Every profile in a group',
    'schedules.f.pick': 'Choose…',
    'schedules.f.action': 'What to do',
    'schedules.action.warm': 'Warming launch',
    'schedules.action.refresh_browser': 'Browser refresh',
    'schedules.action.refreshHint': 'Records the Camoufox version check. It never changes the profile\'s hardware.',
    'schedules.f.urls': 'URLs to visit (one per line)',
    'schedules.f.urlsHint': 'http:// or https:// only, up to 20.',
    'schedules.f.dwell': 'Minutes on each URL',
    'schedules.f.dwellHint': 'Light scrolling while it stays. 0.1 = 6 seconds.',
    'schedules.f.recurrence': 'Repeat',
    'schedules.rec.daily': 'Every day',
    'schedules.rec.weekdays': 'On weekdays',
    'schedules.rec.every_hours': 'Every N hours',
    'schedules.f.time': 'At',
    'schedules.f.hours': 'Hours',
    'schedules.f.days': 'Days',
    'schedules.f.timezone': 'Timezone',
    'schedules.f.timezoneHint': 'IANA name, e.g. Europe/Berlin.',
    'schedules.f.grace': 'Grace window (minutes)',
    'schedules.f.graceHint': 'A missed run still starts if it is at most this late; otherwise it is recorded as skipped.',
    'schedules.f.concurrent': 'Profiles at once',
    'schedules.f.concurrentHint': 'For tags and groups: how many profiles run in parallel.',
    'schedules.f.pre': 'Before each run',
    'schedules.f.preClosed': 'The profile must be closed (waits for it)',
    'schedules.f.preProxy': 'The proxy check must pass',
    'schedules.f.preTimeout': 'Wait up to (seconds)',
    'schedules.f.enabled': 'Enabled',
    'schedules.days.0': 'Mon', 'schedules.days.1': 'Tue', 'schedules.days.2': 'Wed', 'schedules.days.3': 'Thu',
    'schedules.days.4': 'Fri', 'schedules.days.5': 'Sat', 'schedules.days.6': 'Sun',
    'schedules.sum.warm': 'Visit {n} URL(s), {m} min each',
    'schedules.sum.refresh': 'Browser refresh',
    'schedules.sum.daily': 'daily {at}',
    'schedules.sum.weekdays': '{days} {at}',
    'schedules.sum.every': 'every {n} h',
    'schedules.history': 'Run history',
    'schedules.history.empty': 'No runs yet.',
    'schedules.h.schedule': 'Schedule',
    'schedules.h.profile': 'Profile',
    'schedules.h.started': 'Started',
    'schedules.h.ended': 'Ended',
    'schedules.h.outcome': 'Result',
    'schedules.h.details': 'Details',
    'schedules.out.running': 'Running',
    'schedules.out.ok': 'OK',
    'schedules.out.failed': 'Failed',
    'schedules.out.skipped': 'Skipped',
    'schedules.manual': 'manual',
    'schedules.err.urls': 'Enter at least one URL',
  },
  vi: {
    'schedules.intro': 'Lịch chạy tự mở profile, lướt một lúc rồi đóng lại. Lịch chỉ chạy khi app đang mở; lần chạy bị lỡ lúc app tắt sẽ chạy muộn nếu còn trong thời gian châm chước, nếu không sẽ ghi là bỏ qua.',
    'schedules.add': 'Lịch mới',
    'schedules.edit': 'Sửa',
    'schedules.addTitle': 'Lịch mới',
    'schedules.editTitle': 'Sửa lịch',
    'schedules.empty': 'Chưa có lịch nào.',
    'schedules.col.name': 'Lịch',
    'schedules.col.target': 'Áp dụng cho',
    'schedules.col.when': 'Khi nào',
    'schedules.col.next': 'Lần chạy tới',
    'schedules.col.last': 'Lần chạy gần nhất',
    'schedules.col.on': 'Bật',
    'schedules.runNow': 'Chạy ngay',
    'schedules.runStarted': 'Đã bắt đầu — tiến độ hiện ở lịch sử bên dưới',
    'schedules.saved': 'Đã lưu lịch',
    'schedules.deleted': 'Đã xoá lịch',
    'schedules.deleteTitle': 'Xoá lịch?',
    'schedules.deleteBody': '{name} và lịch sử chạy của nó sẽ bị xoá.',
    'schedules.running': 'đang chạy',
    'schedules.disabled': 'đã tắt',
    'schedules.never': 'chưa lần nào',
    'schedules.f.name': 'Tên',
    'schedules.f.target': 'Áp dụng cho',
    'schedules.target.profile': 'Một profile',
    'schedules.target.tag': 'Mọi profile có một tag',
    'schedules.target.group': 'Mọi profile trong một nhóm',
    'schedules.f.pick': 'Chọn…',
    'schedules.f.action': 'Việc cần làm',
    'schedules.action.warm': 'Mở để làm ấm',
    'schedules.action.refresh_browser': 'Cập nhật trình duyệt',
    'schedules.action.refreshHint': 'Ghi lại kết quả kiểm tra phiên bản Camoufox. Không bao giờ đổi phần cứng của profile.',
    'schedules.f.urls': 'Các URL cần ghé (mỗi dòng một URL)',
    'schedules.f.urlsHint': 'Chỉ http:// hoặc https://, tối đa 20.',
    'schedules.f.dwell': 'Số phút ở mỗi URL',
    'schedules.f.dwellHint': 'Cuộn nhẹ trong lúc ở lại. 0.1 = 6 giây.',
    'schedules.f.recurrence': 'Lặp lại',
    'schedules.rec.daily': 'Mỗi ngày',
    'schedules.rec.weekdays': 'Theo thứ trong tuần',
    'schedules.rec.every_hours': 'Mỗi N giờ',
    'schedules.f.time': 'Lúc',
    'schedules.f.hours': 'Số giờ',
    'schedules.f.days': 'Các thứ',
    'schedules.f.timezone': 'Múi giờ',
    'schedules.f.timezoneHint': 'Tên IANA, ví dụ Asia/Ho_Chi_Minh.',
    'schedules.f.grace': 'Thời gian châm chước (phút)',
    'schedules.f.graceHint': 'Lần chạy bị lỡ vẫn chạy nếu muộn không quá số phút này; nếu không sẽ ghi là bỏ qua.',
    'schedules.f.concurrent': 'Số profile cùng lúc',
    'schedules.f.concurrentHint': 'Với tag và nhóm: bao nhiêu profile chạy song song.',
    'schedules.f.pre': 'Trước mỗi lần chạy',
    'schedules.f.preClosed': 'Profile phải đang đóng (sẽ chờ)',
    'schedules.f.preProxy': 'Kiểm tra proxy phải đạt',
    'schedules.f.preTimeout': 'Chờ tối đa (giây)',
    'schedules.f.enabled': 'Bật',
    'schedules.days.0': 'T2', 'schedules.days.1': 'T3', 'schedules.days.2': 'T4', 'schedules.days.3': 'T5',
    'schedules.days.4': 'T6', 'schedules.days.5': 'T7', 'schedules.days.6': 'CN',
    'schedules.sum.warm': 'Ghé {n} URL, mỗi URL {m} phút',
    'schedules.sum.refresh': 'Cập nhật trình duyệt',
    'schedules.sum.daily': 'mỗi ngày lúc {at}',
    'schedules.sum.weekdays': '{days} lúc {at}',
    'schedules.sum.every': 'mỗi {n} giờ',
    'schedules.history': 'Lịch sử chạy',
    'schedules.history.empty': 'Chưa có lần chạy nào.',
    'schedules.h.schedule': 'Lịch',
    'schedules.h.profile': 'Profile',
    'schedules.h.started': 'Bắt đầu',
    'schedules.h.ended': 'Kết thúc',
    'schedules.h.outcome': 'Kết quả',
    'schedules.h.details': 'Chi tiết',
    'schedules.out.running': 'Đang chạy',
    'schedules.out.ok': 'Thành công',
    'schedules.out.failed': 'Lỗi',
    'schedules.out.skipped': 'Bỏ qua',
    'schedules.manual': 'thủ công',
    'schedules.err.urls': 'Nhập ít nhất một URL',
  },
});

CM.pages.schedules = {
  setup() {
    const loaded = Vue.ref(false), list = Vue.ref([]), history = Vue.ref([]);
    const profiles = Vue.ref([]), tags = Vue.ref([]), groups = Vue.ref([]);
    let timer = null;

    async function load() {
      try {
        const [s, h] = await Promise.all([api('/api/schedules'), api('/api/schedules/runs?limit=100')]);
        list.value = s; history.value = h;
      } catch (e) { toast(t('common.loadFailed', { msg: e.message }), true); }
      loaded.value = true;
    }
    async function loadChoices() {
      try {
        const [ov, org] = await Promise.all([api('/api/overview'), api('/api/org')]);
        profiles.value = ov.profiles.map(p => ({ id: p.id, name: p.name }));
        tags.value = org.tags; groups.value = org.groups;
      } catch (_) { /* the form still works with ids typed by API users */ }
    }
    Vue.onMounted(() => { load(); loadChoices(); timer = setInterval(load, 5000); });
    Vue.onUnmounted(() => clearInterval(timer));

    const when = ts => ts ? new Date(ts * 1000).toLocaleString() : '—';
    const profileName = id => id ? (profiles.value.find(p => p.id === id)?.name || id.slice(0, 8)) : '—';
    const scheduleName = id => list.value.find(s => s.id === id)?.name || '#' + id;

    function summary(s) {
      const a = s.action.type === 'warm'
        ? t('schedules.sum.warm', { n: s.action.urls.length, m: +(s.action.dwellSecs / 60).toFixed(2) })
        : t('schedules.sum.refresh');
      const r = s.recurrence;
      const rec = r.type === 'daily' ? t('schedules.sum.daily', { at: r.at })
        : r.type === 'weekdays' ? t('schedules.sum.weekdays', { days: r.days.slice().sort().map(d => t('schedules.days.' + d)).join(' '), at: r.at })
        : t('schedules.sum.every', { n: r.hours });
      return { a, rec: `${rec} · ${s.timezone}` };
    }
    const outClass = o => ({ ok: 'text-ok', failed: 'text-bad', skipped: 'text-warn', running: 'text-accent' }[o] || '');

    const columns = Vue.computed(() => [
      { key: 'name', label: t('schedules.col.name') },
      { key: 'target', label: t('schedules.col.target') },
      { key: 'when', label: t('schedules.col.when') },
      { key: 'next', label: t('schedules.col.next') },
      { key: 'last', label: t('schedules.col.last') },
      { key: 'on', label: t('schedules.col.on') },
      { key: 'actions', label: '', cls: 'text-right whitespace-nowrap', hcls: 'text-right' },
    ]);
    const hcolumns = Vue.computed(() => [
      { key: 'schedule', label: t('schedules.h.schedule') },
      { key: 'profile', label: t('schedules.h.profile') },
      { key: 'started', label: t('schedules.h.started') },
      { key: 'ended', label: t('schedules.h.ended') },
      { key: 'outcome', label: t('schedules.h.outcome') },
      { key: 'details', label: t('schedules.h.details') },
    ]);

    /* ---- create / edit ---- */
    const browserTz = () => { try { return Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC'; } catch (_) { return 'UTC'; } };
    const zones = (() => { try { return Intl.supportedValuesOf('timeZone'); } catch (_) { return []; } })();
    const blank = () => ({
      id: null, name: '', targetType: 'profile', targetId: '', actionType: 'warm', urls: '', dwellMin: 1,
      recType: 'daily', at: '09:00', days: [0, 1, 2, 3, 4], hours: 6, timezone: browserTz(), grace: 10,
      maxConcurrent: 1, preClosed: true, preProxy: true, preTimeout: 60, enabled: true,
    });
    const form = Vue.reactive({ open: false, busy: false, ...blank() });
    function openForm(s) {
      if (!s) return Object.assign(form, blank(), { open: true });
      const tg = s.target, a = s.action, r = s.recurrence;
      Object.assign(form, blank(), {
        open: true, id: s.id, name: s.name, targetType: tg.type, targetId: tg.id, actionType: a.type,
        urls: a.type === 'warm' ? a.urls.join('\n') : '', dwellMin: a.type === 'warm' ? +(a.dwellSecs / 60).toFixed(2) : 1,
        recType: r.type, at: r.at || '09:00', days: r.days ? r.days.slice() : [0, 1, 2, 3, 4], hours: r.hours || 6,
        timezone: s.timezone, grace: s.graceMinutes, maxConcurrent: s.maxConcurrent, enabled: s.enabled,
        preClosed: s.preconditions.profileClosed, preProxy: s.preconditions.proxyCheck, preTimeout: s.preconditions.timeoutSecs,
      });
    }
    const choices = Vue.computed(() => form.targetType === 'profile' ? profiles.value : form.targetType === 'tag' ? tags.value : groups.value);
    Vue.watch(() => form.targetType, () => { form.targetId = ''; });
    function toggleDay(d) { const i = form.days.indexOf(d); if (i >= 0) form.days.splice(i, 1); else form.days.push(d); }

    function body() {
      const urls = form.urls.split('\n').map(x => x.trim()).filter(Boolean);
      const rec = form.recType === 'daily' ? { type: 'daily', at: form.at }
        : form.recType === 'weekdays' ? { type: 'weekdays', at: form.at, days: form.days.slice().sort() }
        : { type: 'every_hours', hours: Number(form.hours) };
      return {
        name: form.name, target: { type: form.targetType, id: form.targetType === 'profile' ? form.targetId : Number(form.targetId) },
        action: form.actionType === 'warm' ? { type: 'warm', urls, dwellSecs: Math.round(Number(form.dwellMin) * 60) } : { type: 'refresh_browser' },
        recurrence: rec, timezone: form.timezone, enabled: form.enabled, graceMinutes: Number(form.grace),
        maxConcurrent: Number(form.maxConcurrent),
        preconditions: { profileClosed: form.preClosed, proxyCheck: form.preProxy, timeoutSecs: Number(form.preTimeout) },
      };
    }
    async function save() {
      if (form.actionType === 'warm' && !form.urls.trim()) return toast(t('schedules.err.urls'), true);
      form.busy = true;
      try {
        await api(form.id ? `/api/schedules/${form.id}` : '/api/schedules', { method: form.id ? 'PUT' : 'POST', body: JSON.stringify(body()) });
        form.open = false; toast(t('schedules.saved')); load();
      } catch (e) { toast(e.message, true); }
      form.busy = false;
    }

    async function toggle(s) {
      try { await api(`/api/schedules/${s.id}/enabled`, { method: 'POST', body: JSON.stringify({ enabled: !s.enabled }) }); } catch (e) { toast(e.message, true); }
      load();
    }
    async function runNow(s) {
      try { await api(`/api/schedules/${s.id}/run`, { method: 'POST' }); toast(t('schedules.runStarted')); } catch (e) { toast(e.message, true); }
      setTimeout(load, 600);
    }
    const del = Vue.reactive({ open: false, busy: false, s: null });
    async function doDelete() {
      del.busy = true;
      try { await api(`/api/schedules/${del.s.id}`, { method: 'DELETE' }); del.open = false; toast(t('schedules.deleted')); load(); }
      catch (e) { toast(e.message, true); }
      del.busy = false;
    }

    return { loaded, list, history, profiles, tags, groups, columns, hcolumns, when, profileName, scheduleName, summary, outClass,
      zones, form, openForm, choices, toggleDay, save, toggle, runNow, del, doDelete,
      dayKeys: [0, 1, 2, 3, 4, 5, 6] };
  },
  template: `<cm-page :title="t('nav.schedules')" :count="list.length">
    <template #actions>
      <button class="btn btn-start" @click="openForm()"><cm-icon name="plus" :size="14"/> {{ t('schedules.add') }}</button>
    </template>
    <p class="m-0 mb-3 text-[13px] text-muted">{{ t('schedules.intro') }}</p>
    <div class="min-h-0 flex-1 overflow-auto flex flex-col gap-4">
      <div class="flex flex-col bg-panel border border-line rounded-xl shadow-card">
        <cm-table :columns="columns" :rows="list" :loading="!loaded" :empty="t('schedules.empty')">
          <template #cell-name="{ row }">
            <div class="font-semibold">{{ row.name }}</div>
            <div class="text-[11.5px] text-faint">{{ summary(row).a }}</div>
          </template>
          <template #cell-target="{ row }">
            <div>{{ row.targetLabel }}</div>
            <div class="text-[11.5px] text-faint">{{ t('schedules.target.' + row.target.type) }}</div>
          </template>
          <template #cell-when="{ row }">{{ summary(row).rec }}</template>
          <template #cell-next="{ row }"><span v-if="row.running" class="text-accent">{{ t('schedules.running') }}</span><span v-else-if="!row.enabled" class="text-muted">{{ t('schedules.disabled') }}</span><span v-else>{{ when(row.nextRunAt) }}</span></template>
          <template #cell-last="{ row }">
            <template v-if="row.lastRun"><span :class="outClass(row.lastRun.outcome)">{{ t('schedules.out.' + row.lastRun.outcome) }}</span> <span class="text-[11.5px] text-faint">{{ when(row.lastRun.startedAt) }}</span></template>
            <span v-else class="text-muted">{{ t('schedules.never') }}</span>
          </template>
          <template #cell-on="{ row }"><cm-switch :model-value="row.enabled" @update:model-value="toggle(row)"/></template>
          <template #cell-actions="{ row }">
            <div class="inline-flex gap-1.5">
              <button class="btn" :disabled="row.running" @click="runNow(row)">{{ t('schedules.runNow') }}</button>
              <button class="btn" @click="openForm(row)">{{ t('schedules.edit') }}</button>
              <button class="btn" @click="del.s = row; del.open = true">{{ t('common.delete') }}</button>
            </div>
          </template>
        </cm-table>
      </div>

      <div class="flex flex-col bg-panel border border-line rounded-xl shadow-card">
        <h2 class="m-0 px-4 pt-3 text-[13px] font-semibold">{{ t('schedules.history') }}</h2>
        <cm-table :columns="hcolumns" :rows="history" :loading="!loaded" :empty="t('schedules.history.empty')">
          <template #cell-schedule="{ row }">{{ scheduleName(row.scheduleId) }}<span v-if="row.trigger === 'manual'" class="text-[11.5px] text-faint"> · {{ t('schedules.manual') }}</span></template>
          <template #cell-profile="{ row }">{{ profileName(row.profileId) }}</template>
          <template #cell-started="{ row }">{{ when(row.startedAt) }}</template>
          <template #cell-ended="{ row }">{{ row.finishedAt ? when(row.finishedAt) : '—' }}</template>
          <template #cell-outcome="{ row }"><span class="font-semibold" :class="outClass(row.outcome)">{{ t('schedules.out.' + row.outcome) }}</span></template>
          <template #cell-details="{ row }"><span class="text-[12.5px] text-muted">{{ row.message || '' }}</span></template>
        </cm-table>
      </div>
    </div>

    <cm-dialog :open="form.open" :title="form.id ? t('schedules.editTitle') : t('schedules.addTitle')" :busy="form.busy" :width="560" @close="form.open = false">
      <div class="flex flex-col gap-3 max-h-[70vh] overflow-auto pr-1">
        <cm-field :label="t('schedules.f.name')"><input class="input" v-model="form.name" autocomplete="off"></cm-field>
        <div class="grid grid-cols-2 gap-2 max-sm:grid-cols-1">
          <cm-field :label="t('schedules.f.target')">
            <select class="input" v-model="form.targetType">
              <option v-for="k in ['profile', 'tag', 'group']" :key="k" :value="k">{{ t('schedules.target.' + k) }}</option>
            </select>
          </cm-field>
          <cm-field label="&nbsp;">
            <select class="input" v-model="form.targetId">
              <option value="">{{ t('schedules.f.pick') }}</option>
              <option v-for="c in choices" :key="c.id" :value="c.id">{{ c.name }}</option>
            </select>
          </cm-field>
        </div>
        <cm-field :label="t('schedules.f.action')" :hint="form.actionType === 'refresh_browser' ? t('schedules.action.refreshHint') : ''">
          <select class="input" v-model="form.actionType">
            <option v-for="k in ['warm', 'refresh_browser']" :key="k" :value="k">{{ t('schedules.action.' + k) }}</option>
          </select>
        </cm-field>
        <template v-if="form.actionType === 'warm'">
          <cm-field :label="t('schedules.f.urls')" :hint="t('schedules.f.urlsHint')">
            <textarea class="input font-mono text-[12.5px] min-h-[84px]" v-model="form.urls" spellcheck="false" autocomplete="off" placeholder="https://example.com"></textarea>
          </cm-field>
          <cm-field :label="t('schedules.f.dwell')" :hint="t('schedules.f.dwellHint')"><input class="input" type="number" min="0.1" max="60" step="0.1" v-model="form.dwellMin"></cm-field>
        </template>
        <div class="grid grid-cols-2 gap-2 max-sm:grid-cols-1">
          <cm-field :label="t('schedules.f.recurrence')">
            <select class="input" v-model="form.recType">
              <option v-for="k in ['daily', 'weekdays', 'every_hours']" :key="k" :value="k">{{ t('schedules.rec.' + k) }}</option>
            </select>
          </cm-field>
          <cm-field v-if="form.recType !== 'every_hours'" :label="t('schedules.f.time')"><input class="input" type="time" v-model="form.at"></cm-field>
          <cm-field v-else :label="t('schedules.f.hours')"><input class="input" type="number" min="1" max="720" v-model="form.hours"></cm-field>
        </div>
        <div v-if="form.recType === 'weekdays'" class="flex flex-wrap gap-1.5">
          <button v-for="d in dayKeys" :key="d" type="button" class="btn" :class="form.days.includes(d) ? 'btn-start' : ''" @click="toggleDay(d)">{{ t('schedules.days.' + d) }}</button>
        </div>
        <cm-field :label="t('schedules.f.timezone')" :hint="t('schedules.f.timezoneHint')">
          <input class="input" list="cm-zones" v-model="form.timezone" autocomplete="off" spellcheck="false">
          <datalist id="cm-zones"><option v-for="z in zones" :key="z" :value="z"></option></datalist>
        </cm-field>
        <div class="grid grid-cols-2 gap-2 max-sm:grid-cols-1">
          <cm-field :label="t('schedules.f.grace')" :hint="t('schedules.f.graceHint')"><input class="input" type="number" min="0" max="1440" v-model="form.grace"></cm-field>
          <cm-field v-if="form.targetType !== 'profile'" :label="t('schedules.f.concurrent')" :hint="t('schedules.f.concurrentHint')"><input class="input" type="number" min="1" max="10" v-model="form.maxConcurrent"></cm-field>
        </div>
        <div v-if="form.actionType === 'warm'" class="flex flex-col gap-2">
          <div class="text-xs text-muted">{{ t('schedules.f.pre') }}</div>
          <cm-switch v-model="form.preClosed" :label="t('schedules.f.preClosed')"/>
          <cm-switch v-model="form.preProxy" :label="t('schedules.f.preProxy')"/>
          <cm-field :label="t('schedules.f.preTimeout')"><input class="input" type="number" min="0" max="3600" v-model="form.preTimeout"></cm-field>
        </div>
        <cm-switch v-model="form.enabled" :label="t('schedules.f.enabled')"/>
      </div>
      <template #footer>
        <button class="btn" :disabled="form.busy" @click="form.open = false">{{ t('common.cancel') }}</button>
        <button class="btn btn-start" :disabled="form.busy || !form.name.trim() || !form.targetId" @click="save">{{ t('common.save') }}</button>
      </template>
    </cm-dialog>

    <cm-dialog :open="del.open" :title="t('schedules.deleteTitle')" :busy="del.busy" @close="del.open = false">
      <p class="m-0 text-[13px] text-muted">{{ t('schedules.deleteBody', { name: del.s?.name }) }}</p>
      <template #footer>
        <button class="btn" :disabled="del.busy" @click="del.open = false">{{ t('common.cancel') }}</button>
        <button class="btn btn-stop" :disabled="del.busy" @click="doDelete">{{ t('common.delete') }}</button>
      </template>
    </cm-dialog>
  </cm-page>`,
};
