/* Organization (phase 7): tags, groups, saved views, row selection and bulk actions.
   Used by pages/profiles.js:
   - CM.org              shared reactive data from GET /api/org (tags, groups, views, assignments);
                         column getters in profiles.js read it, so Tags / Group copy as text too.
   - CM.useOrg(ctx)      filters (tag/group/OS), sortable columns, checkbox selection, saved views.
                         ctx = { q, filter, rows: () => visible rows, profiles: () => all rows }
   - <cm-org-bar>        saved views + tag/group/OS filters + "manage tags & groups"
   - <cm-org-bulk>       toolbar for the checked rows: start, stop, proxy, tag/untag, group, delete
   - <cm-org-tags>       tag chips of one profile
   - <cm-org-manage>     dialog to create / delete tags and create / rename / delete groups */

I18N.add({
  en: {
    'org.col.tags': 'Tags',
    'org.col.group': 'Group',
    'org.noGroup': 'No group',
    'org.view': 'View',
    'org.viewNone': 'All profiles',
    'org.viewSave': 'Save view',
    'org.viewSaveTitle': 'Save the current search, filters and sort as a view',
    'org.viewName': 'View name',
    'org.viewSaved': 'Saved view {name}',
    'org.viewDelete': 'Delete this view',
    'org.viewDeleted': 'Deleted view {name}',
    'org.filter.tag': 'Tag',
    'org.filter.group': 'Group',
    'org.filter.os': 'OS',
    'org.filter.anyTag': 'Any tag',
    'org.filter.anyGroup': 'Any group',
    'org.filter.anyOs': 'Any OS',
    'org.filter.noGroup': 'No group',
    'org.filter.clear': 'Clear filters',
    'org.manage': 'Tags & groups',
    'org.manageTitle': 'Tags and groups',
    'org.tags': 'Tags',
    'org.groups': 'Groups',
    'org.newTag': 'New tag',
    'org.newGroup': 'New group',
    'org.tagColor': 'Tag colour',
    'org.add': 'Add',
    'org.rename': 'Rename',
    'org.confirmDelete': 'Sure?',
    'org.noTags': 'No tags yet',
    'org.noGroups': 'No groups yet',
    'org.deleteTagHint': 'Deleting a tag removes it from every profile.',
    'org.deleteGroupHint': 'Deleting a group leaves its profiles without a group.',
    'org.created': 'Created {name}',
    'org.sort': 'Click to sort',
    'org.selectAll': 'Select all shown profiles',
    'org.selectRow': 'Select this profile (⇧ for a range)',
    'org.selected': '{n} selected',
    'org.clear': 'Clear selection',
    'org.bulk.start': 'Start',
    'org.bulk.stop': 'Stop',
    'org.bulk.delete': 'Delete',
    'org.bulk.proxy': 'Proxy…',
    'org.bulk.tag': 'Tag…',
    'org.bulk.group': 'Move to group…',
    'org.bulk.apply': 'Apply',
    'org.bulk.addTag': 'Add tag',
    'org.bulk.removeTag': 'Remove tag',
    'org.bulk.noProxy': 'No proxy (clear)',
    'org.bulk.running': 'Running…',
    'org.bulk.done': '{action}: {ok} done, {failed} failed',
    'org.bulk.allOk': '{action}: {ok} done',
    'org.bulk.action.start': 'Start',
    'org.bulk.action.stop': 'Stop',
    'org.bulk.action.delete': 'Delete',
    'org.bulk.action.proxy': 'Assign proxy',
    'org.bulk.action.tag': 'Add tag',
    'org.bulk.action.untag': 'Remove tag',
    'org.bulk.action.group': 'Move to group',
    'org.bulk.deleteTitle': 'Delete {n} profiles?',
    'org.bulk.deleteBody': 'Their fingerprints and all browser data (cookies, logins, history) are removed from this machine and cannot be recovered. Running profiles are stopped first.',
    'org.bulk.resultsTitle': '{action} — results',
    'org.bulk.reason.not-found': 'profile not found',
    'org.bulk.reason.running': 'profile is running',
    'org.bulk.reason.error': 'failed',
    'org.bulk.ok': 'ok',
  },
  vi: {
    'org.col.tags': 'Thẻ',
    'org.col.group': 'Nhóm',
    'org.noGroup': 'Không nhóm',
    'org.view': 'Bộ lọc đã lưu',
    'org.viewNone': 'Tất cả profile',
    'org.viewSave': 'Lưu bộ lọc',
    'org.viewSaveTitle': 'Lưu tìm kiếm, bộ lọc và cách sắp xếp hiện tại',
    'org.viewName': 'Tên bộ lọc',
    'org.viewSaved': 'Đã lưu bộ lọc {name}',
    'org.viewDelete': 'Xoá bộ lọc này',
    'org.viewDeleted': 'Đã xoá bộ lọc {name}',
    'org.filter.tag': 'Thẻ',
    'org.filter.group': 'Nhóm',
    'org.filter.os': 'OS',
    'org.filter.anyTag': 'Mọi thẻ',
    'org.filter.anyGroup': 'Mọi nhóm',
    'org.filter.anyOs': 'Mọi OS',
    'org.filter.noGroup': 'Không nhóm',
    'org.filter.clear': 'Bỏ lọc',
    'org.manage': 'Thẻ & nhóm',
    'org.manageTitle': 'Thẻ và nhóm',
    'org.tags': 'Thẻ',
    'org.groups': 'Nhóm',
    'org.newTag': 'Thẻ mới',
    'org.newGroup': 'Nhóm mới',
    'org.tagColor': 'Màu thẻ',
    'org.add': 'Thêm',
    'org.rename': 'Đổi tên',
    'org.confirmDelete': 'Chắc chưa?',
    'org.noTags': 'Chưa có thẻ',
    'org.noGroups': 'Chưa có nhóm',
    'org.deleteTagHint': 'Xoá thẻ sẽ gỡ thẻ khỏi mọi profile.',
    'org.deleteGroupHint': 'Xoá nhóm thì các profile trong nhóm sẽ không còn nhóm.',
    'org.created': 'Đã tạo {name}',
    'org.sort': 'Bấm để sắp xếp',
    'org.selectAll': 'Chọn tất cả profile đang hiện',
    'org.selectRow': 'Chọn profile này (⇧ để chọn vùng)',
    'org.selected': 'Đã chọn {n}',
    'org.clear': 'Bỏ chọn',
    'org.bulk.start': 'Start',
    'org.bulk.stop': 'Stop',
    'org.bulk.delete': 'Xoá',
    'org.bulk.proxy': 'Proxy…',
    'org.bulk.tag': 'Thẻ…',
    'org.bulk.group': 'Chuyển nhóm…',
    'org.bulk.apply': 'Áp dụng',
    'org.bulk.addTag': 'Gắn thẻ',
    'org.bulk.removeTag': 'Gỡ thẻ',
    'org.bulk.noProxy': 'Không proxy (bỏ proxy)',
    'org.bulk.running': 'Đang chạy…',
    'org.bulk.done': '{action}: {ok} xong, {failed} lỗi',
    'org.bulk.allOk': '{action}: {ok} xong',
    'org.bulk.action.start': 'Start',
    'org.bulk.action.stop': 'Stop',
    'org.bulk.action.delete': 'Xoá',
    'org.bulk.action.proxy': 'Gán proxy',
    'org.bulk.action.tag': 'Gắn thẻ',
    'org.bulk.action.untag': 'Gỡ thẻ',
    'org.bulk.action.group': 'Chuyển nhóm',
    'org.bulk.deleteTitle': 'Xoá {n} profile?',
    'org.bulk.deleteBody': 'Fingerprint và toàn bộ dữ liệu browser (cookie, đăng nhập, lịch sử) của các profile sẽ bị xoá khỏi máy, không khôi phục được. Profile đang chạy sẽ được dừng trước.',
    'org.bulk.resultsTitle': '{action} — kết quả',
    'org.bulk.reason.not-found': 'không tìm thấy profile',
    'org.bulk.reason.running': 'profile đang chạy',
    'org.bulk.reason.error': 'lỗi',
    'org.bulk.ok': 'xong',
  },
});

{ // file-local scope: classic scripts share one global scope

CM.org = Vue.reactive({ tags: [], groups: [], views: [], profileTags: {}, profileGroups: {}, loaded: false });

const tagsOf = id => (CM.org.profileTags[id] || []).map(tid => CM.org.tags.find(x => x.id === tid)).filter(Boolean);
const groupOf = id => CM.org.groups.find(g => g.id === CM.org.profileGroups[id]) || null;
// column getters for profiles.js (text, so the table copies as TSV and is searchable)
CM.orgText = {
  tags: p => tagsOf(p.id).map(x => x.name).join(', ') || '—',
  group: p => groupOf(p.id)?.name || '—',
};
const cmp = (a, b) => (typeof a === 'number' && typeof b === 'number') ? a - b : String(a).localeCompare(String(b), undefined, { numeric: true, sensitivity: 'base' });

CM.useOrg = function (ctx) {
  const { reactive, computed } = Vue;
  const of = reactive({ tag: '', group: '', os: '' });
  const sort = reactive({ key: '', dir: 1 });
  const checked = reactive({});
  let lastIdx = null;

  async function load() { Object.assign(CM.org, await api('/api/org'), { loaded: true }); }

  /* ----- filters & sort ----- */
  const filterRows = rows => rows.filter(p =>
    (!of.tag || (CM.org.profileTags[p.id] || []).includes(+of.tag)) &&
    (!of.group || (of.group === 'none' ? !CM.org.profileGroups[p.id] : CM.org.profileGroups[p.id] === +of.group)) &&
    (!of.os || p.os === of.os));
  const sortRows = (rows, cols) => {
    const c = sort.key && cols.find(x => x.h === sort.key);
    if (!c) return rows;
    const val = p => c.s ? c.s(p) : String(c.v(p));
    return [...rows].sort((a, b) => sort.dir * cmp(val(a), val(b)));
  };
  const setSort = c => {
    if (!c.h || c.k === 'check' || c.k === 'reveal') return;
    if (sort.key !== c.h) { sort.key = c.h; sort.dir = 1; }
    else if (sort.dir === 1) sort.dir = -1;
    else { sort.key = ''; sort.dir = 1; }
  };
  const sortMark = c => sort.key === c.h ? (sort.dir === 1 ? ' ▲' : ' ▼') : '';
  const filtering = computed(() => !!(of.tag || of.group || of.os));
  const clearFilters = () => { of.tag = of.group = of.os = ''; };

  /* ----- saved views ----- */
  const query = () => ({ q: ctx.q.value, filter: ctx.filter.value, tag: of.tag, group: of.group, os: of.os, sort: sort.key ? { key: sort.key, dir: sort.dir } : null });
  function applyView(v) {
    const qy = v?.query || {};
    ctx.q.value = qy.q || '';
    ctx.filter.value = qy.filter || 'all';
    // ids of tags/groups deleted since the view was saved are dropped
    of.tag = CM.org.tags.some(x => String(x.id) === String(qy.tag)) ? String(qy.tag) : '';
    of.group = qy.group === 'none' || CM.org.groups.some(x => String(x.id) === String(qy.group)) ? String(qy.group) : '';
    of.os = qy.os || '';
    sort.key = qy.sort?.key || ''; sort.dir = qy.sort?.dir === -1 ? -1 : 1;
  }

  /* ----- checkbox selection (by profile id, survives refreshes) ----- */
  const checkedIds = computed(() => (ctx.profiles() || []).filter(p => checked[p.id]).map(p => p.id));
  const shown = computed(() => ctx.rows());
  const allChecked = computed(() => shown.value.length > 0 && shown.value.every(p => checked[p.id]));
  const someChecked = computed(() => shown.value.some(p => checked[p.id]));
  function toggle(id, e) {
    const rows = ctx.rows(), i = rows.findIndex(p => p.id === id), on = !checked[id];
    if (e?.shiftKey && lastIdx !== null && rows[lastIdx] && i >= 0) {
      for (let k = Math.min(i, lastIdx); k <= Math.max(i, lastIdx); k++) checked[rows[k].id] = on;
    } else checked[id] = on;
    lastIdx = i;
  }
  const toggleAll = () => { const on = !allChecked.value; ctx.rows().forEach(p => { checked[p.id] = on; }); };
  const clearChecked = () => { for (const k of Object.keys(checked)) delete checked[k]; lastIdx = null; };

  return { org: CM.org, of, sort, checked, checkedIds, allChecked, someChecked, loadOrg: load, filterRows, sortRows, setSort, sortMark, filtering, clearFilters, query, applyView, toggle, toggleAll, clearChecked, tagsOf, groupOf };
};

/* ---------- tag chips ---------- */
CM.components.CmOrgTags = {
  props: { id: String },
  setup(props) { return { list: Vue.computed(() => tagsOf(props.id)) }; },
  template: `<span class="inline-flex items-center gap-1">
    <span v-for="x in list" :key="x.id" class="inline-flex items-center gap-1 px-[7px] rounded-full bg-panel2 border border-line text-[11px] leading-[18px]" :title="x.name">
      <span class="size-[7px] rounded-full bg-faint" :style="x.color ? { background: x.color } : null"></span>{{ x.name }}
    </span>
    <span v-if="!list.length" class="text-faint">—</span>
  </span>`,
};

/* ---------- views + filters bar ---------- */
CM.components.CmOrgBar = {
  props: { orgx: Object },
  emits: ['manage'],
  setup(props) {
    const { ref, reactive, nextTick } = Vue;
    const cur = ref('');          // selected view id
    const save = reactive({ open: false, name: '', busy: false });
    const nameInput = ref(null);
    const OSES = [{ k: 'win', t: 'Windows' }, { k: 'mac', t: 'macOS' }, { k: 'lin', t: 'Linux' }];
    const pick = e => {
      cur.value = e.target.value;
      const v = CM.org.views.find(x => String(x.id) === cur.value);
      props.orgx.applyView(v || { query: {} });
    };
    const openSave = () => { save.open = true; save.name = CM.org.views.find(x => String(x.id) === cur.value)?.name || ''; nextTick(() => nameInput.value?.focus()); };
    async function doSave() {
      const name = save.name.trim(); if (!name || save.busy) return;
      save.busy = true;
      try {
        const v = await api('/api/views', { method: 'POST', body: JSON.stringify({ name, query: props.orgx.query() }) });
        await props.orgx.loadOrg();
        cur.value = String(v.id); save.open = false;
        toast(t('org.viewSaved', { name: v.name }));
      } catch (e) { toast(e.message, true); }
      save.busy = false;
    }
    async function del() {
      const v = CM.org.views.find(x => String(x.id) === cur.value); if (!v) return;
      try { await api('/api/views/' + v.id, { method: 'DELETE' }); await props.orgx.loadOrg(); cur.value = ''; toast(t('org.viewDeleted', { name: v.name })); }
      catch (e) { toast(e.message, true); }
    }
    return { cur, save, nameInput, OSES, pick, openSave, doSave, del, CMorg: CM.org };
  },
  template: `<div class="flex flex-wrap items-center gap-2 mb-2.5">
    <select class="input !py-1 !text-[12.5px] max-w-[190px]" :value="cur" @change="pick" :title="t('org.view')">
      <option value="">{{ t('org.viewNone') }}</option>
      <option v-for="v in CMorg.views" :key="v.id" :value="v.id">{{ v.name }}</option>
    </select>
    <button class="btn !py-1 !text-[12.5px]" :title="t('org.viewSaveTitle')" @click="openSave">{{ t('org.viewSave') }}</button>
    <button v-if="cur" class="icon-btn" :title="t('org.viewDelete')" @click="del">🗑</button>
    <span class="w-px h-5 bg-line mx-1"></span>
    <select class="input !py-1 !text-[12.5px]" v-model="orgx.of.tag" :title="t('org.filter.tag')">
      <option value="">{{ t('org.filter.anyTag') }}</option>
      <option v-for="x in CMorg.tags" :key="x.id" :value="String(x.id)">{{ x.name }}</option>
    </select>
    <select class="input !py-1 !text-[12.5px]" v-model="orgx.of.group" :title="t('org.filter.group')">
      <option value="">{{ t('org.filter.anyGroup') }}</option>
      <option value="none">{{ t('org.filter.noGroup') }}</option>
      <option v-for="g in CMorg.groups" :key="g.id" :value="String(g.id)">{{ g.name }}</option>
    </select>
    <select class="input !py-1 !text-[12.5px]" v-model="orgx.of.os" :title="t('org.filter.os')">
      <option value="">{{ t('org.filter.anyOs') }}</option>
      <option v-for="x in OSES" :key="x.k" :value="x.k">{{ x.t }}</option>
    </select>
    <button v-if="orgx.filtering.value" class="icon-btn !text-[12px]" @click="orgx.clearFilters()">✕ {{ t('org.filter.clear') }}</button>
    <div class="flex-1"></div>
    <button class="btn !py-1 !text-[12.5px]" @click="$emit('manage')">{{ t('org.manage') }}</button>

    <cm-dialog :open="save.open" :title="t('org.viewSave')" :busy="save.busy" @close="save.open = false">
      <form id="save-view" @submit.prevent="doSave">
        <cm-field :label="t('org.viewName')"><input ref="nameInput" class="input" v-model="save.name" maxlength="60" autocomplete="off"></cm-field>
      </form>
      <template #footer>
        <button type="button" class="btn" @click="save.open = false">{{ t('common.cancel') }}</button>
        <button type="submit" form="save-view" class="btn btn-start" :disabled="save.busy || !save.name.trim()">{{ t('common.save') }}</button>
      </template>
    </cm-dialog>
  </div>`,
};

/* ---------- bulk toolbar ---------- */
CM.components.CmOrgBulk = {
  props: { orgx: Object, profiles: Array, proxies: Array, providerAvailable: Boolean },
  emits: ['done'],
  setup(props, { emit }) {
    const { ref, reactive, computed } = Vue;
    const busy = ref(''), pick = reactive({ proxy: '__', tag: '', group: '__' });
    const confirmDel = ref(false);
    const results = ref(null);
    const ids = computed(() => props.orgx.checkedIds.value);
    const nameOf = id => props.profiles.find(p => p.id === id)?.name || id;
    async function run(action, extra = {}) {
      if (busy.value || !ids.value.length) return;
      busy.value = action;
      try {
        const r = await api('/api/bulk', { method: 'POST', body: JSON.stringify({ action, profileIds: ids.value, ...extra }) });
        const label = t('org.bulk.action.' + action);
        toast(r.failed ? t('org.bulk.done', { action: label, ok: r.ok, failed: r.failed }) : t('org.bulk.allOk', { action: label, ok: r.ok }), !!r.failed && !r.ok);
        if (r.failed) results.value = { ...r, label };
        if (action === 'delete') props.orgx.clearChecked();
      } catch (e) { toast(e.message, true); }
      busy.value = ''; confirmDel.value = false;
      emit('done');
    }
    const applyProxy = () => { if (pick.proxy !== '__') run('proxy', { proxyId: pick.proxy === '' ? null : +pick.proxy }); };
    const applyGroup = () => { if (pick.group !== '__') run('group', { groupId: pick.group === '' ? null : +pick.group }); };
    const reason = r => t('org.bulk.reason.' + (r.reason || 'error'));
    return { busy, pick, ids, confirmDel, results, run, applyProxy, applyGroup, nameOf, reason, CMorg: CM.org };
  },
  template: `<div class="flex flex-wrap items-center gap-2 mb-2.5 px-3 py-2 rounded-xl border border-accent/40 bg-accent2/50 text-[12.5px]">
    <b>{{ t('org.selected', { n: ids.length }) }}</b>
    <button class="icon-btn" :title="t('org.clear')" @click="orgx.clearChecked()">✕</button>
    <span class="w-px h-5 bg-line mx-1"></span>
    <button class="btn btn-start !min-w-0 !py-1" :disabled="!!busy || !providerAvailable" @click="run('start')"><span v-if="busy === 'start'" class="spin"></span><template v-else>▶</template> {{ t('org.bulk.start') }}</button>
    <button class="btn btn-stop !min-w-0 !py-1" :disabled="!!busy" @click="run('stop')"><span v-if="busy === 'stop'" class="spin"></span><template v-else>■</template> {{ t('org.bulk.stop') }}</button>
    <span class="w-px h-5 bg-line mx-1"></span>
    <select class="input !py-1 !text-[12.5px] max-w-[200px]" v-model="pick.proxy">
      <option value="__" disabled>{{ t('org.bulk.proxy') }}</option>
      <option value="">{{ t('org.bulk.noProxy') }}</option>
      <option v-for="x in proxies" :key="x.id" :value="String(x.id)">#{{ x.id }} {{ x.label }}</option>
    </select>
    <button class="btn !py-1" :disabled="!!busy || pick.proxy === '__'" @click="applyProxy">{{ t('org.bulk.apply') }}</button>
    <select class="input !py-1 !text-[12.5px]" v-model="pick.tag">
      <option value="" disabled>{{ t('org.bulk.tag') }}</option>
      <option v-for="x in CMorg.tags" :key="x.id" :value="String(x.id)">{{ x.name }}</option>
    </select>
    <button class="btn !py-1" :disabled="!!busy || !pick.tag" @click="run('tag', { tagId: +pick.tag })">＋ {{ t('org.bulk.addTag') }}</button>
    <button class="btn !py-1" :disabled="!!busy || !pick.tag" @click="run('untag', { tagId: +pick.tag })">－ {{ t('org.bulk.removeTag') }}</button>
    <select class="input !py-1 !text-[12.5px]" v-model="pick.group">
      <option value="__" disabled>{{ t('org.bulk.group') }}</option>
      <option value="">{{ t('org.noGroup') }}</option>
      <option v-for="g in CMorg.groups" :key="g.id" :value="String(g.id)">{{ g.name }}</option>
    </select>
    <button class="btn !py-1" :disabled="!!busy || pick.group === '__'" @click="applyGroup">{{ t('org.bulk.apply') }}</button>
    <div class="flex-1"></div>
    <button class="btn btn-stop !min-w-0 !py-1" :disabled="!!busy" @click="confirmDel = true">🗑 {{ t('org.bulk.delete') }}</button>

    <cm-dialog :open="confirmDel" :title="t('org.bulk.deleteTitle', { n: ids.length })" :busy="busy === 'delete'" @close="confirmDel = false">
      <p class="m-0 text-[13px] text-muted">{{ t('org.bulk.deleteBody') }}</p>
      <template #footer>
        <button class="btn" :disabled="busy === 'delete'" @click="confirmDel = false">{{ t('common.cancel') }}</button>
        <button class="btn btn-stop" :disabled="busy === 'delete'" @click="run('delete')"><span v-if="busy === 'delete'" class="spin"></span><template v-else>{{ t('common.delete') }}</template></button>
      </template>
    </cm-dialog>

    <cm-dialog :open="!!results" :title="results ? t('org.bulk.resultsTitle', { action: results.label }) : ''" :width="460" @close="results = null">
      <div v-if="results" class="max-h-[50vh] overflow-auto text-[13px] flex flex-col gap-1">
        <div v-for="r in results.results" :key="r.profileId" class="flex gap-2">
          <span :class="r.ok ? 'text-ok' : 'text-bad'">{{ r.ok ? '✓' : '✗' }}</span>
          <span class="font-semibold">{{ nameOf(r.profileId) }}</span>
          <span class="text-muted truncate" :title="r.error">{{ r.ok ? t('org.bulk.ok') : (r.error && r.error !== r.reason ? r.error : reason(r)) }}</span>
        </div>
      </div>
      <template #footer><button class="btn" @click="results = null">{{ t('common.close') }}</button></template>
    </cm-dialog>
  </div>`,
};

/* ---------- manage tags & groups ---------- */
CM.components.CmOrgManage = {
  props: { open: Boolean, orgx: Object },
  emits: ['close', 'changed'],
  setup(props, { emit }) {
    const { reactive, ref } = Vue;
    const nt = reactive({ name: '', color: '#6366f1', useColor: true }), ng = reactive({ name: '' });
    const arm = ref('');          // two-step delete: "tag:3" / "group:2"
    const editing = reactive({ id: null, name: '' });
    async function call(path, method, body) {
      try { const r = await api(path, { method, body: body && JSON.stringify(body) }); await props.orgx.loadOrg(); emit('changed'); return r; }
      catch (e) { toast(e.message, true); }
    }
    async function addTag() {
      if (!nt.name.trim()) return;
      if (await call('/api/tags', 'POST', { name: nt.name, color: nt.useColor ? nt.color : null })) nt.name = '';
    }
    async function addGroup() {
      if (!ng.name.trim()) return;
      if (await call('/api/groups', 'POST', { name: ng.name })) ng.name = '';
    }
    async function remove(kind, id) {
      const k = kind + ':' + id;
      if (arm.value !== k) { arm.value = k; return; }
      arm.value = '';
      await call(`/api/${kind === 'tag' ? 'tags' : 'groups'}/${id}`, 'DELETE');
    }
    async function rename(g) {
      if (editing.name.trim() && editing.name.trim() !== g.name) await call('/api/groups/' + g.id, 'PUT', { name: editing.name });
      editing.id = null;
    }
    return { nt, ng, arm, editing, addTag, addGroup, remove, rename, CMorg: CM.org };
  },
  template: `<cm-dialog :open="open" :title="t('org.manageTitle')" :width="460" @close="$emit('close')">
    <div class="flex flex-col gap-4 text-[13px]">
      <section class="flex flex-col gap-2">
        <h3 class="m-0 text-[12px] uppercase tracking-wider text-faint">{{ t('org.tags') }}</h3>
        <div v-if="!CMorg.tags.length" class="text-muted">{{ t('org.noTags') }}</div>
        <div v-for="x in CMorg.tags" :key="x.id" class="flex items-center gap-2">
          <span class="size-3 rounded-full bg-faint" :style="x.color ? { background: x.color } : null"></span>
          <span class="flex-1 truncate">{{ x.name }}</span>
          <button class="btn !py-0.5 !text-[12px]" :class="arm === 'tag:' + x.id ? '!bg-bad !text-white' : ''" :title="t('org.deleteTagHint')" @click="remove('tag', x.id)">{{ arm === 'tag:' + x.id ? t('org.confirmDelete') : t('common.delete') }}</button>
        </div>
        <form class="flex items-center gap-2" @submit.prevent="addTag">
          <input type="color" v-model="nt.color" class="w-8 h-8 p-0.5 border border-line rounded-lg bg-panel cursor-pointer" :title="t('org.tagColor')">
          <input class="input flex-1" v-model="nt.name" maxlength="60" :placeholder="t('org.newTag')" autocomplete="off">
          <button class="btn" :disabled="!nt.name.trim()">{{ t('org.add') }}</button>
        </form>
      </section>
      <section class="flex flex-col gap-2">
        <h3 class="m-0 text-[12px] uppercase tracking-wider text-faint">{{ t('org.groups') }}</h3>
        <div v-if="!CMorg.groups.length" class="text-muted">{{ t('org.noGroups') }}</div>
        <div v-for="g in CMorg.groups" :key="g.id" class="flex items-center gap-2">
          <input v-if="editing.id === g.id" class="input flex-1 !py-1" v-model="editing.name" maxlength="60" @keydown.enter="rename(g)" @keydown.esc="editing.id = null" @blur="rename(g)" autofocus>
          <span v-else class="flex-1 truncate">{{ g.name }}</span>
          <button class="btn !py-0.5 !text-[12px]" @click="editing.id = g.id; editing.name = g.name">{{ t('org.rename') }}</button>
          <button class="btn !py-0.5 !text-[12px]" :class="arm === 'group:' + g.id ? '!bg-bad !text-white' : ''" :title="t('org.deleteGroupHint')" @click="remove('group', g.id)">{{ arm === 'group:' + g.id ? t('org.confirmDelete') : t('common.delete') }}</button>
        </div>
        <form class="flex items-center gap-2" @submit.prevent="addGroup">
          <input class="input flex-1" v-model="ng.name" maxlength="60" :placeholder="t('org.newGroup')" autocomplete="off">
          <button class="btn" :disabled="!ng.name.trim()">{{ t('org.add') }}</button>
        </form>
      </section>
    </div>
    <template #footer><button class="btn" @click="$emit('close')">{{ t('common.close') }}</button></template>
  </cm-dialog>`,
};

} // end file-local scope
