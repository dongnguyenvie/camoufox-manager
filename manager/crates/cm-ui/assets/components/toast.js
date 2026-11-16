/* Toasts: call toast(message) or toast(message, true) for an error from anywhere;
   <cm-toasts/> (mounted once by app.js) renders them bottom-right. */

const toasts = Vue.reactive([]);
let toastSeq = 0;

function toast(msg, err) {
  const item = { id: ++toastSeq, msg, err: !!err };
  toasts.push(item);
  setTimeout(() => { const i = toasts.indexOf(item); if (i >= 0) toasts.splice(i, 1); }, err ? 6500 : 3000);
}

CM.components.CmToasts = {
  setup: () => ({ toasts }),
  template: `<div class="fixed right-[18px] bottom-[18px] z-50 flex flex-col gap-2" role="status" aria-live="polite">
    <div v-for="x in toasts" :key="x.id" class="px-3.5 py-2.5 rounded-[10px] shadow-card max-w-[380px] animate-toast whitespace-pre-line"
         :class="x.err ? 'bg-bad text-white' : 'bg-ink text-bg'">{{ x.msg }}</div>
  </div>`,
};
