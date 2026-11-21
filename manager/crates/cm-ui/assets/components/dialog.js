/* <cm-dialog :open="x" :title="..." :busy="b" @close="x = false"> body <template #footer>buttons</template></cm-dialog>
   Modal centred over a dimmed page. Esc / click outside closes unless `busy`. `width` in px. */

CM.components.CmDialog = {
  props: { open: Boolean, title: String, busy: Boolean, width: { type: Number, default: 380 } },
  emits: ['close'],
  setup(props, { emit }) {
    const close = () => { if (!props.busy) emit('close'); };
    const onKey = e => { if (e.key === 'Escape' && props.open) close(); };
    Vue.onMounted(() => document.addEventListener('keydown', onKey));
    Vue.onUnmounted(() => document.removeEventListener('keydown', onKey));
    return { close };
  },
  /* Teleported to <body>: dialogs opened from a page header (backdrop-blur creates a containing block for fixed) would be clipped. */
  template: `<teleport to="body"><div v-if="open" class="fixed inset-0 z-40 grid place-items-center bg-black/35 px-4" @mousedown.self="close">
    <div role="dialog" aria-modal="true" class="w-full bg-panel border border-line rounded-xl shadow-card p-5 flex flex-col gap-3" :style="{ maxWidth: width + 'px' }">
      <h2 v-if="title" class="m-0 text-[15px] font-semibold">{{ title }}</h2>
      <slot></slot>
      <div v-if="$slots.footer" class="flex justify-end gap-2 mt-1"><slot name="footer"></slot></div>
    </div>
  </div></teleport>`,
};
