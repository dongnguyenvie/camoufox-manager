/* Templates page (phase 9): reusable profile settings + the Firefox extension library.
   - Template = named settings (OS, screen, window, locale, timezone, WebRTC, fonts, humanize, canvas,
     start URL, default proxy, tags, group, extensions). It never pins a machine: every profile made
     from it gets its own new machine.
   - Extensions: upload .xpi (a zip with manifest.json and a Gecko add-on id), list, delete, assign to
     profiles in bulk. They are installed into the profile at the next Start.
   Also registers <cm-tpl-dialogs> used by the profile table: "extensions of this profile" and
   "save this profile's settings as a template". */

I18N.add({
  en: {
    'tpl.tab.templates': 'Templates',
    'tpl.tab.extensions': 'Extensions',
    'tpl.intro': 'A template keeps reusable settings, never a machine: every profile made from it pins its own new hardware fingerprint.',
    'tpl.new': 'New template',
    'tpl.empty': 'No templates yet. Create one here, or use the star button on a profile to save its settings.',
    'tpl.col.name': 'Name',
    'tpl.col.os': 'OS',
    'tpl.col.settings': 'Settings',
    'tpl.col.proxy': 'Default proxy',
    'tpl.col.org': 'Tags / group',
    'tpl.col.extensions': 'Extensions',
    'tpl.createProfiles': 'Create profiles',
    'tpl.edit': 'Edit',
    'tpl.editTitle': 'Edit template',
    'tpl.newTitle': 'New template',
    'tpl.saved': 'Template saved',
    'tpl.deleted': 'Template deleted',
    'tpl.deleteTitle': 'Delete template?',
    'tpl.deleteBody': '{name} is deleted. Profiles already made from it are not changed.',
    'tpl.sec.settings': 'Settings',
    'tpl.sec.links': 'Proxy, tags and extensions',
    'tpl.proxy': 'Default proxy',
    'tpl.noProxy': 'No proxy (machine IP)',
    'tpl.group': 'Group',
    'tpl.noGroup': 'No group',
    'tpl.tags': 'Tags',
    'tpl.noTags': 'No tags yet. Create tags on the Profiles page.',
    'tpl.extensions': 'Extensions',
    'tpl.noExtensions': 'No extensions in the library yet (Extensions tab).',
    'tpl.fontsHint': 'One font per line, added to the machine font list.',
    'tpl.createTitle': 'Create profiles from {name}',
    'tpl.count': 'How many',
    'tpl.pattern': 'Name pattern',
    'tpl.patternHint': 'Use {n} for the number, e.g. Shop {n}. For one profile this is its name.',
    'tpl.start': 'First number',
    'tpl.createDo': 'Create {n}',
    'tpl.createdN': 'Created {n} profiles, each with its own machine',
    'tpl.createdPartial': 'Created {n} profiles, then stopped: {msg}',
    'tpl.creating': 'Generating machines…',
    'tpl.extPageIntro': 'Firefox add-ons (.xpi) stored on this machine. Assign them to profiles or templates; they are installed when the profile starts. The add-on needs an id in its manifest (browser_specific_settings.gecko.id); Camoufox accepts unsigned add-ons.',
    'tpl.ext.upload': 'Upload .xpi',
    'tpl.ext.uploaded': 'Added {name}',
    'tpl.ext.empty': 'No extensions yet.',
    'tpl.ext.col.name': 'Name',
    'tpl.ext.col.id': 'Add-on id',
    'tpl.ext.col.version': 'Version',
    'tpl.ext.col.used': 'Used by',
    'tpl.ext.usedBy': '{p} profiles, {t} templates',
    'tpl.ext.assign': 'Assign',
    'tpl.ext.deleteTitle': 'Delete extension?',
    'tpl.ext.deleteBody': '{name} is removed from the library, from {p} profiles and {t} templates. Running browsers keep it until their next start.',
    'tpl.ext.deleted': 'Extension deleted',
    'tpl.ext.assignTitle': 'Assign {name}',
    'tpl.ext.assignHint': 'Applies at the next Start of each profile.',
    'tpl.ext.add': 'Add to selected',
    'tpl.ext.remove': 'Remove from selected',
    'tpl.ext.assigned': 'Updated {n} profiles',
    'tpl.ext.noProfiles': 'No profiles yet.',
    'tpl.ext.limits': 'Limits: installed at launch (restart to change); WebExtension APIs Camoufox lacks do nothing.',
    'tpl.prof.extTitle': 'Extensions of {name}',
    'tpl.prof.extHint': 'Installed into the profile at the next Start.',
    'tpl.prof.extSaved': 'Extensions saved — apply at next Start',
    'tpl.prof.saveTitle': 'Save {name} as a template',
    'tpl.prof.saveHint': 'Copies the settings, proxy, tags, group and extensions. The machine is not copied: profiles made from the template get their own.',
    'tpl.prof.saveDo': 'Save template',
    'tpl.prof.starTitle': 'Save as template',
    'tpl.prof.extBtn': 'Extensions',
  },
  vi: {
    'tpl.tab.templates': 'Template',
    'tpl.tab.extensions': 'Extension',
    'tpl.intro': 'Template chỉ giữ thiết lập dùng lại, không giữ máy: mỗi profile tạo từ nó tự ghim một fingerprint phần cứng mới.',
    'tpl.new': 'Template mới',
    'tpl.empty': 'Chưa có template. Tạo ở đây, hoặc bấm nút ngôi sao ở một profile để lưu thiết lập của nó.',
    'tpl.col.name': 'Tên',
    'tpl.col.os': 'HĐH',
    'tpl.col.settings': 'Thiết lập',
    'tpl.col.proxy': 'Proxy mặc định',
    'tpl.col.org': 'Tag / nhóm',
    'tpl.col.extensions': 'Extension',
    'tpl.createProfiles': 'Tạo profile',
    'tpl.edit': 'Sửa',
    'tpl.editTitle': 'Sửa template',
    'tpl.newTitle': 'Template mới',
    'tpl.saved': 'Đã lưu template',
    'tpl.deleted': 'Đã xoá template',
    'tpl.deleteTitle': 'Xoá template?',
    'tpl.deleteBody': '{name} sẽ bị xoá. Các profile đã tạo từ nó không đổi.',
    'tpl.sec.settings': 'Thiết lập',
    'tpl.sec.links': 'Proxy, tag và extension',
    'tpl.proxy': 'Proxy mặc định',
    'tpl.noProxy': 'Không proxy (IP máy)',
    'tpl.group': 'Nhóm',
    'tpl.noGroup': 'Không nhóm',
    'tpl.tags': 'Tag',
    'tpl.noTags': 'Chưa có tag. Tạo tag ở trang Profiles.',
    'tpl.extensions': 'Extension',
    'tpl.noExtensions': 'Thư viện chưa có extension nào (tab Extension).',
    'tpl.fontsHint': 'Mỗi dòng một font, thêm vào danh sách font của máy.',
    'tpl.createTitle': 'Tạo profile từ {name}',
    'tpl.count': 'Số lượng',
    'tpl.pattern': 'Mẫu tên',
    'tpl.patternHint': 'Dùng {n} cho số thứ tự, ví dụ Shop {n}. Với một profile thì đây là tên của nó.',
    'tpl.start': 'Số bắt đầu',
    'tpl.createDo': 'Tạo {n}',
    'tpl.createdN': 'Đã tạo {n} profile, mỗi cái một máy riêng',
    'tpl.createdPartial': 'Đã tạo {n} profile rồi dừng: {msg}',
    'tpl.creating': 'Đang tạo máy…',
    'tpl.extPageIntro': 'Add-on Firefox (.xpi) lưu trên máy này. Gán cho profile hoặc template; chúng được cài khi profile Start. Add-on cần có id trong manifest (browser_specific_settings.gecko.id); Camoufox nhận add-on chưa ký.',
    'tpl.ext.upload': 'Tải lên .xpi',
    'tpl.ext.uploaded': 'Đã thêm {name}',
    'tpl.ext.empty': 'Chưa có extension nào.',
    'tpl.ext.col.name': 'Tên',
    'tpl.ext.col.id': 'Id add-on',
    'tpl.ext.col.version': 'Phiên bản',
    'tpl.ext.col.used': 'Đang dùng bởi',
    'tpl.ext.usedBy': '{p} profile, {t} template',
    'tpl.ext.assign': 'Gán',
    'tpl.ext.deleteTitle': 'Xoá extension?',
    'tpl.ext.deleteBody': '{name} bị xoá khỏi thư viện, khỏi {p} profile và {t} template. Browser đang chạy giữ nó đến lần Start sau.',
    'tpl.ext.deleted': 'Đã xoá extension',
    'tpl.ext.assignTitle': 'Gán {name}',
    'tpl.ext.assignHint': 'Áp dụng ở lần Start sau của mỗi profile.',
    'tpl.ext.add': 'Thêm vào các profile đã chọn',
    'tpl.ext.remove': 'Gỡ khỏi các profile đã chọn',
    'tpl.ext.assigned': 'Đã cập nhật {n} profile',
    'tpl.ext.noProfiles': 'Chưa có profile nào.',
    'tpl.ext.limits': 'Giới hạn: cài lúc khởi động (đổi thì cần mở lại); API WebExtension mà Camoufox không có sẽ không chạy.',
    'tpl.prof.extTitle': 'Extension của {name}',
    'tpl.prof.extHint': 'Được cài vào profile ở lần Start sau.',
    'tpl.prof.extSaved': 'Đã lưu extension — áp dụng ở lần Start sau',
    'tpl.prof.saveTitle': 'Lưu {name} thành template',
    'tpl.prof.saveHint': 'Sao chép thiết lập, proxy, tag, nhóm và extension. Máy không được sao chép: profile tạo từ template có máy riêng.',
    'tpl.prof.saveDo': 'Lưu template',
    'tpl.prof.starTitle': 'Lưu thành template',
    'tpl.prof.extBtn': 'Extension',
  },
});

{ // file-local scope: classic scripts share one global scope
const TPL_OSES = [{ k: 'win', t: 'Windows' }, { k: 'mac', t: 'macOS' }, { k: 'lin', t: 'Linux' }];
const osName = k => TPL_OSES.find(o => o.k === k)?.t || k;
const WEBRTC = ['replace', 'real', 'forward', 'none'];
const blank = () => ({ id: 0, name: '', os: 'win', startUrl: '', screen: '', window: '', locale: '', timezone: '', webrtc: 'replace', canvas: 'stable', fonts: '', humanize: false, proxyId: null, groupId: null, tags: [], extensions: [] });

function toForm(tp) {
  const s = tp.settings || {};
  return { id: tp.id, name: tp.name, os: tp.os, startUrl: s.startUrl || '', screen: s.screen || '', window: s.window || '', locale: s.locale || '', timezone: s.timezone || '',
    webrtc: s.webrtc || 'replace', canvas: s.canvas || 'stable', fonts: (s.fonts || []).join('\n'), humanize: !!s.humanize, proxyId: tp.proxyId ?? null, groupId: tp.groupId ?? null,
    tags: [...(tp.tags || [])], extensions: [...(tp.extensions || [])] };
}
function toBody(f) {
  const v = x => (x || '').trim() || null;
  return { id: f.id, name: f.name.trim(), os: f.os, proxyId: f.proxyId, groupId: f.groupId, tags: f.tags, extensions: f.extensions,
    settings: { startUrl: v(f.startUrl), screen: v(f.screen), window: v(f.window), locale: v(f.locale), timezone: v(f.timezone), webrtc: f.webrtc, canvas: f.canvas,
      fonts: f.fonts.split('\n').map(x => x.trim()).filter(Boolean), humanize: f.humanize ? true : null } };
}
const toggle = (arr, id) => { const i = arr.indexOf(id); if (i >= 0) arr.splice(i, 1); else arr.push(id); };

CM.pages.templates = {
  setup() {
    const { ref, reactive, computed, onMounted } = Vue;
    const tab = ref('templates');
    const loaded = ref(false);
    const templates = ref([]), extensions = ref([]), proxies = ref([]), tags = ref([]), groups = ref([]), profiles = ref([]);

    async function load() {
      try {
        const [tp, ex, px, og, ov] = await Promise.all([api('/api/templates'), api('/api/extensions'), api('/api/proxies'), api('/api/org'), api('/api/overview')]);
        templates.value = tp; extensions.value = ex; proxies.value = px.proxies || []; tags.value = og.tags || []; groups.value = og.groups || []; profiles.value = ov.profiles || [];
      } catch (e) { toast(t('common.loadFailed', { msg: e.message }), true); }
      loaded.value = true;
    }
    onMounted(load);

    const proxyName = id => { const p = proxies.value.find(x => x.id === id); return p ? (p.name && p.name !== p.label ? p.name : p.label) : '#' + id; };
    const tagOf = id => tags.value.find(x => x.id === id);
    const groupName = id => groups.value.find(x => x.id === id)?.name;
    const extName = id => extensions.value.find(x => x.id === id)?.name || '#' + id;
    const summary = tp => {
      const s = tp.settings || {}, out = [];
      if (s.startUrl) out.push(s.startUrl);
      if (s.screen) out.push(s.screen);
      if (s.window) out.push('⬚ ' + s.window);
      if (s.locale) out.push(s.locale);
      if (s.timezone) out.push(s.timezone);
      if (s.webrtc && s.webrtc !== 'replace') out.push('WebRTC ' + s.webrtc);
      if (s.fonts?.length) out.push('+' + s.fonts.length + ' fonts');
      if (s.humanize) out.push('humanize');
      return out;
    };

    // ---- template form ----
    const form = reactive({ open: false, busy: false, f: blank() });
    const openForm = tp => { form.f = tp ? toForm(tp) : blank(); form.open = true; };
    async function saveForm() {
      if (!form.f.name.trim() || form.busy) return;
      form.busy = true;
      try {
        const body = JSON.stringify(toBody(form.f));
        if (form.f.id) await api('/api/templates/' + form.f.id, { method: 'PUT', body });
        else await api('/api/templates', { method: 'POST', body });
        toast(t('tpl.saved')); form.open = false; load();
      } catch (e) { toast(e.message, true); }
      form.busy = false;
    }
    const del = reactive({ tp: null, busy: false });
    async function doDelete() {
      del.busy = true;
      try { await api('/api/templates/' + del.tp.id, { method: 'DELETE' }); toast(t('tpl.deleted')); del.tp = null; load(); }
      catch (e) { toast(e.message, true); }
      del.busy = false;
    }

    // ---- create profiles ----
    const mk = reactive({ tp: null, busy: false, count: 1, pattern: '', start: 1 });
    const openMake = tp => { Object.assign(mk, { tp, busy: false, count: 1, pattern: tp.name, start: 1 }); };
    async function doMake() {
      mk.busy = true;
      try {
        const body = { count: mk.count };
        if (mk.count > 1) { body.pattern = mk.pattern; body.start = mk.start; } else body.name = mk.pattern;
        const r = await api(`/api/templates/${mk.tp.id}/create`, { method: 'POST', body: JSON.stringify(body) });
        if (r.error) toast(t('tpl.createdPartial', { n: r.created.length, msg: r.error }), true);
        else toast(t('tpl.createdN', { n: r.created.length }));
        mk.tp = null; load();
      } catch (e) { toast(e.message, true); }
      mk.busy = false;
    }

    // ---- extension library ----
    const fileInput = ref(null);
    const uploading = ref(false);
    async function upload(ev) {
      const file = ev.target.files[0]; ev.target.value = '';
      if (!file) return;
      uploading.value = true;
      try {
        const r = await fetch('/api/extensions', { method: 'POST', body: file, headers: { 'content-type': 'application/octet-stream' } });
        const j = await r.json().catch(() => ({}));
        if (!r.ok) throw new Error(j.error || r.statusText);
        toast(t('tpl.ext.uploaded', { name: j.name })); load();
      } catch (e) { toast(e.message, true); }
      uploading.value = false;
    }
    const xdel = reactive({ x: null, busy: false });
    async function doDeleteExt() {
      xdel.busy = true;
      try { await api('/api/extensions/' + xdel.x.id, { method: 'DELETE' }); toast(t('tpl.ext.deleted')); xdel.x = null; load(); }
      catch (e) { toast(e.message, true); }
      xdel.busy = false;
    }
    const asg = reactive({ x: null, busy: false, picked: [] });
    const openAssign = x => { asg.x = x; asg.picked = []; asg.busy = false; };
    async function doAssign(mode) {
      asg.busy = true;
      try {
        const r = await api('/api/extensions/assign', { method: 'POST', body: JSON.stringify({ profiles: asg.picked, extensions: [asg.x.id], mode }) });
        toast(t('tpl.ext.assigned', { n: r.profiles })); asg.x = null; load();
      } catch (e) { toast(e.message, true); }
      asg.busy = false;
    }

    const tplCols = computed(() => [
      { key: 'name', label: t('tpl.col.name') }, { key: 'os', label: t('tpl.col.os') }, { key: 'settings', label: t('tpl.col.settings') },
      { key: 'proxy', label: t('tpl.col.proxy') }, { key: 'org', label: t('tpl.col.org') }, { key: 'extensions', label: t('tpl.col.extensions') },
      { key: 'actions', label: '', cls: 'text-right whitespace-nowrap', hcls: 'text-right' },
    ]);
    const extCols = computed(() => [
      { key: 'name', label: t('tpl.ext.col.name') }, { key: 'addonId', label: t('tpl.ext.col.id'), cls: 'font-mono text-[12px]' },
      { key: 'version', label: t('tpl.ext.col.version') }, { key: 'used', label: t('tpl.ext.col.used') },
      { key: 'actions', label: '', cls: 'text-right whitespace-nowrap', hcls: 'text-right' },
    ]);

    return { TPL_OSES, WEBRTC, osName, tab, loaded, templates, extensions, proxies, tags, groups, profiles, proxyName, tagOf, groupName, extName, summary, toggle,
      form, openForm, saveForm, del, doDelete, mk, openMake, doMake, fileInput, uploading, upload, xdel, doDeleteExt, asg, openAssign, doAssign, tplCols, extCols };
  },
  template: `<cm-page :title="t('nav.templates')">
    <template #tools>
      <div class="inline-flex gap-1">
        <button class="btn" :class="tab === 'templates' ? '!bg-accent2 !text-accent' : ''" @click="tab = 'templates'">{{ t('tpl.tab.templates') }} <span class="text-faint">{{ templates.length }}</span></button>
        <button class="btn" :class="tab === 'extensions' ? '!bg-accent2 !text-accent' : ''" @click="tab = 'extensions'">{{ t('tpl.tab.extensions') }} <span class="text-faint">{{ extensions.length }}</span></button>
      </div>
    </template>
    <template #actions>
      <button v-if="tab === 'templates'" class="btn btn-start" @click="openForm()"><cm-icon name="plus" :size="14"/> {{ t('tpl.new') }}</button>
      <template v-else>
        <input ref="fileInput" type="file" accept=".xpi,application/x-xpinstall,application/zip" class="hidden" @change="upload">
        <button class="btn btn-start" :disabled="uploading" @click="fileInput.click()"><span v-if="uploading" class="spin"></span><cm-icon v-else name="plus" :size="14"/> {{ t('tpl.ext.upload') }}</button>
      </template>
    </template>

    <template v-if="tab === 'templates'">
      <p class="m-0 mb-3 text-[13px] text-muted">{{ t('tpl.intro') }}</p>
      <div class="min-h-0 flex flex-col bg-panel border border-line rounded-xl shadow-card">
        <cm-table :columns="tplCols" :rows="templates" :loading="!loaded" :empty="t('tpl.empty')">
          <template #cell-name="{ row }"><div class="font-semibold">{{ row.name }}</div></template>
          <template #cell-os="{ row }">{{ osName(row.os) }}</template>
          <template #cell-settings="{ row }">
            <div class="flex flex-wrap gap-1"><span v-for="s in summary(row)" :key="s" class="chip !py-[2px]">{{ s }}</span><span v-if="!summary(row).length" class="text-faint">—</span></div>
          </template>
          <template #cell-proxy="{ row }">{{ row.proxyId ? proxyName(row.proxyId) : '—' }}</template>
          <template #cell-org="{ row }">
            <div class="flex flex-wrap gap-1">
              <span v-if="row.groupId" class="chip !py-[2px]">▣ {{ groupName(row.groupId) }}</span>
              <span v-for="id in row.tags" :key="id" class="chip !py-[2px]">{{ tagOf(id)?.name || '#' + id }}</span>
              <span v-if="!row.groupId && !row.tags.length" class="text-faint">—</span>
            </div>
          </template>
          <template #cell-extensions="{ row }">
            <div class="flex flex-wrap gap-1"><span v-for="id in row.extensions" :key="id" class="chip !py-[2px]">{{ extName(id) }}</span><span v-if="!row.extensions.length" class="text-faint">—</span></div>
          </template>
          <template #cell-actions="{ row }">
            <div class="inline-flex gap-1.5">
              <button class="btn" @click="openMake(row)">{{ t('tpl.createProfiles') }}</button>
              <button class="btn" @click="openForm(row)">{{ t('tpl.edit') }}</button>
              <button class="btn" @click="del.tp = row">{{ t('common.delete') }}</button>
            </div>
          </template>
        </cm-table>
      </div>
    </template>

    <template v-else>
      <p class="m-0 mb-1 text-[13px] text-muted">{{ t('tpl.extPageIntro') }}</p>
      <p class="m-0 mb-3 text-[12px] text-faint">{{ t('tpl.ext.limits') }}</p>
      <div class="min-h-0 flex flex-col bg-panel border border-line rounded-xl shadow-card">
        <cm-table :columns="extCols" :rows="extensions" :loading="!loaded" :empty="t('tpl.ext.empty')">
          <template #cell-name="{ row }"><div class="font-semibold">{{ row.name }}</div></template>
          <template #cell-version="{ row }">{{ row.version || '—' }}</template>
          <template #cell-used="{ row }">{{ t('tpl.ext.usedBy', { p: row.profiles.length, t: row.templates.length }) }}</template>
          <template #cell-actions="{ row }">
            <div class="inline-flex gap-1.5">
              <button class="btn" @click="openAssign(row)">{{ t('tpl.ext.assign') }}</button>
              <button class="btn" @click="xdel.x = row">{{ t('common.delete') }}</button>
            </div>
          </template>
        </cm-table>
      </div>
    </template>

    <cm-dialog :open="form.open" :title="form.f.id ? t('tpl.editTitle') : t('tpl.newTitle')" :busy="form.busy" :width="680" @close="form.open = false">
      <form class="flex flex-col gap-3 max-h-[70vh] overflow-auto pr-1" @submit.prevent="saveForm">
        <div class="grid grid-cols-2 gap-3 max-sm:grid-cols-1">
          <cm-field :label="t('profileform.name')"><input class="input" v-model="form.f.name" maxlength="80" autocomplete="off"></cm-field>
          <cm-field :label="t('profileform.os')"><select class="input" v-model="form.f.os"><option v-for="x in TPL_OSES" :key="x.k" :value="x.k">{{ x.t }}</option></select></cm-field>
          <cm-field :label="t('profileform.startUrl')"><input class="input" v-model="form.f.startUrl" placeholder="https://" autocomplete="off"></cm-field>
          <cm-field :label="t('profileform.locale')"><input class="input" v-model="form.f.locale" placeholder="de-DE" autocomplete="off"></cm-field>
          <cm-field :label="t('profileform.screen')"><input class="input" v-model="form.f.screen" placeholder="1920x1080" autocomplete="off"></cm-field>
          <cm-field :label="t('profileform.window')"><input class="input" v-model="form.f.window" placeholder="1280x800" autocomplete="off"></cm-field>
          <cm-field :label="t('profileform.timezone')"><input class="input" v-model="form.f.timezone" placeholder="Europe/Berlin" autocomplete="off"></cm-field>
          <cm-field :label="t('profileform.webrtc')"><select class="input" v-model="form.f.webrtc"><option v-for="m in WEBRTC" :key="m" :value="m">{{ t('profileform.webrtc.' + m) }}</option></select></cm-field>
          <cm-field :label="t('profileform.canvas')">
            <select class="input" v-model="form.f.canvas">
              <option value="stable">{{ t('profileform.canvas.stable') }}</option><option value="native">{{ t('profileform.canvas.native') }}</option><option value="random">{{ t('profileform.canvas.random') }}</option>
            </select>
          </cm-field>
          <cm-field :label="t('tpl.proxy')">
            <select class="input" v-model="form.f.proxyId"><option :value="null">{{ t('tpl.noProxy') }}</option><option v-for="p in proxies" :key="p.id" :value="p.id">#{{ p.id }} {{ p.label }}</option></select>
          </cm-field>
          <cm-field :label="t('profileform.fonts')" :hint="t('tpl.fontsHint')"><textarea class="input" rows="3" v-model="form.f.fonts"></textarea></cm-field>
          <cm-field :label="t('tpl.group')">
            <select class="input" v-model="form.f.groupId"><option :value="null">{{ t('tpl.noGroup') }}</option><option v-for="g in groups" :key="g.id" :value="g.id">{{ g.name }}</option></select>
          </cm-field>
        </div>
        <cm-switch v-model="form.f.humanize" :label="t('profileform.humanize')"/>
        <div class="flex flex-col gap-1 text-xs text-muted">
          <span>{{ t('tpl.tags') }}</span>
          <div class="flex flex-wrap gap-1.5">
            <button v-for="g in tags" :key="g.id" type="button" class="chip cursor-pointer border-0" :class="form.f.tags.includes(g.id) ? '!bg-accent !text-white' : ''" @click="toggle(form.f.tags, g.id)">{{ g.name }}</button>
            <span v-if="!tags.length" class="text-faint">{{ t('tpl.noTags') }}</span>
          </div>
        </div>
        <div class="flex flex-col gap-1 text-xs text-muted">
          <span>{{ t('tpl.extensions') }}</span>
          <div class="flex flex-wrap gap-1.5">
            <button v-for="x in extensions" :key="x.id" type="button" class="chip cursor-pointer border-0" :class="form.f.extensions.includes(x.id) ? '!bg-accent !text-white' : ''" @click="toggle(form.f.extensions, x.id)">{{ x.name }}</button>
            <span v-if="!extensions.length" class="text-faint">{{ t('tpl.noExtensions') }}</span>
          </div>
        </div>
        <button type="submit" class="hidden"></button>
      </form>
      <template #footer>
        <button class="btn" :disabled="form.busy" @click="form.open = false">{{ t('common.cancel') }}</button>
        <button class="btn btn-start" :disabled="form.busy || !form.f.name.trim()" @click="saveForm"><span v-if="form.busy" class="spin"></span><template v-else>{{ t('common.save') }}</template></button>
      </template>
    </cm-dialog>

    <cm-dialog :open="!!del.tp" :title="t('tpl.deleteTitle')" :busy="del.busy" @close="del.tp = null">
      <p class="m-0 text-[13px] text-muted">{{ t('tpl.deleteBody', { name: del.tp?.name }) }}</p>
      <template #footer>
        <button class="btn" :disabled="del.busy" @click="del.tp = null">{{ t('common.cancel') }}</button>
        <button class="btn btn-stop" :disabled="del.busy" @click="doDelete">{{ t('common.delete') }}</button>
      </template>
    </cm-dialog>

    <cm-dialog :open="!!mk.tp" :title="t('tpl.createTitle', { name: mk.tp?.name })" :busy="mk.busy" @close="mk.tp = null">
      <div class="grid grid-cols-2 gap-3">
        <cm-field :label="t('tpl.count')"><input class="input" type="number" min="1" max="100" v-model.number="mk.count"></cm-field>
        <cm-field v-if="mk.count > 1" :label="t('tpl.start')"><input class="input" type="number" min="0" v-model.number="mk.start"></cm-field>
      </div>
      <cm-field :label="t('tpl.pattern')" :hint="t('tpl.patternHint')"><input class="input" v-model="mk.pattern" maxlength="80" autocomplete="off"></cm-field>
      <p v-if="mk.busy" class="m-0 text-[13px] text-muted"><span class="spin"></span> {{ t('tpl.creating') }}</p>
      <template #footer>
        <button class="btn" :disabled="mk.busy" @click="mk.tp = null">{{ t('common.cancel') }}</button>
        <button class="btn btn-start" :disabled="mk.busy || !mk.count || (mk.count > 1 && !mk.pattern.includes('{n}'))" @click="doMake">{{ t('tpl.createDo', { n: mk.count || 0 }) }}</button>
      </template>
    </cm-dialog>

    <cm-dialog :open="!!xdel.x" :title="t('tpl.ext.deleteTitle')" :busy="xdel.busy" @close="xdel.x = null">
      <p class="m-0 text-[13px] text-muted">{{ t('tpl.ext.deleteBody', { name: xdel.x?.name, p: xdel.x?.profiles.length, t: xdel.x?.templates.length }) }}</p>
      <template #footer>
        <button class="btn" :disabled="xdel.busy" @click="xdel.x = null">{{ t('common.cancel') }}</button>
        <button class="btn btn-stop" :disabled="xdel.busy" @click="doDeleteExt">{{ t('common.delete') }}</button>
      </template>
    </cm-dialog>

    <cm-dialog :open="!!asg.x" :title="t('tpl.ext.assignTitle', { name: asg.x?.name })" :busy="asg.busy" :width="460" @close="asg.x = null">
      <p class="m-0 text-[12.5px] text-muted">{{ t('tpl.ext.assignHint') }}</p>
      <div class="max-h-[40vh] overflow-auto flex flex-col gap-1">
        <label v-for="p in profiles" :key="p.id" class="flex items-center gap-2 text-[13px]">
          <input type="checkbox" :value="p.id" v-model="asg.picked"> {{ p.name }}
          <span v-if="asg.x && asg.x.profiles.includes(p.id)" class="chip !py-[1px]">✓</span>
        </label>
        <span v-if="!profiles.length" class="text-muted text-[13px]">{{ t('tpl.ext.noProfiles') }}</span>
      </div>
      <template #footer>
        <button class="btn" :disabled="asg.busy || !asg.picked.length" @click="doAssign('remove')">{{ t('tpl.ext.remove') }}</button>
        <button class="btn btn-start" :disabled="asg.busy || !asg.picked.length" @click="doAssign('add')">{{ t('tpl.ext.add') }}</button>
      </template>
    </cm-dialog>
  </cm-page>`,
};

/* <cm-tpl-dialogs :ext-profile="row|null" :save-profile="row|null" @close-ext="…" @close-save="…" @changed="reload"/>
   Used by the profile table: extensions of one profile, and "save settings as template". */
CM.components.CmTplDialogs = {
  props: { extProfile: Object, saveProfile: Object },
  emits: ['close-ext', 'close-save', 'changed'],
  setup(props, { emit }) {
    const { reactive, watch } = Vue;
    const ext = reactive({ list: [], picked: [], busy: false });
    watch(() => props.extProfile?.id, async id => {
      if (!id) return;
      ext.busy = true; ext.picked = [];
      try {
        const [all, mine] = await Promise.all([api('/api/extensions'), api(`/api/profiles/${id}/extensions`)]);
        ext.list = all; ext.picked = mine.extensions;
      } catch (e) { toast(e.message, true); }
      ext.busy = false;
    });
    async function saveExt() {
      ext.busy = true;
      try {
        await api(`/api/profiles/${props.extProfile.id}/extensions`, { method: 'PUT', body: JSON.stringify({ extensions: ext.picked }) });
        toast(t('tpl.prof.extSaved')); emit('close-ext'); emit('changed');
      } catch (e) { toast(e.message, true); }
      ext.busy = false;
    }
    const sv = reactive({ name: '', busy: false });
    watch(() => props.saveProfile?.id, id => { if (id) { sv.name = props.saveProfile.name; sv.busy = false; } });
    async function saveTpl() {
      if (!sv.name.trim() || sv.busy) return;
      sv.busy = true;
      try {
        await api(`/api/templates/from-profile/${props.saveProfile.id}`, { method: 'POST', body: JSON.stringify({ name: sv.name.trim() }) });
        toast(t('tpl.saved')); emit('close-save');
      } catch (e) { toast(e.message, true); }
      sv.busy = false;
    }
    return { ext, saveExt, sv, saveTpl, toggle };
  },
  template: `<div>
    <cm-dialog :open="!!extProfile" :title="t('tpl.prof.extTitle', { name: extProfile?.name })" :busy="ext.busy" :width="440" @close="$emit('close-ext')">
      <p class="m-0 text-[12.5px] text-muted">{{ t('tpl.prof.extHint') }}</p>
      <div class="flex flex-col gap-1 max-h-[40vh] overflow-auto">
        <label v-for="x in ext.list" :key="x.id" class="flex items-center gap-2 text-[13px]">
          <input type="checkbox" :checked="ext.picked.includes(x.id)" @change="toggle(ext.picked, x.id)"> {{ x.name }} <span class="text-faint text-[11.5px]">{{ x.version }}</span>
        </label>
        <span v-if="!ext.list.length" class="text-muted text-[13px]">{{ t('tpl.noExtensions') }}</span>
      </div>
      <template #footer>
        <button class="btn" :disabled="ext.busy" @click="$emit('close-ext')">{{ t('common.cancel') }}</button>
        <button class="btn btn-start" :disabled="ext.busy" @click="saveExt">{{ t('common.save') }}</button>
      </template>
    </cm-dialog>
    <cm-dialog :open="!!saveProfile" :title="t('tpl.prof.saveTitle', { name: saveProfile?.name })" :busy="sv.busy" @close="$emit('close-save')">
      <cm-field :label="t('tpl.col.name')" :hint="t('tpl.prof.saveHint')"><input class="input" v-model="sv.name" maxlength="80" autocomplete="off" @keydown.enter="saveTpl"></cm-field>
      <template #footer>
        <button class="btn" :disabled="sv.busy" @click="$emit('close-save')">{{ t('common.cancel') }}</button>
        <button class="btn btn-start" :disabled="sv.busy || !sv.name.trim()" @click="saveTpl">{{ t('tpl.prof.saveDo') }}</button>
      </template>
    </cm-dialog>
  </div>`,
};
}
