/* Root app (loaded last): sidebar + hash router (#/profiles …) + language.
   Pages register themselves in CM.pages[key]; NAV below fixes the sidebar order.
   The language comes from /api/settings (SettingsRepo) and is saved there on change. */

const NAV = [
  { key: 'profiles', icon: 'users', label: 'nav.profiles' },
  { key: 'proxies', icon: 'network', label: 'nav.proxies' },
  { key: 'templates', icon: 'layers', label: 'nav.templates' },
  { key: 'schedules', icon: 'calendar', label: 'nav.schedules' },
  { key: 'devices', icon: 'devices', label: 'nav.devices' },
  { key: 'settings', icon: 'settings', label: 'nav.settings' },
];

/* Switch the UI language and persist it. */
CM.setLanguage = async lang => {
  I18N.use(lang);
  try { await api('/api/settings', { method: 'PUT', body: JSON.stringify({ language: lang }) }); }
  catch (e) { toast(e.message, true); }
};

const app = Vue.createApp({
  setup() {
    const route = () => {
      const k = location.hash.replace(/^#\/?/, '').split('/')[0];
      return NAV.some(n => n.key === k) ? k : 'profiles';
    };
    const current = Vue.ref(route());
    window.addEventListener('hashchange', () => { current.value = route(); });
    const page = Vue.computed(() => 'page-' + current.value);
    Vue.watchEffect(() => { document.title = t('nav.' + current.value) + ' · ' + t('app.name'); });

    Vue.onMounted(async () => {
      try {
        const s = await api('/api/settings');
        if (s.language && s.language !== I18N.locale.value) I18N.use(s.language);
      } catch (_) { /* keep the cached / default language */ }
    });

    return { NAV, current, page, setLanguage: CM.setLanguage };
  },
  template: `<div class="flex h-full min-h-0">
    <cm-sidebar :nav="NAV" :current="current" @lang="setLanguage"></cm-sidebar>
    <main class="flex-1 min-w-0 min-h-0 flex flex-col"><component :is="page" :key="current"></component></main>
    <cm-toasts></cm-toasts>
  </div>`,
});

app.config.globalProperties.t = t;
for (const [name, c] of Object.entries(CM.components)) app.component(name, c);
for (const n of NAV) app.component('page-' + n.key, CM.pages[n.key] || { template: '<cm-page :title="t(\'nav.' + n.key + '\')"><cm-coming-soon/></cm-page>' });
app.mount('#app');
