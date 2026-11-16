/* Settings page. Phase 1: language + where data lives. Phase 12 extends it (engine status/version,
   install/update Camoufox, headless default, API key). Layout: titled sections of label/value rows. */

I18N.add({
  en: {
    'settings.general': 'General',
    'settings.storage': 'Storage',
    'settings.dataDir': 'Data folder',
    'settings.db': 'Database',
    'settings.relayPorts': 'Local proxy ports from',
    'settings.more': 'Engine install/update, headless default and API key are coming in a later update.',
  },
  vi: {
    'settings.general': 'Chung',
    'settings.storage': 'Lưu trữ',
    'settings.dataDir': 'Thư mục dữ liệu',
    'settings.db': 'Cơ sở dữ liệu',
    'settings.relayPorts': 'Cổng proxy local từ',
    'settings.more': 'Cài/cập nhật engine, chế độ headless mặc định và API key sẽ có ở bản cập nhật sau.',
  },
});

CM.pages.settings = {
  setup() {
    const s = Vue.ref(null);
    Vue.onMounted(async () => {
      try { s.value = await api('/api/settings'); } catch (e) { toast(t('common.loadFailed', { msg: e.message }), true); }
    });
    return { s, LANGS: I18N.LANGS, locale: I18N.locale, setLanguage: CM.setLanguage };
  },
  template: `<cm-page :title="t('nav.settings')">
    <div class="w-full max-w-[720px] mx-auto flex flex-col gap-6 py-2">
      <section>
        <h2 class="section-title">{{ t('settings.general') }}</h2>
        <div class="kv">
          <div class="kv-row"><span>{{ t('common.language') }}</span>
            <select class="input max-w-[220px]" :value="locale" @change="setLanguage($event.target.value)">
              <option v-for="l in LANGS" :key="l.k" :value="l.k">{{ l.t }}</option>
            </select>
          </div>
        </div>
      </section>
      <section>
        <h2 class="section-title">{{ t('settings.storage') }}</h2>
        <div class="kv">
          <div class="kv-row"><span>{{ t('settings.dataDir') }}</span><code class="break-all">{{ s ? s.dataDir : '…' }}</code></div>
          <div class="kv-row"><span>{{ t('settings.db') }}</span><code class="break-all">{{ s ? (s.dbPath || '—') : '…' }}</code></div>
          <div class="kv-row"><span>{{ t('settings.relayPorts') }}</span><code>{{ s ? '127.0.0.1:' + s.proxyBasePort : '…' }}</code></div>
        </div>
      </section>
      <p class="m-0 text-[13px] text-muted">{{ t('settings.more') }}</p>
    </div>
  </cm-page>`,
};
