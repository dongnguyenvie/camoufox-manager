/* Settings page: language + headless default, engine status/version with optional Camoufox
   install/update (live log), named API keys, storage + ports. Layout: titled sections of label/value rows.
   Install log: POST /api/engine/install, then poll GET /api/engine/install?from=N until it stops. */

I18N.add({
  en: {
    'settings.general': 'General',
    'settings.headless': 'Headless by default',
    'settings.headlessHint': 'Start browsers without a window unless told otherwise.',
    'settings.engine': 'Browser engine',
    'settings.engineStatus': 'Status',
    'settings.engineReady': 'Ready',
    'settings.engineMissing': 'Not installed',
    'settings.engineBroken': 'Needs attention',
    'settings.pkgVersion': 'Camoufox package',
    'settings.browserVersion': 'Camoufox browser',
    'settings.unknown': 'unknown',
    'settings.install': 'Install / Update Camoufox',
    'settings.installing': 'Installing…',
    'settings.installHint': 'Optional. Downloads about 2.4 GB (browser + fonts). The app keeps working without it; only launching profiles needs it.',
    'settings.installLog': 'Install log',
    'settings.installDone': 'Camoufox is installed',
    'settings.installDoneBody': 'Done. You can launch profiles now.',
    'settings.installFailedTitle': 'The install did not finish',
    'settings.retry': 'Retry',
    'settings.copyLog': 'Copy log',
    'settings.manual': 'Manual install',
    'settings.manualIntro': 'Run these in Terminal, in order, then reopen this page. They use the same tools and folder as the app.',
    'settings.manualVenv': '1. Create the Python environment (skip if it already exists)',
    'settings.manualPkg': '2. Install the Camoufox package',
    'settings.manualFetch': '3. Download the browser',
    'settings.uvMissing': 'uv was not found on this machine. Install it first:',
    'settings.installFailed': 'Install failed: {msg}',
    'settings.keys': 'API keys',
    'settings.keysIntro': 'One key per agent or machine, sent as the X-API-Key header (the CLI reads CM_API_KEY). This window never needs a key.',
    'settings.requireKey': 'Require an API key for non-UI clients',
    'settings.requireKeyHint': 'When on, scripts and agents without a valid key get 401. Create a key first.',
    'settings.keyName': 'Key name',
    'settings.keyNamePh': 'e.g. claude on laptop',
    'settings.keyCreate': 'Create key',
    'settings.keyNone': 'No keys yet.',
    'settings.colName': 'Name',
    'settings.colCreated': 'Created',
    'settings.colLastUsed': 'Last used',
    'settings.never': 'never',
    'settings.revoked': 'revoked',
    'settings.revoke': 'Revoke',
    'settings.revokeTitle': 'Revoke this key?',
    'settings.revokeBody': 'Clients using "{name}" will get 401 immediately. This cannot be undone.',
    'settings.keyShownTitle': 'Copy your new key now',
    'settings.keyShownBody': 'It is shown only once. Only a hash is stored, so it cannot be recovered later.',
    'settings.copy': 'Copy',
    'settings.storage': 'Storage',
    'settings.dataDir': 'Data folder',
    'settings.reveal': 'Show in Finder',
    'settings.db': 'Database',
    'settings.ports': 'Ports',
    'settings.uiPort': 'App / API',
    'settings.relayPorts': 'Local proxy ports from',
    'settings.portsHint': 'Everything listens on 127.0.0.1 only. Change with CM_PORT and CM_PROXY_BASE_PORT.',
  },
  vi: {
    'settings.general': 'Chung',
    'settings.headless': 'Mặc định chạy headless',
    'settings.headlessHint': 'Mở trình duyệt không có cửa sổ, trừ khi có yêu cầu khác.',
    'settings.engine': 'Engine trình duyệt',
    'settings.engineStatus': 'Trạng thái',
    'settings.engineReady': 'Sẵn sàng',
    'settings.engineMissing': 'Chưa cài',
    'settings.engineBroken': 'Cần xử lý',
    'settings.pkgVersion': 'Gói Camoufox',
    'settings.browserVersion': 'Trình duyệt Camoufox',
    'settings.unknown': 'không rõ',
    'settings.install': 'Cài / Cập nhật Camoufox',
    'settings.installing': 'Đang cài…',
    'settings.installHint': 'Không bắt buộc. Tải khoảng 2,4 GB (trình duyệt + font). App vẫn chạy bình thường khi chưa cài, chỉ việc mở profile mới cần đến nó.',
    'settings.installLog': 'Nhật ký cài đặt',
    'settings.installDone': 'Đã cài xong Camoufox',
    'settings.installDoneBody': 'Xong rồi. Bạn có thể mở profile ngay.',
    'settings.installFailedTitle': 'Cài đặt chưa hoàn tất',
    'settings.retry': 'Thử lại',
    'settings.copyLog': 'Copy log',
    'settings.manual': 'Cài thủ công',
    'settings.manualIntro': 'Chạy lần lượt các lệnh này trong Terminal rồi mở lại trang này. Chúng dùng cùng công cụ và thư mục với app.',
    'settings.manualVenv': '1. Tạo môi trường Python (bỏ qua nếu đã có)',
    'settings.manualPkg': '2. Cài gói Camoufox',
    'settings.manualFetch': '3. Tải trình duyệt',
    'settings.uvMissing': 'Không tìm thấy uv trên máy này. Hãy cài uv trước:',
    'settings.installFailed': 'Cài đặt lỗi: {msg}',
    'settings.keys': 'API key',
    'settings.keysIntro': 'Mỗi agent hoặc máy một key, gửi trong header X-API-Key (CLI đọc CM_API_KEY). Cửa sổ này không bao giờ cần key.',
    'settings.requireKey': 'Bắt buộc API key cho client ngoài giao diện',
    'settings.requireKeyHint': 'Khi bật, script và agent không có key hợp lệ sẽ nhận 401. Hãy tạo key trước.',
    'settings.keyName': 'Tên key',
    'settings.keyNamePh': 'vd: claude trên laptop',
    'settings.keyCreate': 'Tạo key',
    'settings.keyNone': 'Chưa có key nào.',
    'settings.colName': 'Tên',
    'settings.colCreated': 'Tạo lúc',
    'settings.colLastUsed': 'Dùng gần nhất',
    'settings.never': 'chưa dùng',
    'settings.revoked': 'đã thu hồi',
    'settings.revoke': 'Thu hồi',
    'settings.revokeTitle': 'Thu hồi key này?',
    'settings.revokeBody': 'Client đang dùng "{name}" sẽ nhận 401 ngay. Không hoàn tác được.',
    'settings.keyShownTitle': 'Hãy sao chép key mới ngay',
    'settings.keyShownBody': 'Key chỉ hiện một lần. App chỉ lưu bản băm nên không xem lại được.',
    'settings.copy': 'Sao chép',
    'settings.storage': 'Lưu trữ',
    'settings.dataDir': 'Thư mục dữ liệu',
    'settings.reveal': 'Mở trong Finder',
    'settings.db': 'Cơ sở dữ liệu',
    'settings.ports': 'Cổng',
    'settings.uiPort': 'App / API',
    'settings.relayPorts': 'Cổng proxy local từ',
    'settings.portsHint': 'Tất cả chỉ lắng nghe trên 127.0.0.1. Đổi bằng CM_PORT và CM_PROXY_BASE_PORT.',
  },
});

CM.pages.settings = {
  setup() {
    const s = Vue.ref(null);
    const keys = Vue.ref([]);
    const log = Vue.reactive({ lines: [], next: 0, running: false, open: false, ok: null, error: null });
    const copied = Vue.ref('');
    const logEl = Vue.ref(null);
    const newKey = Vue.reactive({ name: '', busy: false, secret: null, copied: false });
    const revoke = Vue.reactive({ open: false, key: null, busy: false });
    let timer = null;

    const load = async () => {
      try { s.value = await api('/api/settings'); } catch (e) { toast(t('common.loadFailed', { msg: e.message }), true); }
    };
    const loadKeys = async () => {
      try { keys.value = (await api('/api/api-keys')).keys; } catch (e) { toast(t('common.loadFailed', { msg: e.message }), true); }
    };
    const patch = async body => {
      try { s.value = await api('/api/settings', { method: 'PUT', body: JSON.stringify(body) }); }
      catch (e) { toast(e.message, true); await load(); }
    };

    /* ---- install log polling ---- */
    const poll = async () => {
      timer = null;
      try {
        const r = await api('/api/engine/install?from=' + log.next);
        log.lines.push(...r.lines);
        log.next = r.next;
        log.running = r.running;
        log.ok = r.ok; log.error = r.error;
        await Vue.nextTick();
        if (logEl.value) logEl.value.scrollTop = logEl.value.scrollHeight;
        if (r.running) { timer = setTimeout(poll, 700); return; }
        await load();
        if (r.ok === true) toast(t('settings.installDone'));
        else if (r.ok === false) toast(t('settings.installFailed', { msg: r.error }), true);
      } catch (e) { log.running = false; toast(e.message, true); }
    };
    const install = async () => {
      try {
        await api('/api/engine/install', { method: 'POST', body: '{}' });
      } catch (e) {
        // 409: one is already running (e.g. started from the Profiles banner): just follow its log.
        if (e.status !== 409) { toast(e.message, true); return; }
      }
      if (timer) { clearTimeout(timer); timer = null; }
      Object.assign(log, { lines: [], next: 0, running: true, open: true, ok: null, error: null });
      poll();
    };

    /* ---- copy helpers (log + manual commands) ---- */
    const copyText = async (text, key) => {
      try { await navigator.clipboard.writeText(text); copied.value = key; toast(t('common.copied')); }
      catch (_) { /* clipboard blocked: the text stays selectable on the page */ }
    };
    const sh = v => "'" + String(v).replace(/'/g, "'\\''") + "'";
    const manual = Vue.computed(() => {
      const e = s.value && s.value.engine;
      // Same rule as the server-side install: only a `<venv>/bin/python` can be (re)created.
      if (!e || !e.venvPython || !/\/bin\/python$/.test(e.venvPython)) return null;
      const venv = e.venvPython.replace(/\/bin\/python$/, '');
      const uv = e.uvPath;
      const pkg = 'camoufox==' + e.camoufoxVersion;
      return {
        uv: !!uv,
        steps: uv ? [
          { k: 'venv', label: t('settings.manualVenv'), cmd: sh(uv) + ' venv --python 3.12 ' + sh(venv) },
          { k: 'pkg', label: t('settings.manualPkg'), cmd: sh(uv) + ' pip install --python ' + sh(e.venvPython) + ' ' + pkg },
          { k: 'fetch', label: t('settings.manualFetch'), cmd: sh(e.venvPython) + ' -m camoufox fetch' },
        ] : [],
      };
    });

    /* ---- API keys ---- */
    const createKey = async () => {
      newKey.busy = true;
      try {
        const k = await api('/api/api-keys', { method: 'POST', body: JSON.stringify({ name: newKey.name }) });
        newKey.secret = k.key; newKey.copied = false; newKey.name = '';
        await loadKeys();
      } catch (e) { toast(e.message, true); }
      newKey.busy = false;
    };
    const copyKey = async () => {
      try { await navigator.clipboard.writeText(newKey.secret); newKey.copied = true; toast(t('common.copied')); }
      catch (_) { /* clipboard blocked: the key stays selectable in the dialog */ }
    };
    const doRevoke = async () => {
      revoke.busy = true;
      try { await api('/api/api-keys/' + revoke.key.id, { method: 'DELETE' }); revoke.open = false; await loadKeys(); }
      catch (e) { toast(e.message, true); }
      revoke.busy = false;
    };
    const reveal = async () => {
      try { await api('/api/settings/reveal-data', { method: 'POST', body: '{}' }); } catch (e) { toast(e.message, true); }
    };

    Vue.onMounted(async () => {
      await Promise.all([load(), loadKeys()]);
      // Arrived from the Profiles banner (#/settings/install): start now, then drop the flag so a reload does not repeat it.
      const auto = location.hash === '#/settings/install';
      if (auto) history.replaceState(null, '', '#/settings');
      if (s.value && s.value.engine.installRunning) { log.open = true; log.running = true; poll(); }
      else if (auto && s.value && !s.value.engine.ready) install();
    });
    Vue.onUnmounted(() => { if (timer) clearTimeout(timer); });

    const when = iso => iso ? new Date(iso).toLocaleString() : null;
    const state = Vue.computed(() => {
      const e = s.value && s.value.engine;
      if (!e) return { label: '…', dot: '' };
      if (e.ready) return { label: t('settings.engineReady'), dot: 'bg-ok' };
      return e.hint && e.hint.code === 'not_installed'
        ? { label: t('settings.engineMissing'), dot: 'bg-faint' }
        : { label: t('settings.engineBroken'), dot: 'bg-bad' };
    });
    const hasActiveKey = Vue.computed(() => keys.value.some(k => !k.revokedAt));

    return {
      s, keys, log, logEl, copied, manual, copyText, newKey, revoke, state, hasActiveKey, when,
      LANGS: I18N.LANGS, locale: I18N.locale, setLanguage: CM.setLanguage,
      patch, install, createKey, copyKey, doRevoke, reveal,
    };
  },
  template: `<cm-page :title="t('nav.settings')">
    <div class="w-full max-w-[720px] mx-auto flex flex-col gap-6 py-2 overflow-auto">
      <section>
        <h2 class="section-title">{{ t('settings.general') }}</h2>
        <div class="kv">
          <div class="kv-row"><span>{{ t('common.language') }}</span>
            <select class="input max-w-[220px]" :value="locale" @change="setLanguage($event.target.value)">
              <option v-for="l in LANGS" :key="l.k" :value="l.k">{{ l.t }}</option>
            </select>
          </div>
          <div class="kv-row"><span>{{ t('settings.headless') }}</span>
            <div class="flex flex-col gap-1">
              <cm-switch :model-value="!!(s && s.headless)" :disabled="!s" @update:model-value="patch({ headless: $event })"/>
              <span class="text-xs text-faint">{{ t('settings.headlessHint') }}</span>
            </div>
          </div>
        </div>
      </section>

      <section>
        <h2 class="section-title">{{ t('settings.engine') }}</h2>
        <div class="kv">
          <div class="kv-row"><span>{{ t('settings.engineStatus') }}</span>
            <div class="flex flex-col gap-1">
              <span class="flex items-center gap-2"><i class="dot" :class="state.dot"></i>{{ s ? s.engine.name : '' }} · {{ state.label }}</span>
              <span v-if="s && s.engine.hint" class="text-xs text-faint break-all">{{ s.engine.hint.detail }}</span>
            </div>
          </div>
          <div class="kv-row"><span>{{ t('settings.pkgVersion') }}</span><code>{{ s && s.engine.versions && s.engine.versions.package || t('settings.unknown') }}</code></div>
          <div class="kv-row"><span>{{ t('settings.browserVersion') }}</span><code>{{ s && s.engine.versions && s.engine.versions.browser || t('settings.unknown') }}</code></div>
          <div class="kv-row"><span></span>
            <div class="flex flex-col gap-2">
              <div><button class="btn" :disabled="log.running || !s" @click="install"><i v-if="log.running" class="spin"></i>{{ log.running ? t('settings.installing') : t('settings.install') }}</button></div>
              <span class="text-xs text-faint">{{ t('settings.installHint') }}</span>
            </div>
          </div>
        </div>
        <div v-if="log.ok === true" class="banner" style="margin-top:10px"><cm-icon name="check"/><div><b>{{ t('settings.installDone') }}</b> — {{ t('settings.installDoneBody') }}</div></div>
        <div v-if="log.ok === false" class="banner" style="margin-top:10px">
          <cm-icon name="alert"/>
          <div class="flex-1 min-w-0"><b>{{ t('settings.installFailedTitle') }}</b><br><span class="opacity-85 break-all">{{ log.error }}</span></div>
          <button class="btn btn-start" :disabled="log.running" @click="install">{{ t('settings.retry') }}</button>
        </div>
        <details v-if="log.ok === false && manual" style="margin-top:10px">
          <summary class="cursor-pointer text-[13px]">{{ t('settings.manual') }}</summary>
          <p class="m-0 text-xs text-faint" style="margin:6px 0">{{ t('settings.manualIntro') }}</p>
          <p v-if="!manual.uv" class="m-0 text-xs text-bad">{{ t('settings.uvMissing') }} <a href="https://docs.astral.sh/uv/" target="_blank" rel="noopener">https://docs.astral.sh/uv/</a></p>
          <div v-for="st in manual.steps" :key="st.k" class="flex flex-col gap-1" style="margin-bottom:8px">
            <span class="text-xs text-faint">{{ st.label }}</span>
            <div class="flex items-start gap-2">
              <code class="break-all flex-1 min-w-0 p-2 rounded-lg border border-line bg-panel2 text-[12px]" style="user-select:all">{{ st.cmd }}</code>
              <button class="btn" @click="copyText(st.cmd, st.k)"><cm-icon :name="copied === st.k ? 'check' : 'copy'" :size="14"/> {{ t('settings.copy') }}</button>
            </div>
          </div>
        </details>
        <div v-if="log.open" class="flex flex-col gap-1" style="margin-top:10px">
          <div class="flex items-center gap-2">
            <span class="text-xs text-faint flex-1">{{ t('settings.installLog') }}</span>
            <button v-if="log.lines.length" class="btn" @click="copyText(log.lines.join('\\n'), 'log')"><cm-icon :name="copied === 'log' ? 'check' : 'copy'" :size="14"/> {{ t('settings.copyLog') }}</button>
          </div>
          <pre ref="logEl" class="m-0 p-3 rounded-lg border border-line bg-panel2 font-mono text-[12px] overflow-auto max-h-[260px]" style="white-space:pre-wrap;word-break:break-all">{{ log.lines.join('\\n') }}</pre>
        </div>
      </section>

      <section>
        <h2 class="section-title">{{ t('settings.keys') }}</h2>
        <p class="m-0 text-[13px] text-muted" style="margin-bottom:8px">{{ t('settings.keysIntro') }}</p>
        <div class="kv">
          <div class="kv-row"><span>{{ t('settings.requireKey') }}</span>
            <div class="flex flex-col gap-1">
              <cm-switch :model-value="!!(s && s.requireApiKey)" :disabled="!s" @update:model-value="patch({ requireApiKey: $event })"/>
              <span class="text-xs text-faint">{{ t('settings.requireKeyHint') }}</span>
              <span v-if="s && s.requireApiKey && !hasActiveKey" class="text-xs text-bad">{{ t('settings.keyNone') }}</span>
            </div>
          </div>
          <div class="kv-row"><span>{{ t('settings.keyName') }}</span>
            <form class="flex gap-2 flex-wrap" @submit.prevent="newKey.name.trim() && createKey()">
              <input class="input flex-1 min-w-0" v-model="newKey.name" maxlength="60" autocomplete="off" :placeholder="t('settings.keyNamePh')">
              <button class="btn btn-start" type="submit" :disabled="newKey.busy || !newKey.name.trim()">{{ t('settings.keyCreate') }}</button>
            </form>
          </div>
        </div>
        <div v-if="keys.length" class="kv" style="margin-top:10px">
          <table class="list-tbl">
            <thead><tr><th>{{ t('settings.colName') }}</th><th>{{ t('settings.colCreated') }}</th><th>{{ t('settings.colLastUsed') }}</th><th></th></tr></thead>
            <tbody>
              <tr v-for="k in keys" :key="k.id" :class="k.revokedAt ? 'opacity-50' : ''">
                <td>{{ k.name }} <span v-if="k.revokedAt" class="chip">{{ t('settings.revoked') }}</span><br><code class="text-xs text-faint">cmk_{{ k.id }}_…</code></td>
                <td>{{ when(k.createdAt) }}</td>
                <td>{{ when(k.lastUsedAt) || t('settings.never') }}</td>
                <td><button v-if="!k.revokedAt" class="btn" @click="revoke.key = k; revoke.open = true">{{ t('settings.revoke') }}</button></td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      <section>
        <h2 class="section-title">{{ t('settings.storage') }}</h2>
        <div class="kv">
          <div class="kv-row"><span>{{ t('settings.dataDir') }}</span>
            <div class="flex items-center gap-2 flex-wrap"><code class="break-all flex-1 min-w-0">{{ s ? s.dataDir : '…' }}</code><button class="btn" :disabled="!s" @click="reveal">{{ t('settings.reveal') }}</button></div>
          </div>
          <div class="kv-row"><span>{{ t('settings.db') }}</span><code class="break-all">{{ s ? (s.dbPath || '—') : '…' }}</code></div>
          <div class="kv-row"><span>{{ t('settings.uiPort') }}</span><code>{{ s && s.uiPort ? '127.0.0.1:' + s.uiPort : '—' }}</code></div>
          <div class="kv-row"><span>{{ t('settings.relayPorts') }}</span><code>{{ s ? '127.0.0.1:' + s.proxyBasePort : '…' }}</code></div>
        </div>
        <p class="m-0 text-xs text-faint" style="margin-top:6px">{{ t('settings.portsHint') }}</p>
      </section>

      <cm-doctor></cm-doctor>
    </div>

    <cm-dialog :open="revoke.open" :title="t('settings.revokeTitle')" :busy="revoke.busy" @close="revoke.open = false">
      <p class="m-0 text-[13px] text-muted">{{ t('settings.revokeBody', { name: revoke.key && revoke.key.name }) }}</p>
      <template #footer>
        <button class="btn" :disabled="revoke.busy" @click="revoke.open = false">{{ t('common.cancel') }}</button>
        <button class="btn btn-stop" :disabled="revoke.busy" @click="doRevoke">{{ t('settings.revoke') }}</button>
      </template>
    </cm-dialog>

    <cm-dialog :open="!!newKey.secret" :title="t('settings.keyShownTitle')" :width="520" @close="newKey.secret = null">
      <p class="m-0 text-[13px] text-muted">{{ t('settings.keyShownBody') }}</p>
      <code class="break-all p-3 rounded-lg border border-line bg-panel2 text-[12.5px]" style="user-select:all">{{ newKey.secret }}</code>
      <template #footer>
        <button class="btn" @click="copyKey"><cm-icon :name="newKey.copied ? 'check' : 'copy'" :size="14"/> {{ t('settings.copy') }}</button>
        <button class="btn btn-start" @click="newKey.secret = null">{{ t('common.close') }}</button>
      </template>
    </cm-dialog>
  </cm-page>`,
};
