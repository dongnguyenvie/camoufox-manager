/* Cookies dialog (phase 5): export / import the cookies of a CLOSED profile.
   <cm-cookies :profile="p" @close="p = null" @done="reload"/>  — `profile` = { id, name, running? } or null (closed).
   Export downloads JSON (browser-extension format) or Netscape cookies.txt. Import takes a file or pasted
   text (format auto-detected); "only the domains in the file" keeps other sessions, "replace all" clears first.
   API: GET /api/cookies/{id}/export?format=…  ·  POST /api/cookies/{id}/import {content, format, mode}. */

I18N.add({
  en: {
    'cookies.title': 'Cookies · {name}',
    'cookies.running': 'The profile is running. Stop it first: its cookies can only be read or changed while it is closed.',
    'cookies.export': 'Export',
    'cookies.exportHint': 'Save this profile’s cookies as a file.',
    'cookies.formatJson': 'JSON (browser extensions)',
    'cookies.formatNetscape': 'Netscape cookies.txt',
    'cookies.download': 'Download',
    'cookies.exported': 'Exported {n} cookies',
    'cookies.import': 'Import',
    'cookies.importHint': 'Choose a file or paste the text. JSON and Netscape cookies.txt are detected automatically.',
    'cookies.chooseFile': 'Choose file…',
    'cookies.paste': 'Or paste cookies here',
    'cookies.modeDomains': 'Replace only the domains in the file',
    'cookies.modeDomainsHint': 'Cookies of other sites stay as they are.',
    'cookies.modeAll': 'Replace all cookies',
    'cookies.modeAllHint': 'Every existing cookie of this profile is deleted first.',
    'cookies.run': 'Import cookies',
    'cookies.importing': 'Importing…',
    'cookies.empty': 'Choose a file or paste some cookies first',
    'cookies.report': 'Imported {imported} of {total} cookies',
    'cookies.reportDomains': 'Domains: {list}',
    'cookies.reportSession': '{n} session cookies were kept for 30 days so they survive a restart.',
    'cookies.skipped': 'Skipped {n}',
    'cookies.skip.badLine': 'unreadable line or entry',
    'cookies.skip.missingName': 'no name',
    'cookies.skip.badName': 'invalid name or value',
    'cookies.skip.badDomain': 'invalid domain',
    'cookies.skip.badPath': 'invalid path',
    'cookies.skip.expired': 'already expired',
    'cookies.skip.duplicate': 'duplicate (the later one was used)',
    'cookies.close': 'Close',
  },
  vi: {
    'cookies.title': 'Cookie · {name}',
    'cookies.running': 'Profile đang chạy. Hãy dừng trước: chỉ đọc/sửa được cookie khi profile đã đóng.',
    'cookies.export': 'Xuất',
    'cookies.exportHint': 'Lưu cookie của profile này thành file.',
    'cookies.formatJson': 'JSON (extension trình duyệt)',
    'cookies.formatNetscape': 'Netscape cookies.txt',
    'cookies.download': 'Tải xuống',
    'cookies.exported': 'Đã xuất {n} cookie',
    'cookies.import': 'Nhập',
    'cookies.importHint': 'Chọn file hoặc dán nội dung. Tự nhận diện JSON và Netscape cookies.txt.',
    'cookies.chooseFile': 'Chọn file…',
    'cookies.paste': 'Hoặc dán cookie vào đây',
    'cookies.modeDomains': 'Chỉ thay các domain có trong file',
    'cookies.modeDomainsHint': 'Cookie của các trang khác được giữ nguyên.',
    'cookies.modeAll': 'Thay toàn bộ cookie',
    'cookies.modeAllHint': 'Xoá hết cookie hiện có của profile trước khi nhập.',
    'cookies.run': 'Nhập cookie',
    'cookies.importing': 'Đang nhập…',
    'cookies.empty': 'Hãy chọn file hoặc dán cookie trước',
    'cookies.report': 'Đã nhập {imported}/{total} cookie',
    'cookies.reportDomains': 'Domain: {list}',
    'cookies.reportSession': '{n} cookie phiên được giữ 30 ngày để còn sau khi khởi động lại.',
    'cookies.skipped': 'Bỏ qua {n}',
    'cookies.skip.badLine': 'dòng/mục không đọc được',
    'cookies.skip.missingName': 'thiếu tên',
    'cookies.skip.badName': 'tên hoặc giá trị không hợp lệ',
    'cookies.skip.badDomain': 'domain không hợp lệ',
    'cookies.skip.badPath': 'path không hợp lệ',
    'cookies.skip.expired': 'đã hết hạn',
    'cookies.skip.duplicate': 'trùng (dùng mục sau)',
    'cookies.close': 'Đóng',
  },
});

CM.components.CmCookies = {
  props: { profile: Object },
  emits: ['close', 'done'],
  setup(props, { emit }) {
    const format = Vue.ref('json');
    const text = Vue.ref('');
    const fileName = Vue.ref('');
    const mode = Vue.ref('domains');
    const busy = Vue.ref(false);
    const report = Vue.ref(null);
    const fileInput = Vue.ref(null);

    // Fresh state each time the dialog opens for a profile.
    Vue.watch(() => props.profile && props.profile.id, () => {
      text.value = ''; fileName.value = ''; report.value = null; mode.value = 'domains'; busy.value = false;
    });

    const locked = Vue.computed(() => !!(props.profile && props.profile.running));

    async function download() {
      busy.value = true;
      try {
        const r = await fetch(`/api/cookies/${props.profile.id}/export?format=${format.value}`);
        if (!r.ok) throw new Error((await r.json().catch(() => ({}))).error || r.statusText);
        const n = r.headers.get('x-cookie-count') || '0';
        const blob = await r.blob();
        const a = document.createElement('a');
        a.href = URL.createObjectURL(blob);
        a.download = `cookies-${(props.profile.name || props.profile.id).replace(/[^\w.-]+/g, '_')}.${format.value === 'json' ? 'json' : 'txt'}`;
        document.body.appendChild(a); a.click(); a.remove();
        setTimeout(() => URL.revokeObjectURL(a.href), 1000);
        toast(t('cookies.exported', { n }));
      } catch (e) { toast(e.message, true); }
      busy.value = false;
    }

    function pick(ev) {
      const f = ev.target.files && ev.target.files[0];
      if (!f) return;
      const reader = new FileReader();
      reader.onload = () => { text.value = String(reader.result || ''); fileName.value = f.name; report.value = null; };
      reader.readAsText(f);
      ev.target.value = '';
    }

    async function run() {
      if (!text.value.trim()) { toast(t('cookies.empty'), true); return; }
      busy.value = true; report.value = null;
      try {
        report.value = await api(`/api/cookies/${props.profile.id}/import`, {
          method: 'POST', body: JSON.stringify({ content: text.value, mode: mode.value }),
        });
        if (report.value.imported > 0) emit('done');
      } catch (e) { toast(e.message, true); }
      busy.value = false;
    }

    const skipText = c => t('cookies.skip.' + c.code) + ' × ' + c.count;
    return { format, text, fileName, mode, busy, report, fileInput, locked, download, pick, run, skipText };
  },
  template: `<cm-dialog :open="!!profile" :title="profile ? t('cookies.title', { name: profile.name }) : ''" :busy="busy" :width="520" @close="$emit('close')">
    <div v-if="locked" class="rounded-lg border border-line bg-bg px-3 py-2 text-[13px] text-bad">{{ t('cookies.running') }}</div>
    <template v-else>
      <section class="flex flex-col gap-2">
        <h3 class="m-0 text-[13px] font-semibold">{{ t('cookies.export') }}</h3>
        <p class="m-0 text-xs text-muted">{{ t('cookies.exportHint') }}</p>
        <div class="flex gap-2">
          <select class="input flex-1" v-model="format" :disabled="busy">
            <option value="json">{{ t('cookies.formatJson') }}</option>
            <option value="netscape">{{ t('cookies.formatNetscape') }}</option>
          </select>
          <button class="btn" :disabled="busy" @click="download">{{ t('cookies.download') }}</button>
        </div>
      </section>
      <hr class="border-line w-full">
      <section class="flex flex-col gap-2">
        <h3 class="m-0 text-[13px] font-semibold">{{ t('cookies.import') }}</h3>
        <p class="m-0 text-xs text-muted">{{ t('cookies.importHint') }}</p>
        <div class="flex items-center gap-2">
          <button class="btn" :disabled="busy" @click="fileInput.click()">{{ t('cookies.chooseFile') }}</button>
          <span class="text-xs text-muted truncate">{{ fileName }}</span>
          <input ref="fileInput" type="file" accept=".json,.txt,text/plain,application/json" class="hidden" @change="pick">
        </div>
        <textarea class="input font-mono text-xs h-28" v-model="text" :placeholder="t('cookies.paste')" :disabled="busy" spellcheck="false"></textarea>
        <label class="flex items-start gap-2 text-[13px] cursor-pointer">
          <input type="radio" value="domains" v-model="mode" class="mt-1"><span>{{ t('cookies.modeDomains') }}<br><span class="text-xs text-faint">{{ t('cookies.modeDomainsHint') }}</span></span>
        </label>
        <label class="flex items-start gap-2 text-[13px] cursor-pointer">
          <input type="radio" value="all" v-model="mode" class="mt-1"><span>{{ t('cookies.modeAll') }}<br><span class="text-xs text-faint">{{ t('cookies.modeAllHint') }}</span></span>
        </label>
        <div v-if="report" class="rounded-lg border border-line bg-bg px-3 py-2 text-[13px] flex flex-col gap-1">
          <b>{{ t('cookies.report', { imported: report.imported, total: report.total }) }}</b>
          <span v-if="report.domains.length" class="text-xs text-muted break-words">{{ t('cookies.reportDomains', { list: report.domains.join(', ') }) }}</span>
          <span v-if="report.sessionExtended" class="text-xs text-muted">{{ t('cookies.reportSession', { n: report.sessionExtended }) }}</span>
          <template v-if="report.skipped">
            <span class="text-xs text-bad">{{ t('cookies.skipped', { n: report.skipped }) }}</span>
            <ul class="m-0 pl-4 text-xs text-muted"><li v-for="c in report.skippedReasons" :key="c.code">{{ skipText(c) }}</li></ul>
          </template>
        </div>
      </section>
    </template>
    <template #footer>
      <button class="btn" :disabled="busy" @click="$emit('close')">{{ t('cookies.close') }}</button>
      <button v-if="!locked" class="btn btn-start":disabled="busy || !text.trim()" @click="run">{{ busy ? t('cookies.importing') : t('cookies.run') }}</button>
    </template>
  </cm-dialog>`,
};
