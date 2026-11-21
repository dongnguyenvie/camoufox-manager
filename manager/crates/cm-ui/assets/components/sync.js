/* Device sync dialog (phase 17), opened from Settings → Devices: <cm-sync :device="d" @close="..."/>.
   Step 1: choose a direction (copy their profiles here / send mine there). The server diffs both
   machines (GET /api/sync/devices/{id}/plan) and every profile shows its status: new on the
   target, changed on one side, changed on both (a conflict: you choose, nothing is merged), or
   identical. Running profiles and machines with another OS / CPU / Camoufox build are listed with
   the reason and cannot be selected. Options: proxies (off by default; the password only if
   ticked). A warning says the same logins will exist on both machines and must be ticked before
   Start. Step 2: progress per profile (bytes, state), Cancel, then a result summary. The previous
   profile on the receiving side is never touched by a failed or cancelled copy. */

I18N.add({
  en: {
    'sync.button': 'Sync',
    'sync.title': 'Sync with {name}',
    'sync.direction': 'Direction',
    'sync.pull': 'Copy from {name} to this computer',
    'sync.push': 'Send from this computer to {name}',
    'sync.loading': 'Comparing the two computers…',
    'sync.none': 'There are no profiles to copy in this direction.',
    'sync.incompatible': 'These two computers cannot exchange profiles: {reason}.',
    'sync.colProfile': 'Profile',
    'sync.colSize': 'Size',
    'sync.colStatus': 'Status',
    'sync.colAction': 'Action',
    'sync.status.pull.missingOnTarget': 'New here',
    'sync.status.pull.sourceChanged': 'Changed on {name}',
    'sync.status.pull.targetChanged': 'Changed only here',
    'sync.status.pull.conflict': 'Changed on both',
    'sync.status.pull.unlinked': 'Same profile, never synced',
    'sync.status.pull.identical': 'Identical',
    'sync.status.push.missingOnTarget': 'Not on {name}',
    'sync.status.push.sourceChanged': 'Changed here',
    'sync.status.push.targetChanged': 'Changed only on {name}',
    'sync.status.push.conflict': 'Changed on both',
    'sync.status.push.unlinked': 'Same profile, never synced',
    'sync.status.push.identical': 'Identical',
    'sync.conflictHint': 'Both sides changed. Nothing is merged: choose which version to keep.',
    'sync.act.pull.create': 'Copy here',
    'sync.act.pull.replace': 'Take theirs (replace mine)',
    'sync.act.pull.copy': 'Save theirs as a copy',
    'sync.act.pull.skip': 'Keep mine',
    'sync.act.push.create': 'Copy there',
    'sync.act.push.replace': 'Replace theirs with mine',
    'sync.act.push.copy': 'Save mine as a copy there',
    'sync.act.push.skip': 'Keep theirs',
    'sync.blocked.source_running': 'Running: close it first',
    'sync.blocked.target_running': 'Running on the other computer: close it there first',
    'sync.blocked.incompatible': 'Not compatible',
    'sync.options': 'Options',
    'sync.proxy': 'Also copy each profile’s proxy (host, port, type, user name)',
    'sync.proxyPassword': 'Also copy the proxy password',
    'sync.proxyPasswordHint': 'Off by default. The password travels over the encrypted Tailscale connection.',
    'sync.warnTitle': 'Same logins on both computers',
    'sync.warn': 'Copying a profile copies its cookies and logged-in sessions. The same accounts will be logged in on both computers, and websites may notice and sign you out or flag the accounts. Use each profile on one computer at a time.',
    'sync.ack': 'I understand',
    'sync.backup': 'If a copy fails or is cancelled, the profile on the receiving computer stays as it was. When a profile is replaced, a backup of the old one is kept next to it.',
    'sync.start': 'Start',
    'sync.selected': '{n} selected',
    'sync.running': 'Syncing…',
    'sync.cancel': 'Cancel',
    'sync.cancelling': 'Stopping after the current piece…',
    'sync.back': 'Back',
    'sync.state.queued': 'Waiting',
    'sync.state.preparing': 'Preparing',
    'sync.state.transferring': 'Copying',
    'sync.state.verifying': 'Checking',
    'sync.state.applying': 'Applying',
    'sync.state.done': 'Done',
    'sync.state.failed': 'Failed',
    'sync.state.skipped': 'Skipped',
    'sync.state.cancelled': 'Cancelled',
    'sync.summary': '{done} copied · {failed} failed · {skipped} skipped',
    'sync.finished': 'Sync finished',
    'sync.cancelledMsg': 'Sync cancelled',
    'sync.retryHint': 'A copy that was interrupted continues where it stopped when you start it again.',
  },
  vi: {
    'sync.button': 'Đồng bộ',
    'sync.title': 'Đồng bộ với {name}',
    'sync.direction': 'Hướng',
    'sync.pull': 'Chép từ {name} về máy này',
    'sync.push': 'Gửi từ máy này sang {name}',
    'sync.loading': 'Đang so sánh hai máy…',
    'sync.none': 'Không có profile nào để chép theo hướng này.',
    'sync.incompatible': 'Hai máy này không trao đổi profile được: {reason}.',
    'sync.colProfile': 'Profile',
    'sync.colSize': 'Dung lượng',
    'sync.colStatus': 'Trạng thái',
    'sync.colAction': 'Thao tác',
    'sync.status.pull.missingOnTarget': 'Mới ở máy này',
    'sync.status.pull.sourceChanged': 'Đã đổi trên {name}',
    'sync.status.pull.targetChanged': 'Chỉ đổi ở máy này',
    'sync.status.pull.conflict': 'Cả hai bên đều đổi',
    'sync.status.pull.unlinked': 'Cùng profile, chưa từng đồng bộ',
    'sync.status.pull.identical': 'Giống nhau',
    'sync.status.push.missingOnTarget': 'Chưa có trên {name}',
    'sync.status.push.sourceChanged': 'Đã đổi ở máy này',
    'sync.status.push.targetChanged': 'Chỉ đổi trên {name}',
    'sync.status.push.conflict': 'Cả hai bên đều đổi',
    'sync.status.push.unlinked': 'Cùng profile, chưa từng đồng bộ',
    'sync.status.push.identical': 'Giống nhau',
    'sync.conflictHint': 'Cả hai bên đều đã đổi. Không tự gộp: hãy chọn giữ bản nào.',
    'sync.act.pull.create': 'Chép về đây',
    'sync.act.pull.replace': 'Lấy bản của họ (thay bản của tôi)',
    'sync.act.pull.copy': 'Lưu bản của họ thành bản sao',
    'sync.act.pull.skip': 'Giữ bản của tôi',
    'sync.act.push.create': 'Chép sang đó',
    'sync.act.push.replace': 'Thay bản của họ bằng bản của tôi',
    'sync.act.push.copy': 'Lưu bản của tôi thành bản sao bên đó',
    'sync.act.push.skip': 'Giữ bản của họ',
    'sync.blocked.source_running': 'Đang chạy: hãy đóng trước',
    'sync.blocked.target_running': 'Đang chạy trên máy kia: hãy đóng ở đó trước',
    'sync.blocked.incompatible': 'Không tương thích',
    'sync.options': 'Tuỳ chọn',
    'sync.proxy': 'Chép cả proxy của từng profile (host, cổng, loại, tên đăng nhập)',
    'sync.proxyPassword': 'Chép cả mật khẩu proxy',
    'sync.proxyPasswordHint': 'Mặc định tắt. Mật khẩu đi qua kết nối Tailscale đã mã hoá.',
    'sync.warnTitle': 'Cùng đăng nhập trên hai máy',
    'sync.warn': 'Chép profile là chép cả cookie và phiên đăng nhập. Các tài khoản đó sẽ cùng đăng nhập trên cả hai máy, website có thể phát hiện rồi đăng xuất hoặc gắn cờ tài khoản. Mỗi profile chỉ nên dùng trên một máy tại một thời điểm.',
    'sync.ack': 'Tôi hiểu',
    'sync.backup': 'Nếu việc chép lỗi hoặc bị huỷ, profile trên máy nhận giữ nguyên như cũ. Khi thay một profile, bản sao lưu của profile cũ được giữ lại bên cạnh.',
    'sync.start': 'Bắt đầu',
    'sync.selected': 'Đã chọn {n}',
    'sync.running': 'Đang đồng bộ…',
    'sync.cancel': 'Huỷ',
    'sync.cancelling': 'Sẽ dừng sau phần đang chép…',
    'sync.back': 'Quay lại',
    'sync.state.queued': 'Đang chờ',
    'sync.state.preparing': 'Chuẩn bị',
    'sync.state.transferring': 'Đang chép',
    'sync.state.verifying': 'Kiểm tra',
    'sync.state.applying': 'Áp dụng',
    'sync.state.done': 'Xong',
    'sync.state.failed': 'Lỗi',
    'sync.state.skipped': 'Bỏ qua',
    'sync.state.cancelled': 'Đã huỷ',
    'sync.summary': '{done} đã chép · {failed} lỗi · {skipped} bỏ qua',
    'sync.finished': 'Đã đồng bộ xong',
    'sync.cancelledMsg': 'Đã huỷ đồng bộ',
    'sync.retryHint': 'Bản chép bị gián đoạn sẽ tiếp tục từ chỗ đã dừng khi bạn bắt đầu lại.',
  },
});

{
  CM.components.CmSync = {
    props: { device: Object },
    emits: ['close', 'changed'],
    setup(props, { emit }) {
      const { reactive, ref, computed, onUnmounted, watch } = Vue;
      const dir = ref('pull');
      const plan = ref(null);
      const loading = ref(false);
      const error = ref('');
      const sel = reactive({});
      const act = reactive({});
      const opt = reactive({ proxy: false, password: false, ack: false });
      const job = ref(null);
      const starting = ref(false);
      let timer = null;

      const name = computed(() => props.device ? props.device.name : '');
      const running = computed(() => !!job.value && job.value.state === 'running');

      const load = async () => {
        loading.value = true; error.value = ''; plan.value = null;
        try {
          const p = await api('/api/sync/devices/' + props.device.id + '/plan?direction=' + dir.value);
          for (const k of Object.keys(sel)) delete sel[k];
          for (const k of Object.keys(act)) delete act[k];
          for (const r of p.rows) {
            const ready = !r.blocked && r.status !== 'identical';
            act[r.sourceId] = r.defaultAction;
            sel[r.sourceId] = ready && !r.needsChoice;
          }
          plan.value = p;
        } catch (e) { error.value = e.message; }
        loading.value = false;
      };

      watch(() => props.device && props.device.id, id => { if (id) { job.value = null; load(); } }, { immediate: true });
      watch(dir, () => { if (!running.value) load(); });

      const setAct = (r, v) => { act[r.sourceId] = v; if (v !== 'skip') sel[r.sourceId] = true; };
      const actionable = r => !r.blocked && r.status !== 'identical';
      const picked = computed(() => plan.value ? plan.value.rows.filter(r => actionable(r) && sel[r.sourceId] && act[r.sourceId] !== 'skip') : []);
      const canStart = computed(() => picked.value.length > 0 && opt.ack && !starting.value && !plan.value.incompatible);
      const label = (a) => t('sync.act.' + dir.value + '.' + a);
      const statusText = r => t('sync.status.' + dir.value + '.' + r.status, { name: name.value });
      const statusClass = r => r.status === 'conflict' || r.status === 'unlinked' ? 'text-warn' : r.status === 'identical' ? 'text-faint' : 'text-accent';

      const poll = async () => {
        if (!job.value) return;
        try {
          const j = await api('/api/sync/jobs/' + job.value.id);
          const was = job.value.state;
          job.value = j;
          if (j.state !== 'running') {
            clearInterval(timer); timer = null;
            if (was === 'running') {
              toast(j.state === 'cancelled' ? t('sync.cancelledMsg') : t('sync.finished'));
              if (j.done) emit('changed');
            }
          }
        } catch (e) { /* keep polling: the server may be busy */ }
      };

      const start = async () => {
        starting.value = true; error.value = '';
        try {
          const items = picked.value.map(r => ({ profileId: r.sourceId, action: act[r.sourceId] }));
          job.value = await api('/api/sync/devices/' + props.device.id + '/start', {
            method: 'POST',
            body: JSON.stringify({ direction: dir.value, items, includeProxy: opt.proxy, includeProxyPassword: opt.proxy && opt.password, acknowledged: opt.ack }),
          });
          timer = setInterval(poll, 700);
        } catch (e) { error.value = e.message; }
        starting.value = false;
      };

      const cancel = async () => {
        try { await api('/api/sync/jobs/' + job.value.id + '/cancel', { method: 'POST', body: '{}' }); await poll(); }
        catch (e) { toast(e.message, true); }
      };

      const back = () => { job.value = null; load(); };
      const close = () => { if (!running.value) emit('close'); };
      onUnmounted(() => { if (timer) clearInterval(timer); });

      const pct = i => i.total ? Math.min(100, Math.round(i.bytes * 100 / i.total)) : (i.state === 'done' ? 100 : 0);
      const size = b => fmt.size(b);
      const stateClass = i => i.state === 'done' ? 'text-ok' : i.state === 'failed' ? 'text-bad' : i.state === 'cancelled' || i.state === 'skipped' ? 'text-faint' : 'text-accent';
      const blockedText = r => t('sync.blocked.' + r.blocked.code) + (r.blocked.code === 'incompatible' ? ': ' + r.blocked.detail : '');

      return { dir, plan, loading, error, sel, act, opt, job, starting, name, running, setAct, actionable, picked, canStart, label, statusText, statusClass,
               start, cancel, back, close, pct, size, stateClass, blockedText };
    },
    template: `<cm-dialog :open="!!device" :title="t('sync.title', { name })" :busy="running" :width="760" @close="close">
      <!-- progress / result -->
      <template v-if="job">
        <div class="flex flex-col gap-2" style="max-height:52vh;overflow:auto">
          <div v-for="i in job.items" :key="i.profileId" class="border border-line rounded-lg p-2.5 flex flex-col gap-1">
            <div class="flex items-center gap-2">
              <span class="font-semibold text-[13px] truncate min-w-0" style="flex:1">{{ i.name || i.profileId }}</span>
              <span class="text-xs" :class="stateClass(i)">{{ t('sync.state.' + i.state) }}</span>
            </div>
            <div v-if="i.state === 'transferring' || i.state === 'verifying' || i.state === 'applying' || (i.total && i.state === 'done')" class="flex items-center gap-2">
              <div class="bg-line rounded-full overflow-hidden" style="flex:1;height:6px"><div class="bg-accent" :style="{ width: pct(i) + '%', height: '6px' }"></div></div>
              <span class="text-xs text-faint tabular-nums">{{ size(i.bytes) }} / {{ size(i.total) }}</span>
            </div>
            <div v-if="i.message" class="text-xs break-words" :class="i.state === 'failed' ? 'text-bad' : 'text-muted'">{{ i.message }}</div>
          </div>
        </div>
        <div v-if="running && job.cancelling" class="text-xs text-muted">{{ t('sync.cancelling') }}</div>
        <div v-if="!running" class="text-[13px] font-semibold">{{ t('sync.summary', { done: job.done, failed: job.failed, skipped: job.skipped }) }}</div>
        <div v-if="!running && job.failed" class="text-xs text-muted">{{ t('sync.retryHint') }}</div>
      </template>

      <!-- choose -->
      <template v-else>
        <div class="flex flex-col gap-1.5">
          <span class="text-xs text-muted">{{ t('sync.direction') }}</span>
          <label class="flex items-center gap-2 text-[13px] cursor-pointer"><input type="radio" value="pull" v-model="dir"> {{ t('sync.pull', { name }) }}</label>
          <label class="flex items-center gap-2 text-[13px] cursor-pointer"><input type="radio" value="push" v-model="dir"> {{ t('sync.push', { name }) }}</label>
        </div>
        <div v-if="loading" class="text-[13px] text-muted"><span class="spin"></span> {{ t('sync.loading') }}</div>
        <div v-if="error" class="text-bad text-[12.5px] break-words">{{ error }}</div>
        <template v-if="plan">
          <div v-if="plan.incompatible" class="banner !m-0"><cm-icon name="alert"/><div>{{ t('sync.incompatible', { reason: plan.incompatible }) }}</div></div>
          <p v-if="!plan.rows.length" class="m-0 text-[13px] text-muted">{{ t('sync.none') }}</p>
          <div v-else style="max-height:34vh;overflow:auto">
            <table class="list-tbl">
              <thead><tr><th></th><th>{{ t('sync.colProfile') }}</th><th>{{ t('sync.colSize') }}</th><th>{{ t('sync.colStatus') }}</th><th>{{ t('sync.colAction') }}</th></tr></thead>
              <tbody>
                <tr v-for="r in plan.rows" :key="r.sourceId" :class="actionable(r) ? '' : 'opacity-50'">
                  <td><input type="checkbox" v-model="sel[r.sourceId]" :disabled="!actionable(r) || act[r.sourceId] === 'skip'"></td>
                  <td>{{ r.name }}<template v-if="r.targetName && r.targetName !== r.name"> <span class="text-xs text-faint">→ {{ r.targetName }}</span></template></td>
                  <td class="tabular-nums">{{ size(r.size) }}</td>
                  <td>
                    <span class="text-xs" :class="statusClass(r)">{{ statusText(r) }}</span>
                    <div v-if="r.blocked" class="text-xs text-warn">{{ blockedText(r) }}</div>
                    <div v-else-if="r.needsChoice && r.status === 'conflict'" class="text-xs text-faint">{{ t('sync.conflictHint') }}</div>
                  </td>
                  <td>
                    <select v-if="actionable(r) && r.actions.length > 1" class="input" :value="act[r.sourceId]" @change="setAct(r, $event.target.value)">
                      <option v-for="a in r.actions" :key="a" :value="a">{{ label(a) }}</option>
                    </select>
                    <span v-else-if="actionable(r)" class="text-xs">{{ label(r.actions[0]) }}</span>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          <div class="flex flex-col gap-2">
            <span class="text-xs text-muted">{{ t('sync.options') }}</span>
            <cm-switch v-model="opt.proxy" :label="t('sync.proxy')"/>
            <div v-if="opt.proxy" class="flex flex-col gap-0.5">
              <cm-switch v-model="opt.password" :label="t('sync.proxyPassword')"/>
              <span class="text-xs text-faint">{{ t('sync.proxyPasswordHint') }}</span>
            </div>
          </div>
          <div class="banner !m-0"><cm-icon name="alert"/><div><b>{{ t('sync.warnTitle') }}.</b> {{ t('sync.warn') }}
            <label class="flex items-center gap-2 mt-1.5 text-[13px] cursor-pointer"><input type="checkbox" v-model="opt.ack"> {{ t('sync.ack') }}</label></div></div>
          <p class="m-0 text-xs text-faint">{{ t('sync.backup') }}</p>
        </template>
      </template>
      <template #footer>
        <template v-if="job">
          <button v-if="running" class="btn btn-stop" :disabled="job.cancelling" @click="cancel">{{ t('sync.cancel') }}</button>
          <button v-else class="btn" @click="back">{{ t('sync.back') }}</button>
          <button class="btn" :disabled="running" @click="close">{{ t('common.close') }}</button>
        </template>
        <template v-else>
          <span v-if="plan" class="text-xs text-faint" style="margin-right:auto">{{ t('sync.selected', { n: picked.length }) }}</span>
          <button class="btn" @click="close">{{ t('common.close') }}</button>
          <button class="btn btn-start" :disabled="!plan || !canStart" @click="start">
            <template v-if="starting"><span class="spin"></span></template><template v-else>{{ t('sync.start') }}</template>
          </button>
        </template>
      </template>
    </cm-dialog>`,
  };
}
