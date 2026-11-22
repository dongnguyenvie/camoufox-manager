/* Profile form (phase 4): edit a profile in a dialog.
   <cm-profile-form :open="b" :profile="view" @close="b = false" @saved="reload"/>
   `profile` is a row of /api/overview (kept live by the page's polling; the form copies it when it
   opens, so polling never overwrites what you are typing).
   Editable: name, notes, OS, start URL, screen, window size, locale/timezone override, WebRTC
   mode, extra fonts, humanize, canvas mode. The pinned machine is shown read-only; changing the OS
   asks "keep machine" vs "new machine", and "Regenerate machine" is always explicit. */

I18N.add({
  en: {
    'profileform.title': 'Edit profile',
    'profileform.running': 'This profile is running. Changes apply the next time it starts.',
    'profileform.section.general': 'General',
    'profileform.section.display': 'Screen and window',
    'profileform.section.region': 'Region',
    'profileform.section.network': 'WebRTC',
    'profileform.section.fingerprint': 'Fingerprint',
    'profileform.section.data': 'Data',
    'profileform.dataHint': 'What this profile keeps between runs. Turning one off stops it being saved and clears it when the browser closes. Cookies and logins are never cleared.',
    'profileform.data.passwords': 'Save passwords',
    'profileform.data.history': 'Save browsing history',
    'profileform.data.cache': 'Save cache',
    'profileform.data.siteStorage': 'Save site storage (localStorage, IndexedDB)',
    'profileform.prefs': 'Custom Firefox prefs',
    'profileform.prefsHint': 'Advanced. Applied last at launch. Values: true, false, a number or text. Proxy, WebRTC, geo, intl, camoufox and fingerprint prefs are managed by the app and rejected.',
    'profileform.prefKey': 'about:config name',
    'profileform.prefValue': 'Value',
    'profileform.prefAdd': 'Add pref',
    'profileform.prefRemove': 'Remove',
    'profileform.section.bookmarks': 'Bookmarks',
    'profileform.bookmarksHint': 'Kept in a "Camoufox Manager" folder on the bookmarks toolbar, written when the profile starts. A new profile shows them from its second start. Bookmarks you make yourself are left alone.',
    'profileform.bm.title': 'Title',
    'profileform.bm.url': 'https://',
    'profileform.bm.folder': 'Folder (optional)',
    'profileform.bm.add': 'Add bookmark',
    'profileform.bm.remove': 'Remove',
    'profileform.bm.import': 'Import bookmarks.html',
    'profileform.bm.imported': 'Imported {n} bookmarks',
    'profileform.section.machine': 'Pinned machine',
    'profileform.name': 'Name',
    'profileform.notes': 'Notes',
    'profileform.os': 'Operating system',
    'profileform.osHint': 'Changing it asks whether to keep the pinned machine or pin a new one.',
    'profileform.startUrl': 'Start URL',
    'profileform.startUrlHint': 'Opened when the browser starts. Empty = the browser default.',
    'profileform.screen': 'Screen size',
    'profileform.screenHint': 'Empty = the pinned screen ({res}). Format 1920x1080.',
    'profileform.window': 'Window size',
    'profileform.windowHint': 'Empty = fills the screen. Must fit inside the screen.',
    'profileform.locale': 'Locale',
    'profileform.localeHint': 'Empty = follows the proxy country. Example: de-DE.',
    'profileform.timezone': 'Timezone',
    'profileform.timezoneHint': 'Empty = follows the proxy. Geolocation is only sent together with the proxy timezone.',
    'profileform.followsProxy': 'proxy: {value}',
    'profileform.noProxyValue': 'no proxy: browser default',
    'profileform.webrtc': 'WebRTC mode',
    'profileform.webrtc.replace': 'Replace: report the proxy IP',
    'profileform.webrtc.real': 'Real: report the real IP',
    'profileform.webrtc.forward': 'Forward: leave WebRTC untouched',
    'profileform.webrtc.none': 'None: disable WebRTC',
    'profileform.fonts': 'Extra fonts',
    'profileform.fontsHint': 'One per line, added to the pinned font list ({n} fonts).',
    'profileform.humanize': 'Humanize cursor movement',
    'profileform.blockLocal': 'Block local network access',
    'profileform.blockLocalHint': 'Stops pages from probing localhost and your LAN (port scans). Works with and without a proxy.',
    'profileform.canvas': 'Canvas and WebGL',
    'profileform.canvas.stable': 'Unique: own per-profile value, same on every launch (recommended)',
    'profileform.canvas.native': 'Native: this Mac\'s real rendering, shared by all profiles here',
    'profileform.canvas.random': 'Random: new noise on every launch',
    'profileform.canvasHint': 'Unique gives each profile its own canvas, WebGL and audio values that stay the same across launches. Native makes every profile on this Mac look identical. Random changes each launch, which sites can notice.',
    'profileform.machineHint': 'Frozen when the profile was created. Read-only.',
    'profileform.m.os': 'OS',
    'profileform.m.ua': 'User-Agent',
    'profileform.m.platform': 'Platform',
    'profileform.m.resolution': 'Screen',
    'profileform.m.dpr': 'Pixel ratio',
    'profileform.m.cpu': 'CPU cores',
    'profileform.m.gpu': 'GPU',
    'profileform.m.fonts': 'Fonts',
    'profileform.m.language': 'Language',
    'profileform.regenerate': 'Regenerate machine…',
    'profileform.regenerateStopFirst': 'Stop the profile before pinning a new machine',
    'profileform.regenTitle': 'Pin a new machine?',
    'profileform.regenBody': 'Screen, GPU, CPU, fonts and the rest of the hardware fingerprint are replaced by a new set. Sites that saw the old machine will see a different device, so existing logins may be questioned.',
    'profileform.regenOs': 'Operating system of the new machine',
    'profileform.regenConfirm': 'Pin new machine',
    'profileform.regenerated': 'Pinned a new machine',
    'profileform.osTitle': 'Operating system changed',
    'profileform.osBody': 'The pinned machine is {from}. You chose {to}. What should happen?',
    'profileform.osKeep': 'Keep machine ({from})',
    'profileform.osKeepHint': 'No fingerprint change. The OS stays {from}.',
    'profileform.osNew': 'New {to} machine',
    'profileform.osNewHint': 'Pins fresh {to} hardware: new screen, GPU, cores and fonts.',
    'profileform.saved': 'Profile saved',
    'profileform.saving': 'Saving…',
  },
  vi: {
    'profileform.title': 'Sửa profile',
    'profileform.running': 'Profile đang chạy. Thay đổi có hiệu lực ở lần mở tiếp theo.',
    'profileform.section.general': 'Chung',
    'profileform.section.display': 'Màn hình và cửa sổ',
    'profileform.section.region': 'Vùng',
    'profileform.section.network': 'WebRTC',
    'profileform.section.fingerprint': 'Fingerprint',
    'profileform.section.data': 'Dữ liệu',
    'profileform.dataHint': 'Những gì profile này giữ lại giữa các lần mở. Tắt một mục thì không lưu và sẽ xoá khi đóng browser. Cookie và đăng nhập không bao giờ bị xoá.',
    'profileform.data.passwords': 'Lưu mật khẩu',
    'profileform.data.history': 'Lưu lịch sử duyệt web',
    'profileform.data.cache': 'Lưu cache',
    'profileform.data.siteStorage': 'Lưu dữ liệu website (localStorage, IndexedDB)',
    'profileform.prefs': 'Pref Firefox tuỳ chỉnh',
    'profileform.prefsHint': 'Nâng cao. Áp dụng cuối cùng khi mở. Giá trị: true, false, số hoặc chữ. Pref proxy, WebRTC, geo, intl, camoufox và fingerprint do app quản lý nên bị từ chối.',
    'profileform.prefKey': 'Tên trong about:config',
    'profileform.prefValue': 'Giá trị',
    'profileform.prefAdd': 'Thêm pref',
    'profileform.prefRemove': 'Xoá',
    'profileform.section.bookmarks': 'Dấu trang',
    'profileform.bookmarksHint': 'Lưu trong thư mục "Camoufox Manager" trên thanh dấu trang, ghi khi hồ sơ khởi động. Hồ sơ mới sẽ hiện chúng từ lần khởi động thứ hai. Dấu trang bạn tự tạo được giữ nguyên.',
    'profileform.bm.title': 'Tiêu đề',
    'profileform.bm.url': 'https://',
    'profileform.bm.folder': 'Thư mục (tuỳ chọn)',
    'profileform.bm.add': 'Thêm dấu trang',
    'profileform.bm.remove': 'Xoá',
    'profileform.bm.import': 'Nhập bookmarks.html',
    'profileform.bm.imported': 'Đã nhập {n} dấu trang',
    'profileform.section.machine': 'Máy đã ghim',
    'profileform.name': 'Tên',
    'profileform.notes': 'Ghi chú',
    'profileform.os': 'Hệ điều hành',
    'profileform.osHint': 'Đổi sẽ hỏi giữ máy đã ghim hay ghim máy mới.',
    'profileform.startUrl': 'URL mở đầu',
    'profileform.startUrlHint': 'Mở khi browser khởi động. Để trống = mặc định của browser.',
    'profileform.screen': 'Kích thước màn hình',
    'profileform.screenHint': 'Để trống = màn hình đã ghim ({res}). Dạng 1920x1080.',
    'profileform.window': 'Kích thước cửa sổ',
    'profileform.windowHint': 'Để trống = chiếm cả màn hình. Phải nằm trong màn hình.',
    'profileform.locale': 'Locale',
    'profileform.localeHint': 'Để trống = theo quốc gia của proxy. Ví dụ: de-DE.',
    'profileform.timezone': 'Múi giờ',
    'profileform.timezoneHint': 'Để trống = theo proxy. Vị trí địa lý chỉ gửi cùng múi giờ của proxy.',
    'profileform.followsProxy': 'proxy: {value}',
    'profileform.noProxyValue': 'không proxy: mặc định của browser',
    'profileform.webrtc': 'Chế độ WebRTC',
    'profileform.webrtc.replace': 'Replace: báo IP của proxy',
    'profileform.webrtc.real': 'Real: báo IP thật',
    'profileform.webrtc.forward': 'Forward: để WebRTC nguyên trạng',
    'profileform.webrtc.none': 'None: tắt WebRTC',
    'profileform.fonts': 'Font thêm',
    'profileform.fontsHint': 'Mỗi dòng một font, thêm vào danh sách font đã ghim ({n} font).',
    'profileform.humanize': 'Di chuyển chuột giống người',
    'profileform.blockLocal': 'Chặn truy cập mạng nội bộ',
    'profileform.blockLocalHint': 'Ngăn trang web dò localhost và mạng LAN của bạn (quét cổng). Áp dụng cho cả profile có và không có proxy.',
    'profileform.canvas': 'Canvas và WebGL',
    'profileform.canvas.stable': 'Riêng: mỗi profile một giá trị, giữ nguyên mỗi lần mở (khuyên dùng)',
    'profileform.canvas.native': 'Gốc: cách vẽ thật của máy Mac này, mọi profile ở đây giống nhau',
    'profileform.canvas.random': 'Ngẫu nhiên: nhiễu mới mỗi lần mở',
    'profileform.canvasHint': 'Riêng cho mỗi profile giá trị canvas, WebGL và audio của riêng nó, giữ nguyên qua các lần mở. Gốc khiến mọi profile trên máy này trông giống hệt nhau. Ngẫu nhiên đổi mỗi lần mở, website có thể nhận ra.',
    'profileform.machineHint': 'Được cố định khi tạo profile. Chỉ xem.',
    'profileform.m.os': 'HĐH',
    'profileform.m.ua': 'User-Agent',
    'profileform.m.platform': 'Platform',
    'profileform.m.resolution': 'Màn hình',
    'profileform.m.dpr': 'Tỉ lệ điểm ảnh',
    'profileform.m.cpu': 'Số nhân CPU',
    'profileform.m.gpu': 'GPU',
    'profileform.m.fonts': 'Font',
    'profileform.m.language': 'Ngôn ngữ',
    'profileform.regenerate': 'Tạo lại máy…',
    'profileform.regenerateStopFirst': 'Stop profile trước khi ghim máy mới',
    'profileform.regenTitle': 'Ghim máy mới?',
    'profileform.regenBody': 'Màn hình, GPU, CPU, font và phần cứng còn lại sẽ được thay bằng bộ mới. Website từng thấy máy cũ sẽ thấy một thiết bị khác, nên đăng nhập hiện có có thể bị hỏi lại.',
    'profileform.regenOs': 'Hệ điều hành của máy mới',
    'profileform.regenConfirm': 'Ghim máy mới',
    'profileform.regenerated': 'Đã ghim máy mới',
    'profileform.osTitle': 'Đã đổi hệ điều hành',
    'profileform.osBody': 'Máy đã ghim là {from}. Bạn chọn {to}. Làm gì tiếp?',
    'profileform.osKeep': 'Giữ máy ({from})',
    'profileform.osKeepHint': 'Không đổi fingerprint. HĐH vẫn là {from}.',
    'profileform.osNew': 'Máy {to} mới',
    'profileform.osNewHint': 'Ghim phần cứng {to} mới: màn hình, GPU, số nhân và font mới.',
    'profileform.saved': 'Đã lưu profile',
    'profileform.saving': 'Đang lưu…',
  },
});

{ // file-local scope: classic scripts share one global scope
const OSES = [{ k: 'win', t: 'Windows' }, { k: 'mac', t: 'macOS' }, { k: 'lin', t: 'Linux' }];
const osLabel = k => OSES.find(o => o.k === k)?.t || k;
const WEBRTC = ['replace', 'real', 'forward', 'none'];
const COMMON_LOCALES = ['en-US', 'en-GB', 'de-DE', 'fr-FR', 'es-ES', 'it-IT', 'pt-BR', 'nl-NL', 'pl-PL', 'ru-RU', 'tr-TR', 'ja-JP', 'ko-KR', 'zh-CN', 'vi-VN', 'th-TH', 'id-ID'];
const timezones = () => { try { return Intl.supportedValuesOf('timeZone'); } catch (_) { return []; } };

CM.components.CmProfileForm = {
  props: { open: Boolean, profile: Object },
  emits: ['close', 'saved'],
  setup(props, { emit }) {
    const { reactive, ref, computed, watch } = Vue;
    const f = reactive({ name: '', notes: '', os: 'win', startUrl: '', screen: '', window: '', locale: '', timezone: '', webrtc: 'replace', fonts: '', humanize: false, blockLocal: true, canvas: 'stable', data: { passwords: true, history: true, cache: true, siteStorage: true }, prefs: [], bookmarks: [] });
    const view = ref(null);          // snapshot shown in the machine panel (replaced after a regenerate)
    const busy = ref(false), error = ref(''), osPrompt = ref(false);
    const regen = reactive({ open: false, os: 'win', busy: false });

    function seed(p) {
      view.value = p;
      const s = p.settings || {};
      Object.assign(f, {
        name: p.name, notes: p.notes || '', os: p.os, startUrl: s.startUrl || '', screen: s.screen || '', window: s.window || '',
        locale: s.locale || '', timezone: s.timezone || '', webrtc: s.webrtc || 'replace', fonts: (s.fonts || []).join('\n'),
        humanize: !!s.humanize, blockLocal: s.blockLocalNetwork !== false, canvas: s.canvas || 'stable',
        data: { passwords: true, history: true, cache: true, siteStorage: true, ...(s.data || {}) },
        prefs: Object.entries(s.customPrefs || {}).map(([k, v]) => ({ k, v: String(v) })),
        bookmarks: (s.bookmarks || []).map(b => ({ title: b.title || '', url: b.url || '', folder: b.folder || '' })),
      });
      error.value = '';
    }
    // Copy once per opening (or when another profile is opened), never on every poll.
    // The watched value is a primitive (id while open), so a poll that swaps the profile object
    // for a fresh copy of the same profile does not retrigger it.
    watch(() => (props.open && props.profile ? props.profile.id : null), id => {
      if (id) seed(props.profile);
      osPrompt.value = false; regen.open = false;
    }, { immediate: true });

    const running = computed(() => !!props.profile?.running);
    const fp = computed(() => view.value?.fp || {});
    const proxyTz = computed(() => props.profile?.assigned?.geo?.timezone);
    const proxyLocale = computed(() => props.profile?.assigned?.geo?.countryCode);
    const tzPlaceholder = computed(() => proxyTz.value ? t('profileform.followsProxy', { value: proxyTz.value }) : t('profileform.noProxyValue'));
    const localePlaceholder = computed(() => proxyLocale.value ? t('profileform.followsProxy', { value: proxyLocale.value }) : t('profileform.noProxyValue'));
    const machineRows = computed(() => {
      const m = fp.value;
      return [
        ['profileform.m.os', osLabel(view.value?.os)],
        ['profileform.m.ua', m.userAgent], ['profileform.m.platform', m.platform], ['profileform.m.resolution', m.resolution],
        ['profileform.m.dpr', m.dpr], ['profileform.m.cpu', m.cpu],
        ['profileform.m.gpu', [m.webglVendor, m.webglRenderer].filter(Boolean).join(' / ')],
        ['profileform.m.fonts', m.fonts], ['profileform.m.language', m.language],
      ].filter(r => r[1] !== undefined && r[1] !== null && r[1] !== '');
    });
    const osFrom = computed(() => osLabel(props.profile?.os));
    const osTo = computed(() => osLabel(f.os));
    const tzList = timezones();

    // Text -> typed pref value: true/false, plain numbers, otherwise the text itself.
    const prefValue = v => {
      const x = v.trim();
      if (x === 'true') return true;
      if (x === 'false') return false;
      return x !== '' && /^-?\d+(\.\d+)?$/.test(x) ? Number(x) : v;
    };
    const customPrefs = () => Object.fromEntries(f.prefs.filter(r => r.k.trim()).map(r => [r.k.trim(), prefValue(r.v)]));
    const addPref = () => f.prefs.push({ k: '', v: '' });
    // Bookmarks section (CM-3.4): list editor + bookmarks.html import (parsed by the server).
    const bmFile = ref(null), bmBusy = ref(false);
    const addBookmark = () => f.bookmarks.push({ title: '', url: '', folder: '' });
    const bookmarkList = () => f.bookmarks.filter(b => b.url.trim()).map(b => ({ title: b.title.trim(), url: b.url.trim(), folder: b.folder.trim() || null }));
    async function importBookmarks(ev) {
      const file = ev.target.files[0]; ev.target.value = '';
      if (!file) return;
      bmBusy.value = true;
      try {
        const r = await fetch('/api/bookmarks/import', { method: 'POST', body: file, headers: { 'content-type': 'text/html' } });
        const j = await r.json().catch(() => ({}));
        if (!r.ok) throw new Error(j.error || r.statusText);
        const have = new Set(f.bookmarks.map(b => b.url.trim() + '\n' + b.folder.trim()));
        let n = 0;
        for (const b of j.bookmarks) {
          const k = b.url + '\n' + (b.folder || '');
          if (have.has(k)) continue;
          have.add(k); n++;
          f.bookmarks.push({ title: b.title || '', url: b.url, folder: b.folder || '' });
        }
        toast(t('profileform.bm.imported', { n }));
      } catch (e) { error.value = e.message; }
      bmBusy.value = false;
    }
    const body = osAction => ({
      name: f.name.trim(), notes: f.notes, os: f.os, osAction,
      settings: {
        locale: f.locale.trim() || null, timezone: f.timezone.trim() || null, webrtc: f.webrtc,
        window: f.window.trim() || null, screen: f.screen.trim() || null, startUrl: f.startUrl.trim() || null,
        humanize: f.humanize, fonts: f.fonts.split('\n').map(x => x.trim()).filter(Boolean), canvas: f.canvas,
        data: { ...f.data }, customPrefs: customPrefs(), blockLocalNetwork: f.blockLocal ? null : false, bookmarks: bookmarkList(),
      },
    });
    async function save(osAction) {
      if (busy.value || !f.name.trim()) return;
      if (f.os !== props.profile.os && !osAction) { osPrompt.value = true; return; }
      osPrompt.value = false; busy.value = true; error.value = '';
      try {
        await api(`/api/profiles/${props.profile.id}`, { method: 'PUT', body: JSON.stringify(body(osAction)) });
        toast(t('profileform.saved'));
        emit('saved'); emit('close');
      } catch (e) { error.value = e.message; }
      busy.value = false;
    }
    const openRegen = () => { regen.os = view.value?.os || 'win'; regen.open = true; };
    async function regenerate() {
      regen.busy = true; error.value = '';
      try {
        const r = await api(`/api/profiles/${props.profile.id}/regenerate-machine`, { method: 'POST', body: JSON.stringify({ os: regen.os }) });
        seed(r);
        toast(t('profileform.regenerated'));
        regen.open = false; emit('saved');
      } catch (e) { error.value = e.message; regen.open = false; }
      regen.busy = false;
    }

    return { f, OSES, WEBRTC, COMMON_LOCALES, tzList, busy, error, osPrompt, regen, running, fp, view, addPref, bmFile, bmBusy, addBookmark, importBookmarks, machineRows, tzPlaceholder, localePlaceholder, osFrom, osTo, save, openRegen, regenerate };
  },
  template: `<div>
    <cm-dialog :open="open && !regen.open && !osPrompt" :title="t('profileform.title')" :busy="busy" :width="680" @close="$emit('close')">
      <div v-if="running" class="banner !m-0"><cm-icon name="alert"/><div>{{ t('profileform.running') }}</div></div>
      <form id="profile-form" class="flex flex-col gap-4 max-h-[68vh] overflow-auto pr-1" @submit.prevent="save()">
        <fieldset class="m-0 p-0 border-0 grid grid-cols-2 gap-3 max-sm:grid-cols-1">
          <legend class="col-span-full text-[11px] uppercase tracking-wide text-faint font-semibold mb-1 p-0">{{ t('profileform.section.general') }}</legend>
          <cm-field :label="t('profileform.name')"><input class="input" v-model="f.name" maxlength="80" autocomplete="off"></cm-field>
          <cm-field :label="t('profileform.os')" :hint="t('profileform.osHint')">
            <select class="input" v-model="f.os"><option v-for="x in OSES" :key="x.k" :value="x.k">{{ x.t }}</option></select>
          </cm-field>
          <cm-field class="col-span-full" :label="t('profileform.notes')"><textarea class="input" rows="2" v-model="f.notes" maxlength="2000"></textarea></cm-field>
          <cm-field class="col-span-full" :label="t('profileform.startUrl')" :hint="t('profileform.startUrlHint')">
            <input class="input" v-model="f.startUrl" placeholder="https://" autocomplete="off" spellcheck="false">
          </cm-field>
        </fieldset>

        <fieldset class="m-0 p-0 border-0 grid grid-cols-2 gap-3 max-sm:grid-cols-1">
          <legend class="col-span-full text-[11px] uppercase tracking-wide text-faint font-semibold mb-1 p-0">{{ t('profileform.section.display') }}</legend>
          <cm-field :label="t('profileform.screen')" :hint="t('profileform.screenHint', { res: fp.resolution || '—' })">
            <input class="input mono" v-model="f.screen" :placeholder="fp.resolution || '1920x1080'" autocomplete="off" spellcheck="false">
          </cm-field>
          <cm-field :label="t('profileform.window')" :hint="t('profileform.windowHint')">
            <input class="input mono" v-model="f.window" placeholder="1280x800" autocomplete="off" spellcheck="false">
          </cm-field>
        </fieldset>

        <fieldset class="m-0 p-0 border-0 grid grid-cols-2 gap-3 max-sm:grid-cols-1">
          <legend class="col-span-full text-[11px] uppercase tracking-wide text-faint font-semibold mb-1 p-0">{{ t('profileform.section.region') }}</legend>
          <cm-field :label="t('profileform.locale')" :hint="t('profileform.localeHint')">
            <input class="input mono" v-model="f.locale" list="pf-locales" :placeholder="localePlaceholder" autocomplete="off" spellcheck="false">
          </cm-field>
          <cm-field :label="t('profileform.timezone')" :hint="t('profileform.timezoneHint')">
            <input class="input mono" v-model="f.timezone" list="pf-timezones" :placeholder="tzPlaceholder" autocomplete="off" spellcheck="false">
          </cm-field>
          <datalist id="pf-locales"><option v-for="l in COMMON_LOCALES" :key="l" :value="l"></option></datalist>
          <datalist id="pf-timezones"><option v-for="z in tzList" :key="z" :value="z"></option></datalist>
        </fieldset>

        <fieldset class="m-0 p-0 border-0 grid grid-cols-2 gap-3 max-sm:grid-cols-1">
          <legend class="col-span-full text-[11px] uppercase tracking-wide text-faint font-semibold mb-1 p-0">{{ t('profileform.section.network') }}</legend>
          <cm-field class="col-span-full" :label="t('profileform.webrtc')">
            <select class="input" v-model="f.webrtc"><option v-for="m in WEBRTC" :key="m" :value="m">{{ t('profileform.webrtc.' + m) }}</option></select>
          </cm-field>
        </fieldset>

        <fieldset class="m-0 p-0 border-0 grid grid-cols-2 gap-3 max-sm:grid-cols-1">
          <legend class="col-span-full text-[11px] uppercase tracking-wide text-faint font-semibold mb-1 p-0">{{ t('profileform.section.fingerprint') }}</legend>
          <cm-field class="col-span-full" :label="t('profileform.canvas')" :hint="t('profileform.canvasHint')">
            <select class="input" v-model="f.canvas">
              <option value="stable">{{ t('profileform.canvas.stable') }}</option>
              <option value="native">{{ t('profileform.canvas.native') }}</option>
              <option value="random">{{ t('profileform.canvas.random') }}</option>
            </select>
          </cm-field>
          <cm-field class="col-span-full" :label="t('profileform.fonts')" :hint="t('profileform.fontsHint', { n: fp.fonts ?? 0 })">
            <textarea class="input mono" rows="3" v-model="f.fonts" spellcheck="false"></textarea>
          </cm-field>
          <div class="col-span-full"><cm-switch v-model="f.humanize" :label="t('profileform.humanize')"/></div>
          <div class="col-span-full">
            <cm-switch v-model="f.blockLocal" :label="t('profileform.blockLocal')"/>
            <p class="m-0 mt-1 text-[12px] text-faint">{{ t('profileform.blockLocalHint') }}</p>
          </div>
        </fieldset>

        <fieldset class="m-0 p-0 border-0 flex flex-col gap-2">
          <legend class="text-[11px] uppercase tracking-wide text-faint font-semibold mb-1 p-0">{{ t('profileform.section.data') }}</legend>
          <p class="m-0 text-[12px] text-faint">{{ t('profileform.dataHint') }}</p>
          <cm-switch v-model="f.data.passwords" :label="t('profileform.data.passwords')"/>
          <cm-switch v-model="f.data.history" :label="t('profileform.data.history')"/>
          <cm-switch v-model="f.data.cache" :label="t('profileform.data.cache')"/>
          <cm-switch v-model="f.data.siteStorage" :label="t('profileform.data.siteStorage')"/>
          <cm-field :label="t('profileform.prefs')" :hint="t('profileform.prefsHint')">
            <div class="flex flex-col gap-2">
              <div v-for="(r, i) in f.prefs" :key="i" class="flex gap-2 items-center">
                <input class="input mono flex-[2]" v-model="r.k" :placeholder="t('profileform.prefKey')" autocomplete="off" spellcheck="false">
                <input class="input mono flex-1" v-model="r.v" :placeholder="t('profileform.prefValue')" autocomplete="off" spellcheck="false">
                <button type="button" class="btn" @click="f.prefs.splice(i, 1)">{{ t('profileform.prefRemove') }}</button>
              </div>
              <div><button type="button" class="btn" @click="addPref">{{ t('profileform.prefAdd') }}</button></div>
            </div>
          </cm-field>
        </fieldset>

        <fieldset class="m-0 p-0 border-0 flex flex-col gap-2">
          <legend class="text-[11px] uppercase tracking-wide text-faint font-semibold mb-1 p-0">{{ t('profileform.section.bookmarks') }}</legend>
          <p class="m-0 text-[12px] text-faint">{{ t('profileform.bookmarksHint') }}</p>
          <div v-for="(b, i) in f.bookmarks" :key="i" class="flex gap-2 items-center">
            <input class="input flex-1" v-model="b.title" :placeholder="t('profileform.bm.title')" autocomplete="off">
            <input class="input mono flex-[2]" v-model="b.url" :placeholder="t('profileform.bm.url')" autocomplete="off" spellcheck="false">
            <input class="input flex-1" v-model="b.folder" :placeholder="t('profileform.bm.folder')" autocomplete="off">
            <button type="button" class="btn" @click="f.bookmarks.splice(i, 1)">{{ t('profileform.bm.remove') }}</button>
          </div>
          <div class="flex gap-2">
            <button type="button" class="btn" @click="addBookmark">{{ t('profileform.bm.add') }}</button>
            <button type="button" class="btn" :disabled="bmBusy" @click="bmFile.click()">{{ t('profileform.bm.import') }}</button>
            <input ref="bmFile" type="file" accept=".html,.htm,text/html" class="hidden" @change="importBookmarks">
          </div>
        </fieldset>

        <fieldset class="m-0 p-0 border-0 flex flex-col gap-2">
          <legend class="text-[11px] uppercase tracking-wide text-faint font-semibold mb-1 p-0">{{ t('profileform.section.machine') }}</legend>
          <p class="m-0 text-[12px] text-faint">{{ t('profileform.machineHint') }}</p>
          <dl class="m-0 grid grid-cols-[110px_1fr] gap-x-3 gap-y-1 text-[12.5px] rounded-lg bg-panel2 p-3">
            <template v-for="r in machineRows" :key="r[0]"><dt class="text-muted">{{ t(r[0]) }}</dt><dd class="m-0 break-all select-text">{{ r[1] }}</dd></template>
          </dl>
          <div><button type="button" class="btn" :disabled="running" :title="running ? t('profileform.regenerateStopFirst') : ''" @click="openRegen">{{ t('profileform.regenerate') }}</button></div>
        </fieldset>
        <div v-if="error" class="text-bad text-[12.5px]">{{ error }}</div>
      </form>
      <template #footer>
        <button type="button" class="btn" :disabled="busy" @click="$emit('close')">{{ t('common.cancel') }}</button>
        <button type="submit" form="profile-form" class="btn btn-start" :disabled="busy || !f.name.trim()">
          <template v-if="busy"><span class="spin"></span> {{ t('profileform.saving') }}</template><template v-else>{{ t('common.save') }}</template>
        </button>
      </template>
    </cm-dialog>

    <!-- OS differs from the pinned machine: keep it or pin a new one -->
    <cm-dialog :open="open && osPrompt" :title="t('profileform.osTitle')" @close="osPrompt = false">
      <p class="m-0 text-[13px] text-muted">{{ t('profileform.osBody', { from: osFrom, to: osTo }) }}</p>
      <button type="button" class="btn !h-auto !items-start !justify-start text-left flex-col gap-0.5 py-2" @click="save('keep')">
        <b>{{ t('profileform.osKeep', { from: osFrom }) }}</b><span class="text-faint text-[12px] font-normal">{{ t('profileform.osKeepHint', { from: osFrom }) }}</span>
      </button>
      <button type="button" class="btn !h-auto !items-start !justify-start text-left flex-col gap-0.5 py-2" @click="save('regenerate')">
        <b>{{ t('profileform.osNew', { to: osTo }) }}</b><span class="text-faint text-[12px] font-normal">{{ t('profileform.osNewHint', { to: osTo }) }}</span>
      </button>
      <template #footer><button type="button" class="btn" @click="osPrompt = false">{{ t('common.cancel') }}</button></template>
    </cm-dialog>

    <!-- Explicit regenerate -->
    <cm-dialog :open="open && regen.open" :title="t('profileform.regenTitle')" :busy="regen.busy" @close="regen.open = false">
      <p class="m-0 text-[13px] text-muted">{{ t('profileform.regenBody') }}</p>
      <cm-field :label="t('profileform.regenOs')">
        <select class="input" v-model="regen.os"><option v-for="x in OSES" :key="x.k" :value="x.k">{{ x.t }}</option></select>
      </cm-field>
      <template #footer>
        <button type="button" class="btn" :disabled="regen.busy" @click="regen.open = false">{{ t('common.cancel') }}</button>
        <button type="button" class="btn btn-stop" :disabled="regen.busy" @click="regenerate">
          <template v-if="regen.busy"><span class="spin"></span></template><template v-else>{{ t('profileform.regenConfirm') }}</template>
        </button>
      </template>
    </cm-dialog>
  </div>`,
};
} // end file-local scope
