/* Transfer dialogs (phase 6): export / import / clone / bulk create.
   <cm-transfer :export-profile="row|null" :clone-profile="row|null"
                @close-export="..." @close-clone="..." @changed="reload"/>
   Renders two toolbar buttons (Import, Bulk create) and the four dialogs. The profiles page opens
   export/clone for a row by passing that row; `changed` fires after anything was created.
   Export needs a closed profile and writes one zip (fingerprint + settings + browser data).
   Proxy passwords are never exported, the zip holds the proxy label only. */

I18N.add({
  en: {
    'transfer.import': 'Import',
    'transfer.importTitle': 'Import a profile from a zip file',
    'transfer.bulk': 'Bulk create',
    'transfer.bulkTitle': 'Create many profiles from a name pattern',
    'transfer.export': 'Export profile',
    'transfer.clone': 'Clone profile',
    'transfer.exportTitle': 'Export “{name}”',
    'transfer.exportBody': 'Writes one zip with the pinned fingerprint, settings and all browser data (cookies, logins, history). Lock files and caches are left out. Proxy passwords are never included.',
    'transfer.exportSensitive': 'The zip contains logged-in sessions. Keep it private.',
    'transfer.exportRunning': 'Stop the profile first: it must be closed to export.',
    'transfer.exportGo': 'Export',
    'transfer.exporting': 'Exporting…',
    'transfer.exported': 'Exported {n} files ({size})',
    'transfer.exportPath': 'Saved to',
    'transfer.download': 'Download',
    'transfer.skipped': 'Skipped {n} entries that are not plain files: {list}',
    'transfer.importTitleDlg': 'Import profile',
    'transfer.importBody': 'Choose a zip made by Export. Its checksums are verified before anything is kept.',
    'transfer.file': 'Zip file',
    'transfer.nameOverride': 'Name (optional)',
    'transfer.nameOverrideHint': 'Empty = the name stored in the zip.',
    'transfer.idMode': 'Profile ID',
    'transfer.idNew': 'New ID (default)',
    'transfer.idKeep': 'Keep the original ID when it is free',
    'transfer.importGo': 'Import',
    'transfer.importing': 'Importing…',
    'transfer.imported': 'Imported {name}',
    'transfer.keptId': 'Kept the original ID.',
    'transfer.proxyNote': 'It used proxy {label} on the source machine. Proxies are not transferred, assign one here.',
    'transfer.warn.platform': 'Platform differs: {detail}. The browser data may not work here.',
    'transfer.warn.build': 'Browser build differs: {detail}.',
    'transfer.noFile': 'Choose a zip file first',
    'transfer.cloneTitle': 'Clone “{name}”',
    'transfer.cloneBody': 'Copies settings and browser data into a new profile. The profile must be closed. Both copies will share the same logged-in sessions: use them one at a time.',
    'transfer.cloneName': 'Name of the copy',
    'transfer.cloneNewMachine': 'Pin a new machine (new fingerprint) instead of copying it',
    'transfer.cloneGo': 'Clone',
    'transfer.cloning': 'Cloning…',
    'transfer.cloned': 'Cloned as {name}',
    'transfer.copySuffix': '{name} (copy)',
    'transfer.bulkTitleDlg': 'Bulk create profiles',
    'transfer.pattern': 'Name pattern',
    'transfer.patternHint': '{n} becomes the number, e.g. Shop {n} gives Shop 1, Shop 2…',
    'transfer.count': 'How many',
    'transfer.start': 'First number',
    'transfer.os': 'Fingerprint OS',
    'transfer.preview': 'Preview: {list}',
    'transfer.bulkGo': 'Create',
    'transfer.bulkBusy': 'Creating… this can take a few seconds per profile',
    'transfer.bulkDone': 'Created {n} profiles',
    'transfer.bulkPartial': 'Created {n}, then stopped: {error}',
  },
  vi: {
    'transfer.import': 'Nhập',
    'transfer.importTitle': 'Nhập profile từ file zip',
    'transfer.bulk': 'Tạo hàng loạt',
    'transfer.bulkTitle': 'Tạo nhiều profile từ mẫu tên',
    'transfer.export': 'Xuất profile',
    'transfer.clone': 'Nhân bản profile',
    'transfer.exportTitle': 'Xuất “{name}”',
    'transfer.exportBody': 'Ghi một file zip gồm vân tay đã ghim, cài đặt và toàn bộ dữ liệu trình duyệt (cookie, đăng nhập, lịch sử). Bỏ qua file khoá và cache. Mật khẩu proxy không bao giờ được xuất.',
    'transfer.exportSensitive': 'File zip chứa các phiên đã đăng nhập. Hãy giữ kín.',
    'transfer.exportRunning': 'Hãy dừng profile trước: phải đóng thì mới xuất được.',
    'transfer.exportGo': 'Xuất',
    'transfer.exporting': 'Đang xuất…',
    'transfer.exported': 'Đã xuất {n} file ({size})',
    'transfer.exportPath': 'Đã lưu tại',
    'transfer.download': 'Tải xuống',
    'transfer.skipped': 'Bỏ qua {n} mục không phải file thường: {list}',
    'transfer.importTitleDlg': 'Nhập profile',
    'transfer.importBody': 'Chọn file zip tạo bởi chức năng Xuất. Checksum được kiểm tra trước khi giữ lại bất cứ thứ gì.',
    'transfer.file': 'File zip',
    'transfer.nameOverride': 'Tên (tuỳ chọn)',
    'transfer.nameOverrideHint': 'Để trống = dùng tên lưu trong file zip.',
    'transfer.idMode': 'ID profile',
    'transfer.idNew': 'ID mới (mặc định)',
    'transfer.idKeep': 'Giữ ID gốc nếu chưa bị dùng',
    'transfer.importGo': 'Nhập',
    'transfer.importing': 'Đang nhập…',
    'transfer.imported': 'Đã nhập {name}',
    'transfer.keptId': 'Đã giữ ID gốc.',
    'transfer.proxyNote': 'Trên máy nguồn nó dùng proxy {label}. Proxy không được chuyển, hãy gán proxy ở đây.',
    'transfer.warn.platform': 'Khác nền tảng: {detail}. Dữ liệu trình duyệt có thể không chạy được ở đây.',
    'transfer.warn.build': 'Khác bản trình duyệt: {detail}.',
    'transfer.noFile': 'Hãy chọn file zip trước',
    'transfer.cloneTitle': 'Nhân bản “{name}”',
    'transfer.cloneBody': 'Sao chép cài đặt và dữ liệu trình duyệt sang profile mới. Profile phải đóng. Hai bản dùng chung các phiên đăng nhập: hãy dùng lần lượt, không cùng lúc.',
    'transfer.cloneName': 'Tên bản sao',
    'transfer.cloneNewMachine': 'Ghim máy mới (vân tay mới) thay vì sao chép',
    'transfer.cloneGo': 'Nhân bản',
    'transfer.cloning': 'Đang nhân bản…',
    'transfer.cloned': 'Đã nhân bản thành {name}',
    'transfer.copySuffix': '{name} (bản sao)',
    'transfer.bulkTitleDlg': 'Tạo profile hàng loạt',
    'transfer.pattern': 'Mẫu tên',
    'transfer.patternHint': '{n} được thay bằng số, ví dụ Shop {n} ra Shop 1, Shop 2…',
    'transfer.count': 'Số lượng',
    'transfer.start': 'Số bắt đầu',
    'transfer.os': 'HĐH của vân tay',
    'transfer.preview': 'Xem trước: {list}',
    'transfer.bulkGo': 'Tạo',
    'transfer.bulkBusy': 'Đang tạo… mỗi profile có thể mất vài giây',
    'transfer.bulkDone': 'Đã tạo {n} profile',
    'transfer.bulkPartial': 'Đã tạo {n}, rồi dừng: {error}',
  },
});

{
  const OSES = [{ k: 'win', t: 'Windows' }, { k: 'mac', t: 'macOS' }, { k: 'lin', t: 'Linux' }];

  CM.components.CmTransfer = {
    props: { exportProfile: Object, cloneProfile: Object },
    emits: ['close-export', 'close-clone', 'changed'],
    setup(props, { emit }) {
      const { reactive, ref, computed, watch } = Vue;
      const fmtSize = b => fmt.size(b);

      // ---- export ----
      const ex = reactive({ busy: false, result: null, error: '' });
      watch(() => props.exportProfile?.id, () => { ex.busy = false; ex.result = null; ex.error = ''; });
      async function doExport() {
        ex.busy = true; ex.error = ''; ex.result = null;
        try {
          ex.result = await api('/api/transfer/export/' + props.exportProfile.id, { method: 'POST' });
          toast(t('transfer.exported', { n: ex.result.files, size: fmtSize(ex.result.size) }));
        } catch (e) { ex.error = e.message; }
        ex.busy = false;
      }

      // ---- import ----
      const im = reactive({ open: false, busy: false, file: null, mode: 'new', name: '', error: '', result: null });
      const fileInput = ref(null);
      function openImport() { Object.assign(im, { open: true, busy: false, file: null, mode: 'new', name: '', error: '', result: null }); }
      async function doImport() {
        if (!im.file) { im.error = t('transfer.noFile'); return; }
        im.busy = true; im.error = '';
        try {
          const q = new URLSearchParams({ mode: im.mode === 'keep' ? 'keep_if_free' : 'new' });
          if (im.name.trim()) q.set('name', im.name.trim());
          // The file itself is the body: the browser streams it, nothing is read into memory.
          const r = await fetch('/api/transfer/import?' + q, { method: 'POST', headers: { 'content-type': 'application/zip' }, body: im.file });
          const j = await r.json().catch(() => ({}));
          if (!r.ok) throw new Error(j.error || r.statusText);
          im.result = j;
          toast(t('transfer.imported', { name: j.profile.name }));
          emit('changed');
        } catch (e) { im.error = e.message; }
        im.busy = false;
      }

      // ---- clone ----
      const cl = reactive({ name: '', newMachine: false, busy: false, error: '' });
      watch(() => props.cloneProfile?.id, () => {
        const p = props.cloneProfile;
        Object.assign(cl, { name: p ? t('transfer.copySuffix', { name: p.name }).slice(0, 80) : '', newMachine: false, busy: false, error: '' });
      });
      async function doClone() {
        cl.busy = true; cl.error = '';
        try {
          const v = await api('/api/transfer/clone/' + props.cloneProfile.id, { method: 'POST', body: JSON.stringify({ name: cl.name, newMachine: cl.newMachine }) });
          toast(t('transfer.cloned', { name: v.name }));
          emit('changed'); emit('close-clone');
        } catch (e) { cl.error = e.message; }
        cl.busy = false;
      }

      // ---- bulk ----
      const bk = reactive({ open: false, pattern: 'Shop {n}', count: 5, start: 1, os: 'win', busy: false, error: '' });
      const preview = computed(() => {
        if (!bk.pattern.includes('{n}')) return '';
        const n = Math.max(1, Math.min(+bk.count || 1, 100)), s = +bk.start || 1;
        const names = [];
        for (let i = 0; i < Math.min(n, 3); i++) names.push(bk.pattern.replaceAll('{n}', s + i));
        return names.join(', ') + (n > 3 ? ', … ' + bk.pattern.replaceAll('{n}', s + n - 1) : '');
      });
      function openBulk() { Object.assign(bk, { open: true, busy: false, error: '' }); }
      async function doBulk() {
        bk.busy = true; bk.error = '';
        try {
          const r = await api('/api/transfer/bulk', { method: 'POST', body: JSON.stringify({ pattern: bk.pattern, count: +bk.count, os: bk.os, start: +bk.start || 1 }) });
          if (r.created.length) emit('changed');
          if (r.error) bk.error = t('transfer.bulkPartial', { n: r.created.length, error: r.error });
          else { toast(t('transfer.bulkDone', { n: r.created.length })); bk.open = false; }
        } catch (e) { bk.error = e.message; }
        bk.busy = false;
      }

      return { OSES, ex, doExport, im, fileInput, openImport, doImport, cl, doClone, bk, preview, openBulk, doBulk, fmtSize };
    },
    template: `      <button class="btn" :title="t('transfer.importTitle')" @click="openImport">⤒ {{ t('transfer.import') }}</button>
      <button class="btn" :title="t('transfer.bulkTitle')" @click="openBulk">＋ {{ t('transfer.bulk') }}</button>

      <!-- export -->
      <cm-dialog :open="!!exportProfile" :title="exportProfile ? t('transfer.exportTitle', { name: exportProfile.name }) : ''" :busy="ex.busy" :width="480" @close="$emit('close-export')">
        <p class="m-0 text-[13px] text-muted">{{ t('transfer.exportBody') }}</p>
        <p class="m-0 text-[12.5px] text-warn">{{ t('transfer.exportSensitive') }}</p>
        <div v-if="exportProfile && exportProfile.running" class="banner !m-0"><cm-icon name="alert"/><div>{{ t('transfer.exportRunning') }}</div></div>
        <div v-if="ex.error" class="text-bad text-[12.5px] break-words">{{ ex.error }}</div>
        <div v-if="ex.result" class="text-[12.5px] flex flex-col gap-1">
          <div>{{ t('transfer.exported', { n: ex.result.files, size: fmtSize(ex.result.size) }) }}</div>
          <div class="text-faint">{{ t('transfer.exportPath') }}</div>
          <div class="mono break-all select-text">{{ ex.result.path }}</div>
          <a class="text-accent underline" :href="'/api/transfer/exports/' + encodeURIComponent(ex.result.fileName)" download>{{ t('transfer.download') }}</a>
          <div v-if="ex.result.skipped.length" class="text-warn">{{ t('transfer.skipped', { n: ex.result.skipped.length, list: ex.result.skipped.slice(0, 5).join(', ') }) }}</div>
        </div>
        <template #footer>
          <button class="btn" :disabled="ex.busy" @click="$emit('close-export')">{{ t('common.close') }}</button>
          <button class="btn btn-start" :disabled="ex.busy || !!(exportProfile && exportProfile.running)" @click="doExport">
            <template v-if="ex.busy"><span class="spin"></span> {{ t('transfer.exporting') }}</template><template v-else>{{ t('transfer.exportGo') }}</template>
          </button>
        </template>
      </cm-dialog>

      <!-- import -->
      <cm-dialog :open="im.open" :title="t('transfer.importTitleDlg')" :busy="im.busy" :width="480" @close="im.open = false">
        <p class="m-0 text-[13px] text-muted">{{ t('transfer.importBody') }}</p>
        <template v-if="!im.result">
          <cm-field :label="t('transfer.file')">
            <input ref="fileInput" type="file" accept=".zip,application/zip" class="input" :disabled="im.busy" @change="im.file = $event.target.files[0] || null">
          </cm-field>
          <cm-field :label="t('transfer.nameOverride')" :hint="t('transfer.nameOverrideHint')">
            <input class="input" v-model="im.name" maxlength="80" autocomplete="off" :disabled="im.busy">
          </cm-field>
          <cm-field :label="t('transfer.idMode')">
            <select class="input" v-model="im.mode" :disabled="im.busy"><option value="new">{{ t('transfer.idNew') }}</option><option value="keep">{{ t('transfer.idKeep') }}</option></select>
          </cm-field>
        </template>
        <div v-if="im.error" class="text-bad text-[12.5px] break-words">{{ im.error }}</div>
        <div v-if="im.result" class="text-[12.5px] flex flex-col gap-1">
          <div class="text-ok font-semibold">{{ t('transfer.imported', { name: im.result.profile.name }) }}<template v-if="im.result.keptId"> · {{ t('transfer.keptId') }}</template></div>
          <div v-if="im.result.proxy" class="text-muted">{{ t('transfer.proxyNote', { label: im.result.proxy.name + ' (' + im.result.proxy.scheme + '://' + im.result.proxy.host + ':' + im.result.proxy.port + ')' }) }}</div>
          <div v-for="w in im.result.warnings" class="text-warn">{{ t('transfer.warn.' + w.code, { detail: w.detail }) }}</div>
        </div>
        <template #footer>
          <button class="btn" :disabled="im.busy" @click="im.open = false">{{ t('common.close') }}</button>
          <button v-if="!im.result" class="btn btn-start" :disabled="im.busy || !im.file" @click="doImport">
            <template v-if="im.busy"><span class="spin"></span> {{ t('transfer.importing') }}</template><template v-else>{{ t('transfer.importGo') }}</template>
          </button>
        </template>
      </cm-dialog>

      <!-- clone -->
      <cm-dialog :open="!!cloneProfile" :title="cloneProfile ? t('transfer.cloneTitle', { name: cloneProfile.name }) : ''" :busy="cl.busy" :width="460" @close="$emit('close-clone')">
        <form id="clone-profile" class="flex flex-col gap-3" @submit.prevent="doClone">
          <p class="m-0 text-[13px] text-muted">{{ t('transfer.cloneBody') }}</p>
          <div v-if="cloneProfile && cloneProfile.running" class="banner !m-0"><cm-icon name="alert"/><div>{{ t('transfer.exportRunning') }}</div></div>
          <cm-field :label="t('transfer.cloneName')"><input class="input" v-model="cl.name" maxlength="80" autocomplete="off"></cm-field>
          <cm-switch v-model="cl.newMachine" :label="t('transfer.cloneNewMachine')"/>
          <div v-if="cl.error" class="text-bad text-[12.5px] break-words">{{ cl.error }}</div>
        </form>
        <template #footer>
          <button type="button" class="btn" :disabled="cl.busy" @click="$emit('close-clone')">{{ t('common.cancel') }}</button>
          <button type="submit" form="clone-profile" class="btn btn-start" :disabled="cl.busy || !cl.name.trim() || !!(cloneProfile && cloneProfile.running)">
            <template v-if="cl.busy"><span class="spin"></span> {{ t('transfer.cloning') }}</template><template v-else>{{ t('transfer.cloneGo') }}</template>
          </button>
        </template>
      </cm-dialog>

      <!-- bulk create -->
      <cm-dialog :open="bk.open" :title="t('transfer.bulkTitleDlg')" :busy="bk.busy" :width="460" @close="bk.open = false">
        <form id="bulk-create" class="flex flex-col gap-3" @submit.prevent="doBulk">
          <cm-field :label="t('transfer.pattern')" :hint="t('transfer.patternHint', { n: '{n}' })"><input class="input" v-model="bk.pattern" maxlength="80" autocomplete="off" spellcheck="false"></cm-field>
          <div class="grid grid-cols-3 gap-3">
            <cm-field :label="t('transfer.count')"><input class="input" type="number" min="1" max="100" v-model="bk.count"></cm-field>
            <cm-field :label="t('transfer.start')"><input class="input" type="number" min="0" v-model="bk.start"></cm-field>
            <cm-field :label="t('transfer.os')"><select class="input" v-model="bk.os"><option v-for="x in OSES" :key="x.k" :value="x.k">{{ x.t }}</option></select></cm-field>
          </div>
          <div v-if="preview" class="text-[12.5px] text-muted">{{ t('transfer.preview', { list: preview }) }}</div>
          <div v-if="bk.busy" class="text-[12.5px] text-muted">{{ t('transfer.bulkBusy') }}</div>
          <div v-if="bk.error" class="text-bad text-[12.5px] break-words">{{ bk.error }}</div>
        </form>
        <template #footer>
          <button type="button" class="btn" :disabled="bk.busy" @click="bk.open = false">{{ t('common.close') }}</button>
          <button type="submit" form="bulk-create" class="btn btn-start" :disabled="bk.busy || !preview">
            <template v-if="bk.busy"><span class="spin"></span></template><template v-else>{{ t('transfer.bulkGo') }}</template>
          </button>
        </template>
      </cm-dialog>
    `,
  };
}
