/* Excel/CSV export + import (phase 11).
   <cm-sheets @changed="reload"/> renders two toolbar buttons (Export, Import sheet) and their dialogs.
   Export downloads every profile as .xlsx or .csv (proxy = library name, never credentials).
   Import: choose an edited .xlsx/.csv, a dry run lists what would be created / updated / rejected
   (with sheet row numbers), Apply writes it using the same uploaded file (token). `changed` fires
   after an apply. Columns missing from the file are left alone; blank cells clear a field. */

I18N.add({
  en: {
    'sheets.export': 'Export',
    'sheets.exportTitle': 'Export profiles to a spreadsheet',
    'sheets.exportDlg': 'Export profiles',
    'sheets.exportBody': 'One row per profile: id, name, notes, OS, group, tags, proxy, timezone, locale, WebRTC, window, start URL and status. The proxy column is the library name only, never credentials.',
    'sheets.exportXlsx': 'Download .xlsx',
    'sheets.exportCsv': 'Download .csv',
    'sheets.import': 'Import sheet',
    'sheets.importTitle': 'Edit many profiles from a spreadsheet',
    'sheets.importDlg': 'Import a spreadsheet',
    'sheets.importBody': 'Use an exported .xlsx or .csv. Rows are matched by id; a blank id creates a new profile with a new machine. Nothing is written until you press Apply.',
    'sheets.file': 'Spreadsheet file',
    'sheets.createMissing': 'Create groups and tags that do not exist yet',
    'sheets.createMissingHint': 'Off: an unknown group or tag is an error on that row. Unknown proxies are always an error (add them in Proxies first).',
    'sheets.check': 'Check file',
    'sheets.checking': 'Checking…',
    'sheets.noFile': 'Choose a .xlsx or .csv file first',
    'sheets.summary': '{create} to create · {update} to update · {unchanged} unchanged · {errors} with errors',
    'sheets.col.row': 'Row',
    'sheets.col.action': 'Action',
    'sheets.col.profile': 'Profile',
    'sheets.col.changes': 'Changes',
    'sheets.action.create': 'Create',
    'sheets.action.update': 'Update',
    'sheets.action.unchanged': 'Unchanged',
    'sheets.action.error': 'Error',
    'sheets.showUnchanged': 'Show unchanged rows',
    'sheets.newGroups': 'Groups to create: {list}',
    'sheets.newTags': 'Tags to create: {list}',
    'sheets.ignored': 'Ignored columns: {list}',
    'sheets.skipNote': 'Rows with errors are skipped; the rest is applied.',
    'sheets.newMachine': 'New profile: a new machine is pinned',
    'sheets.warn.os-regenerate': 'The OS changes: a new machine (fingerprint) is pinned',
    'sheets.empty': '(empty)',
    'sheets.apply': 'Apply',
    'sheets.applying': 'Applying…',
    'sheets.applied': 'Created {created}, updated {updated}, skipped {skipped}',
    'sheets.failedRows': 'Failed while writing:',
    'sheets.back': 'Choose another file',
    'sheets.nothing': 'Nothing to apply',
    'sheets.f.name': 'name',
    'sheets.f.notes': 'notes',
    'sheets.f.os': 'OS',
    'sheets.f.group': 'group',
    'sheets.f.tags': 'tags',
    'sheets.f.proxy': 'proxy',
    'sheets.f.timezone': 'timezone',
    'sheets.f.locale': 'locale',
    'sheets.f.webrtc': 'WebRTC',
    'sheets.f.window': 'window',
    'sheets.f.start_url': 'start URL',
    'sheets.f.id': 'id',
  },
  vi: {
    'sheets.export': 'Xuất bảng',
    'sheets.exportTitle': 'Xuất profile ra bảng tính',
    'sheets.exportDlg': 'Xuất profile',
    'sheets.exportBody': 'Mỗi profile một dòng: id, tên, ghi chú, HĐH, nhóm, thẻ, proxy, múi giờ, ngôn ngữ, WebRTC, cửa sổ, URL khởi động và trạng thái. Cột proxy chỉ là tên trong thư viện, không bao giờ có thông tin đăng nhập.',
    'sheets.exportXlsx': 'Tải .xlsx',
    'sheets.exportCsv': 'Tải .csv',
    'sheets.import': 'Nhập bảng',
    'sheets.importTitle': 'Sửa nhiều profile từ bảng tính',
    'sheets.importDlg': 'Nhập bảng tính',
    'sheets.importBody': 'Dùng file .xlsx hoặc .csv đã xuất. Dòng được khớp theo id; id trống sẽ tạo profile mới với máy mới. Không có gì được ghi cho đến khi bạn bấm Áp dụng.',
    'sheets.file': 'File bảng tính',
    'sheets.createMissing': 'Tạo nhóm và thẻ chưa tồn tại',
    'sheets.createMissingHint': 'Tắt: nhóm hoặc thẻ lạ là lỗi ở dòng đó. Proxy lạ luôn là lỗi (hãy thêm ở mục Proxy trước).',
    'sheets.check': 'Kiểm tra file',
    'sheets.checking': 'Đang kiểm tra…',
    'sheets.noFile': 'Hãy chọn file .xlsx hoặc .csv trước',
    'sheets.summary': '{create} tạo mới · {update} cập nhật · {unchanged} không đổi · {errors} lỗi',
    'sheets.col.row': 'Dòng',
    'sheets.col.action': 'Thao tác',
    'sheets.col.profile': 'Profile',
    'sheets.col.changes': 'Thay đổi',
    'sheets.action.create': 'Tạo',
    'sheets.action.update': 'Cập nhật',
    'sheets.action.unchanged': 'Không đổi',
    'sheets.action.error': 'Lỗi',
    'sheets.showUnchanged': 'Hiện các dòng không đổi',
    'sheets.newGroups': 'Nhóm sẽ được tạo: {list}',
    'sheets.newTags': 'Thẻ sẽ được tạo: {list}',
    'sheets.ignored': 'Cột bị bỏ qua: {list}',
    'sheets.skipNote': 'Dòng có lỗi sẽ bị bỏ qua; phần còn lại vẫn được áp dụng.',
    'sheets.newMachine': 'Profile mới: ghim một máy mới',
    'sheets.warn.os-regenerate': 'HĐH đổi: sẽ ghim máy (vân tay) mới',
    'sheets.empty': '(trống)',
    'sheets.apply': 'Áp dụng',
    'sheets.applying': 'Đang áp dụng…',
    'sheets.applied': 'Đã tạo {created}, cập nhật {updated}, bỏ qua {skipped}',
    'sheets.failedRows': 'Lỗi khi ghi:',
    'sheets.back': 'Chọn file khác',
    'sheets.nothing': 'Không có gì để áp dụng',
    'sheets.f.name': 'tên',
    'sheets.f.notes': 'ghi chú',
    'sheets.f.os': 'HĐH',
    'sheets.f.group': 'nhóm',
    'sheets.f.tags': 'thẻ',
    'sheets.f.proxy': 'proxy',
    'sheets.f.timezone': 'múi giờ',
    'sheets.f.locale': 'ngôn ngữ',
    'sheets.f.webrtc': 'WebRTC',
    'sheets.f.window': 'cửa sổ',
    'sheets.f.start_url': 'URL khởi động',
    'sheets.f.id': 'id',
  },
});

{
  CM.components.CmSheets = {
    emits: ['changed'],
    setup(props, { emit }) {
      const { reactive, computed } = Vue;

      const ex = reactive({ open: false });

      const im = reactive({ open: false, busy: false, file: null, createMissing: false, error: '', preview: null, result: null, showUnchanged: false });
      function openImport() {
        Object.assign(im, { open: true, busy: false, file: null, createMissing: false, error: '', preview: null, result: null, showUnchanged: false });
      }
      const rows = computed(() => (im.preview ? im.preview.rows.filter(r => im.showUnchanged || r.action !== 'unchanged') : []));
      const actionable = computed(() => (im.preview ? im.preview.summary.create + im.preview.summary.update : 0));
      const fieldName = f => t('sheets.f.' + f);
      const val = v => (v == null || v === '' ? t('sheets.empty') : v);
      const cls = a => ({ create: 'text-ok', update: 'text-accent', unchanged: 'text-faint', error: 'text-bad' }[a] || '');

      async function check() {
        if (!im.file) { im.error = t('sheets.noFile'); return; }
        im.busy = true; im.error = ''; im.preview = null;
        try {
          const q = new URLSearchParams({ createMissing: im.createMissing ? 'true' : 'false' });
          // The file itself is the body (format detected on the server).
          const r = await fetch('/api/sheets/import/preview?' + q, { method: 'POST', body: im.file });
          const j = await r.json().catch(() => ({}));
          if (!r.ok) throw new Error(j.error || r.statusText);
          im.preview = j;
        } catch (e) { im.error = e.message; }
        im.busy = false;
      }
      // Changing the option changes what is valid: re-check with the same file.
      Vue.watch(() => im.createMissing, () => { if (im.preview && im.file) check(); });

      async function apply() {
        im.busy = true; im.error = '';
        try {
          im.result = await api('/api/sheets/import/apply', { method: 'POST', body: JSON.stringify({ token: im.preview.token, createMissing: im.createMissing }) });
          toast(t('sheets.applied', im.result));
          emit('changed');
        } catch (e) { im.error = e.message; }
        im.busy = false;
      }

      return { ex, im, openImport, rows, actionable, fieldName, val, cls, check, apply };
    },
    template: `      <button class="btn" :title="t('sheets.exportTitle')" @click="ex.open = true">⤓ {{ t('sheets.export') }}</button>
      <button class="btn" :title="t('sheets.importTitle')" @click="openImport">⤒ {{ t('sheets.import') }}</button>

      <!-- export -->
      <cm-dialog :open="ex.open" :title="t('sheets.exportDlg')" :width="440" @close="ex.open = false">
        <p class="m-0 text-[13px] text-muted">{{ t('sheets.exportBody') }}</p>
        <div class="flex gap-2">
          <a class="btn btn-start" href="/api/sheets/export?format=xlsx" download>{{ t('sheets.exportXlsx') }}</a>
          <a class="btn" href="/api/sheets/export?format=csv" download>{{ t('sheets.exportCsv') }}</a>
        </div>
        <template #footer><button class="btn" @click="ex.open = false">{{ t('common.close') }}</button></template>
      </cm-dialog>

      <!-- import -->
      <cm-dialog :open="im.open" :title="t('sheets.importDlg')" :busy="im.busy" :width="860" @close="im.open = false">
        <p v-if="!im.preview && !im.result" class="m-0 text-[13px] text-muted">{{ t('sheets.importBody') }}</p>
        <template v-if="!im.result">
          <cm-field :label="t('sheets.file')">
            <input type="file" accept=".xlsx,.csv,text/csv,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" class="input" :disabled="im.busy" @change="im.file = $event.target.files[0] || null; im.preview = null; im.error = ''">
          </cm-field>
          <div>
            <cm-switch v-model="im.createMissing" :label="t('sheets.createMissing')"/>
            <div class="text-[12px] text-faint mt-1">{{ t('sheets.createMissingHint') }}</div>
          </div>
        </template>
        <div v-if="im.error" class="text-bad text-[12.5px] break-words">{{ im.error }}</div>

        <template v-if="im.preview && !im.result">
          <div class="text-[13px] font-semibold">{{ t('sheets.summary', im.preview.summary) }}</div>
          <div v-if="im.preview.newGroups.length" class="text-[12.5px] text-muted">{{ t('sheets.newGroups', { list: im.preview.newGroups.join(', ') }) }}</div>
          <div v-if="im.preview.newTags.length" class="text-[12.5px] text-muted">{{ t('sheets.newTags', { list: im.preview.newTags.join(', ') }) }}</div>
          <div v-if="im.preview.ignoredColumns.length" class="text-[12.5px] text-faint">{{ t('sheets.ignored', { list: im.preview.ignoredColumns.join(', ') }) }}</div>
          <div v-if="im.preview.summary.errors" class="text-[12.5px] text-warn">{{ t('sheets.skipNote') }}</div>
          <label class="flex items-center gap-2 text-[12.5px] text-muted"><input type="checkbox" v-model="im.showUnchanged"> {{ t('sheets.showUnchanged') }}</label>
          <div class="border border-line rounded-lg overflow-auto" style="max-height: 46vh">
            <table class="w-full text-[12.5px] border-collapse">
              <thead class="sticky top-0 bg-panel">
                <tr class="text-left text-faint">
                  <th class="px-2 py-1.5 font-medium w-14">{{ t('sheets.col.row') }}</th>
                  <th class="px-2 py-1.5 font-medium w-24">{{ t('sheets.col.action') }}</th>
                  <th class="px-2 py-1.5 font-medium w-48">{{ t('sheets.col.profile') }}</th>
                  <th class="px-2 py-1.5 font-medium">{{ t('sheets.col.changes') }}</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="r in rows" :key="r.row" class="border-t border-line align-top">
                  <td class="px-2 py-1.5 mono">{{ r.row }}</td>
                  <td class="px-2 py-1.5 font-semibold" :class="cls(r.action)">{{ t('sheets.action.' + r.action) }}</td>
                  <td class="px-2 py-1.5 break-words">{{ r.name || '—' }}<div v-if="r.profileId" class="mono text-faint text-[11px]">{{ r.profileId }}</div></td>
                  <td class="px-2 py-1.5">
                    <div v-if="r.action === 'create'" class="text-muted">{{ t('sheets.newMachine') }}</div>
                    <div v-for="c in r.changes" :key="c.field"><span class="text-faint">{{ fieldName(c.field) }}:</span> <span v-if="r.action !== 'create'" class="text-muted">{{ val(c.before) }} → </span><b>{{ val(c.after) }}</b></div>
                    <div v-for="(msg, f) in r.errors" :key="'e' + f" class="text-bad"><span>{{ fieldName(f) }}:</span> {{ msg }}</div>
                    <div v-for="w in r.warnings" :key="w" class="text-warn">{{ t('sheets.warn.' + w) }}</div>
                  </td>
                </tr>
                <tr v-if="!rows.length"><td colspan="4" class="px-2 py-3 text-faint">{{ t('sheets.nothing') }}</td></tr>
              </tbody>
            </table>
          </div>
        </template>

        <div v-if="im.result" class="text-[12.5px] flex flex-col gap-1">
          <div class="text-ok font-semibold">{{ t('sheets.applied', im.result) }}</div>
          <template v-if="im.result.failed.length">
            <div class="text-bad">{{ t('sheets.failedRows') }}</div>
            <div v-for="f in im.result.failed" :key="f.row" class="text-bad break-words">{{ t('sheets.col.row') }} {{ f.row }}: {{ f.error }}</div>
          </template>
        </div>

        <template #footer>
          <button class="btn" :disabled="im.busy" @click="im.open = false">{{ t('common.close') }}</button>
          <template v-if="!im.result">
            <button class="btn" :disabled="im.busy || !im.file" @click="check">
              <template v-if="im.busy && !im.preview"><span class="spin"></span> {{ t('sheets.checking') }}</template><template v-else>{{ t('sheets.check') }}</template>
            </button>
            <button class="btn btn-start" :disabled="im.busy || !im.preview || !actionable" @click="apply">
              <template v-if="im.busy && im.preview"><span class="spin"></span> {{ t('sheets.applying') }}</template><template v-else>{{ t('sheets.apply') }}</template>
            </button>
          </template>
        </template>
      </cm-dialog>
    `,
  };
}
