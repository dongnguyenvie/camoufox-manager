/* i18n (D1): English (default) and Vietnamese.
   Each page/component file registers its own messages with I18N.add({ en: {...}, vi: {...} });
   keys are namespaced by file ("profiles.*", "proxies.*", ...). Templates call t(key, params)
   (exposed as a global property by app.js); `{name}` in a message is replaced from params.
   Missing in the current language → English → the key itself. */

const I18N = (() => {
  const LANGS = [{ k: 'en', t: 'English' }, { k: 'vi', t: 'Tiếng Việt' }];
  const dict = { en: {}, vi: {} };
  let saved = null;
  try { saved = localStorage.getItem('cm.lang'); } catch (_) { /* storage unavailable */ }
  const locale = Vue.ref(LANGS.some(l => l.k === saved) ? saved : 'en');

  function add(msgs) {
    for (const l of Object.keys(msgs)) Object.assign(dict[l] || (dict[l] = {}), msgs[l]);
  }

  function t(key, params) {
    let s = dict[locale.value]?.[key] ?? dict.en[key] ?? key;
    if (params) s = s.replace(/\{(\w+)\}/g, (_, k) => params[k] ?? '');
    return s;
  }

  /* Switch language now; persistence (SettingsRepo) is done by app.js. */
  function use(lang) {
    if (!LANGS.some(l => l.k === lang)) return;
    locale.value = lang;
    document.documentElement.lang = lang;
    try { localStorage.setItem('cm.lang', lang); } catch (_) { /* ignore */ }
  }

  return { LANGS, locale, add, t, use };
})();
const t = I18N.t;
document.documentElement.lang = I18N.locale.value;

/* Messages shared by every page. */
I18N.add({
  en: {
    'app.name': 'Camoufox Manager',
    'app.tagline': 'Local profiles, your own proxies',
    'nav.profiles': 'Profiles',
    'nav.proxies': 'Proxies',
    'nav.templates': 'Templates',
    'nav.schedules': 'Schedules',
    'nav.settings': 'Settings',
    'common.language': 'Language',
    'common.cancel': 'Cancel',
    'common.close': 'Close',
    'common.save': 'Save',
    'common.delete': 'Delete',
    'common.create': 'Create',
    'common.refresh': 'Refresh',
    'common.loading': 'Loading…',
    'common.copied': 'Copied',
    'common.updated': 'Updated',
    'common.loadFailed': 'Could not load: {msg}',
    'common.empty': 'Nothing here yet',
    'common.comingSoon': 'Coming soon',
    'common.comingSoonBody': 'This section is being built. Its features will appear here in a later update.',
    'time.justNow': 'just now',
    'time.minutes': '{n} min ago',
    'time.hours': '{n} h ago',
    'time.days': '{n} d ago',
    'engine.ready': '{name} ready',
    'engine.notReady': '{name} not ready',
    'engine.hint.not_installed': '{name} is not installed',
    'engine.hint.browser_missing': 'The {name} browser is not downloaded',
    'engine.hint.error': '{name} reported an error',
    'engine.setup': 'Run manager/camoufox/setup.sh, then reopen the app.',
  },
  vi: {
    'app.name': 'Camoufox Manager',
    'app.tagline': 'Profile ở máy, proxy của bạn',
    'nav.profiles': 'Profile',
    'nav.proxies': 'Proxy',
    'nav.templates': 'Mẫu',
    'nav.schedules': 'Lịch chạy',
    'nav.settings': 'Cài đặt',
    'common.language': 'Ngôn ngữ',
    'common.cancel': 'Huỷ',
    'common.close': 'Đóng',
    'common.save': 'Lưu',
    'common.delete': 'Xoá',
    'common.create': 'Tạo',
    'common.refresh': 'Làm mới',
    'common.loading': 'Đang tải…',
    'common.copied': 'Đã sao chép',
    'common.updated': 'Đã cập nhật',
    'common.loadFailed': 'Không tải được: {msg}',
    'common.empty': 'Chưa có gì',
    'common.comingSoon': 'Sắp có',
    'common.comingSoonBody': 'Phần này đang được xây dựng. Tính năng sẽ xuất hiện ở bản cập nhật sau.',
    'time.justNow': 'vừa xong',
    'time.minutes': '{n} phút trước',
    'time.hours': '{n} giờ trước',
    'time.days': '{n} ngày trước',
    'engine.ready': '{name} sẵn sàng',
    'engine.notReady': '{name} chưa sẵn sàng',
    'engine.hint.not_installed': '{name} chưa được cài',
    'engine.hint.browser_missing': 'Chưa tải trình duyệt {name}',
    'engine.hint.error': '{name} báo lỗi',
    'engine.setup': 'Chạy manager/camoufox/setup.sh rồi mở lại app.',
  },
});

/* Shared formatters that need translations. */
const fmt = {
  ago(ts) {
    if (!ts) return '—';
    const s = (Date.now() - (typeof ts === 'number' ? ts * 1000 : new Date(ts))) / 1000;
    if (s < 60) return t('time.justNow');
    if (s < 3600) return t('time.minutes', { n: Math.floor(s / 60) });
    if (s < 86400) return t('time.hours', { n: Math.floor(s / 3600) });
    return t('time.days', { n: Math.floor(s / 86400) });
  },
  size: b => b > 1e9 ? (b / 1e9).toFixed(2) + ' GB' : b > 1e6 ? (b / 1e6).toFixed(1) + ' MB' : b > 1e3 ? Math.round(b / 1e3) + ' KB' : b + ' B',
  flag: cc => cc ? String.fromCodePoint(...[...cc.toUpperCase()].map(c => 127397 + c.charCodeAt(0))) : '🌐',
  /* Engine hint → translated sentence + detail. */
  engineHint(o) {
    const h = o?.providerHint; if (!h) return '';
    return t('engine.hint.' + h.code, { name: o.provider }) + ' — ' + t('engine.setup') + (h.detail ? ' (' + h.detail + ')' : '');
  },
};
