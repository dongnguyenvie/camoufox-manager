/* Left sidebar (D8): brand, section nav, language switch, server address.
   Props: nav = [{ key, icon, label }], current = active key. Emits `lang` with the chosen code. */

CM.components.CmSidebar = {
  props: { nav: Array, current: String },
  emits: ['lang'],
  setup() {
    return { LANGS: I18N.LANGS, locale: I18N.locale, host: location.host };
  },
  template: `<aside class="w-[212px] shrink-0 flex flex-col border-r border-line bg-panel max-md:w-[64px]">
    <div class="flex items-center gap-2.5 px-4 h-[52px] border-b border-line">
      <div class="size-7 rounded-[9px] bg-linear-to-br from-accent to-[#22c1c3] grid place-items-center text-white font-bold text-[13px] shrink-0">C</div>
      <div class="min-w-0 max-md:hidden">
        <div class="text-[13.5px] font-semibold tracking-tight truncate">{{ t('app.name') }}</div>
      </div>
    </div>
    <nav class="flex flex-col gap-0.5 p-2" :aria-label="t('app.name')">
      <a v-for="n in nav" :key="n.key" :href="'#/' + n.key"
         class="flex items-center gap-2.5 px-2.5 py-[7px] rounded-lg text-[13.5px] no-underline max-md:justify-center"
         :class="current === n.key ? 'bg-accent2 text-accent font-semibold' : 'text-muted hover:bg-panel2 hover:text-ink'"
         :aria-current="current === n.key ? 'page' : null" :title="t(n.label)">
        <cm-icon :name="n.icon"/><span class="max-md:hidden">{{ t(n.label) }}</span>
      </a>
    </nav>
    <div class="flex-1"></div>
    <div class="p-3 flex flex-col gap-2 max-md:hidden">
      <label class="flex flex-col gap-1 text-xs text-muted">{{ t('common.language') }}
        <select class="input" :value="locale" @change="$emit('lang', $event.target.value); $event.target.blur()">
          <option v-for="l in LANGS" :key="l.k" :value="l.k">{{ l.t }}</option>
        </select>
      </label>
      <div class="font-mono text-[11px] text-faint">{{ host }}</div>
    </div>
  </aside>`,
};
