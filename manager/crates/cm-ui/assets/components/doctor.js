/* Doctor panel (phase 19), a section of the Settings page: <cm-doctor/>.
   Run = GET /api/doctor (read-only checks on this computer). Each check has a status
   (ok / warn / fail), what was found, and for problems a plain-language fix. "Copy report" puts
   the server's markdown report (no proxy passwords, API keys or cookie values) on the clipboard.
   Titles come from doctor.check.<id>, fixes from doctor.fix.<code>; the server's English text is
   the fallback. Nothing is sent anywhere. */

I18N.add({
  en: {
    'doctor.title': 'Doctor',
    'doctor.intro': 'Checks that this app, Camoufox, the database and the local proxy relay work, and tells you how to fix what does not.',
    'doctor.run': 'Run checks',
    'doctor.running': 'Checking…',
    'doctor.copy': 'Copy report',
    'doctor.copied': 'Report copied',
    'doctor.summary': '{ok} ok · {warn} warning · {fail} failed',
    'doctor.fixLabel': 'How to fix',
    'doctor.privacy': 'Privacy: no telemetry. Checks run on this computer and nothing is sent anywhere. The copied report holds no proxy passwords, API keys or cookie values.',
    'doctor.status.ok': 'OK',
    'doctor.status.warn': 'Warning',
    'doctor.status.fail': 'Failed',
    'doctor.check.camoufox.package': 'Camoufox package',
    'doctor.check.camoufox.browser': 'Camoufox browser',
    'doctor.check.helper.files': 'Helper files',
    'doctor.check.data.dir': 'Data folder',
    'doctor.check.db.writable': 'Database writable',
    'doctor.check.db.integrity': 'Database integrity',
    'doctor.check.port.ui': 'App port',
    'doctor.check.port.proxy': 'Proxy relay ports',
    'doctor.check.relay.failclosed': 'Proxy relay fails closed',
    'doctor.check.processes.orphans': 'Leftover processes',
    'doctor.check.journals': 'Interrupted imports',
    'doctor.check.tailscale': 'Tailscale',
    'doctor.fix.install_engine': 'Open Settings, then "Install / Update Camoufox" (or run manager/camoufox/setup.sh in a terminal). Camoufox is optional: the rest of the app works without it.',
    'doctor.fix.fetch_browser': 'The package is there but the browser build is not downloaded. Press "Install / Update Camoufox" above.',
    'doctor.fix.restart_app': 'Restart the app: it rewrites the helper files at startup. If they keep disappearing, check that the data folder is writable.',
    'doctor.fix.data_dir': 'The data folder is not writable. Fix its permissions or free disk space, then run the checks again.',
    'doctor.fix.db_writable': 'Another program may hold the database or the file is read-only. Close other copies of the app and check the file permissions.',
    'doctor.fix.db_repair': 'The database file is damaged. Quit the app and keep a copy of camoufox-manager.db before doing anything else; restore the newest backup if you have one. Do not delete it: your profiles are listed in it.',
    'doctor.fix.ui_port': 'Start the app with another port (CM_PORT=<n>) and open that address.',
    'doctor.fix.proxy_ports': 'Quit the program that uses the port, or start the app with a different CM_PROXY_BASE_PORT (for example 41000).',
    'doctor.fix.relay': 'Do not start browsers with a proxy until this passes. Restart the app; if it still fails, the relay is broken and your real IP could leak.',
    'doctor.fix.orphans': 'A previous run left these behind. They are cleaned up when the app starts, so restart the app to clean them now. The doctor stopped nothing.',
    'doctor.fix.journals': 'An import was interrupted. Restart the app so it can finish or roll back by itself; do not delete those files by hand.',
    'doctor.fix.tailscale': 'Open the Tailscale app, sign in and turn it on. Only device sync needs it; everything else works without.',
  },
  vi: {
    'doctor.title': 'Chẩn đoán (Doctor)',
    'doctor.intro': 'Kiểm tra app, Camoufox, cơ sở dữ liệu và relay proxy local có chạy tốt không, và chỉ cách sửa chỗ nào chưa ổn.',
    'doctor.run': 'Chạy kiểm tra',
    'doctor.running': 'Đang kiểm tra…',
    'doctor.copy': 'Sao chép báo cáo',
    'doctor.copied': 'Đã sao chép báo cáo',
    'doctor.summary': '{ok} ổn · {warn} cảnh báo · {fail} lỗi',
    'doctor.fixLabel': 'Cách sửa',
    'doctor.privacy': 'Riêng tư: không có telemetry. Mọi kiểm tra chạy trên máy này và không gửi gì đi đâu. Báo cáo sao chép không chứa mật khẩu proxy, API key hay giá trị cookie.',
    'doctor.status.ok': 'Ổn',
    'doctor.status.warn': 'Cảnh báo',
    'doctor.status.fail': 'Lỗi',
    'doctor.check.camoufox.package': 'Gói Camoufox',
    'doctor.check.camoufox.browser': 'Trình duyệt Camoufox',
    'doctor.check.helper.files': 'Tệp helper',
    'doctor.check.data.dir': 'Thư mục dữ liệu',
    'doctor.check.db.writable': 'Ghi được vào cơ sở dữ liệu',
    'doctor.check.db.integrity': 'Toàn vẹn cơ sở dữ liệu',
    'doctor.check.port.ui': 'Cổng của app',
    'doctor.check.port.proxy': 'Cổng relay proxy',
    'doctor.check.relay.failclosed': 'Relay proxy chặn khi proxy chết',
    'doctor.check.processes.orphans': 'Tiến trình còn sót',
    'doctor.check.journals': 'Import bị ngắt giữa chừng',
    'doctor.check.tailscale': 'Tailscale',
    'doctor.fix.install_engine': 'Vào Cài đặt, bấm "Cài / Cập nhật Camoufox" (hoặc chạy manager/camoufox/setup.sh trong terminal). Camoufox không bắt buộc: phần còn lại của app vẫn chạy.',
    'doctor.fix.fetch_browser': 'Đã có gói nhưng chưa tải trình duyệt. Bấm "Cài / Cập nhật Camoufox" ở trên.',
    'doctor.fix.restart_app': 'Mở lại app: app ghi lại các tệp helper khi khởi động. Nếu chúng cứ biến mất, kiểm tra thư mục dữ liệu có ghi được không.',
    'doctor.fix.data_dir': 'Không ghi được vào thư mục dữ liệu. Sửa quyền truy cập hoặc giải phóng dung lượng rồi chạy kiểm tra lại.',
    'doctor.fix.db_writable': 'Có thể chương trình khác đang giữ cơ sở dữ liệu hoặc tệp chỉ đọc. Đóng các bản app khác và kiểm tra quyền của tệp.',
    'doctor.fix.db_repair': 'Tệp cơ sở dữ liệu bị hỏng. Hãy thoát app và sao lưu camoufox-manager.db trước khi làm gì khác; khôi phục bản sao lưu mới nhất nếu có. Đừng xoá: danh sách profile nằm trong đó.',
    'doctor.fix.ui_port': 'Mở app với cổng khác (CM_PORT=<n>) rồi truy cập địa chỉ đó.',
    'doctor.fix.proxy_ports': 'Tắt chương trình đang dùng cổng đó, hoặc mở app với CM_PROXY_BASE_PORT khác (ví dụ 41000).',
    'doctor.fix.relay': 'Đừng mở trình duyệt với proxy cho tới khi mục này đạt. Mở lại app; nếu vẫn lỗi thì relay hỏng và IP thật của bạn có thể bị lộ.',
    'doctor.fix.orphans': 'Lần chạy trước để lại các tiến trình này. App tự dọn khi khởi động, nên mở lại app để dọn ngay. Doctor không dừng gì cả.',
    'doctor.fix.journals': 'Một lần import bị ngắt. Mở lại app để nó tự hoàn tất hoặc quay lui; đừng xoá các tệp đó bằng tay.',
    'doctor.fix.tailscale': 'Mở app Tailscale, đăng nhập và bật lên. Chỉ đồng bộ thiết bị mới cần; các tính năng khác vẫn chạy.',
  },
});

CM.components.CmDoctor = {
  setup() {
    const report = Vue.ref(null);
    const busy = Vue.ref(false);
    const failed = Vue.ref('');

    const run = async () => {
      busy.value = true; failed.value = '';
      try { report.value = await api('/api/doctor'); }
      catch (e) { failed.value = e.message; }
      busy.value = false;
    };
    const copy = async () => {
      const text = report.value.markdown;
      try { await navigator.clipboard.writeText(text); }
      catch (_) {
        /* clipboard API blocked: fall back to a hidden textarea */
        const ta = document.createElement('textarea');
        ta.value = text; ta.style.position = 'fixed'; ta.style.opacity = '0';
        document.body.appendChild(ta); ta.select();
        try { document.execCommand('copy'); } finally { ta.remove(); }
      }
      toast(t('doctor.copied'));
    };
    const label = (prefix, key, fallback) => { const k = prefix + key, s = t(k); return s === k ? fallback : s; };
    return {
      report, busy, failed, run, copy,
      title: c => label('doctor.check.', c.id, c.id),
      fix: c => c.fix ? label('doctor.fix.', c.fix, c.hint || '') : '',
      dot: c => c.status === 'ok' ? 'bg-ok' : c.status === 'warn' ? 'bg-warn' : 'bg-bad',
      statusText: c => t('doctor.status.' + c.status),
    };
  },
  template: `<section>
    <h2 class="section-title">{{ t('doctor.title') }}</h2>
    <p class="m-0 text-[13px] text-muted" style="margin-bottom:8px">{{ t('doctor.intro') }}</p>
    <div class="flex items-center gap-2 flex-wrap" style="margin-bottom:10px">
      <button class="btn btn-start" :disabled="busy" @click="run"><i v-if="busy" class="spin"></i>{{ busy ? t('doctor.running') : t('doctor.run') }}</button>
      <button class="btn" :disabled="!report" @click="copy"><cm-icon name="copy" :size="14"/> {{ t('doctor.copy') }}</button>
      <span v-if="report" class="text-[13px] text-muted">{{ t('doctor.summary', report.summary) }}</span>
    </div>
    <p v-if="failed" class="m-0 text-[13px] text-bad" style="margin-bottom:8px">{{ t('common.loadFailed', { msg: failed }) }}</p>
    <div v-if="report" class="kv">
      <div v-for="c in report.checks" :key="c.id" class="flex gap-3 px-4 py-3 border-b border-line last:border-b-0">
        <i class="dot" :class="dot(c)" style="margin-top:6px" :title="statusText(c)"></i>
        <div class="min-w-0 flex-1 flex flex-col gap-1">
          <div class="flex items-center gap-2 flex-wrap">
            <span class="text-[13.5px] font-semibold">{{ title(c) }}</span>
            <span class="chip" :class="c.status === 'fail' ? 'text-bad' : c.status === 'warn' ? 'text-warn' : ''">{{ statusText(c) }}</span>
          </div>
          <span class="text-[12.5px] text-muted break-words">{{ c.detail }}</span>
          <span v-if="c.fix" class="text-[12.5px] break-words"><b>{{ t('doctor.fixLabel') }}:</b> {{ fix(c) }}</span>
        </div>
      </div>
    </div>
    <p class="m-0 text-xs text-faint" style="margin-top:8px">{{ t('doctor.privacy') }}</p>
  </section>`,
};
