/* <cm-table :columns="[{ key, label, cls (cell class), hcls (header class), w }]" :rows="rows" row-key="id" :empty="t('...')">
     <template #cell-name="{ row }">…</template>     (optional per-column slot, default = row[key])
   </cm-table>
   Simple list table (header row, hover, no grid lines).
   The profile list keeps its own spreadsheet table (pages/profiles.js). */

CM.components.CmTable = {
  props: { columns: Array, rows: Array, rowKey: { type: String, default: 'id' }, empty: String, loading: Boolean },
  template: `<div class="min-h-0 overflow-auto">
    <table class="list-tbl">
      <thead><tr>
        <th v-for="c in columns" :key="c.key" :class="c.hcls" :style="c.w ? { width: c.w + 'px' } : null">{{ c.label }}</th>
      </tr></thead>
      <tbody>
        <tr v-if="loading"><td :colspan="columns.length" class="!py-10 text-center"><div class="skel w-[30%] mx-auto"></div></td></tr>
        <tr v-else-if="!rows || !rows.length"><td :colspan="columns.length" class="!py-10 text-center text-muted">{{ empty || t('common.empty') }}</td></tr>
        <tr v-else v-for="r in rows" :key="r[rowKey]">
          <td v-for="c in columns" :key="c.key" :class="c.cls">
            <slot :name="'cell-' + c.key" :row="r">{{ r[c.key] ?? '—' }}</slot>
          </td>
        </tr>
      </tbody>
    </table>
  </div>`,
};
