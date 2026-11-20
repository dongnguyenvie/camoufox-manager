/* Page frame shared by every sidebar page.
   <cm-page :title="t('nav.x')" :count="n">
     <template #tools>search / filters (left, after the title)</template>
     <template #actions>buttons (right)</template>
     body
   </cm-page>
   <cm-coming-soon icon="layers" :title="..."/> is the placeholder of pages whose phase has not landed. */

CM.components.CmPage = {
  props: { title: String, count: [Number, String] },
  template: `<section class="flex-1 min-h-0 flex flex-col">
    <header class="flex flex-wrap items-center gap-3 px-5 min-h-[52px] py-2 border-b border-line bg-panel/90 backdrop-blur-[10px] max-sm:px-4">
      <h1 class="m-0 text-[15px] font-semibold tracking-tight whitespace-nowrap">{{ title }}</h1>
      <span v-if="count !== undefined && count !== null" class="text-[13px] text-faint -ml-1">{{ count }}</span>
      <slot name="tools"></slot>
      <div class="flex-1"></div>
      <slot name="actions"></slot>
      <cm-bell/>
    </header>
    <div class="flex-1 min-h-0 flex flex-col px-5 pt-3.5 pb-4 max-sm:px-4"><slot></slot></div>
  </section>`,
};

CM.components.CmComingSoon = {
  props: { icon: String, title: String, body: String },
  template: `<div class="flex-1 grid place-items-center">
    <div class="max-w-[420px] text-center flex flex-col items-center gap-3 p-8 rounded-xl border border-dashed border-line bg-panel">
      <div class="size-11 rounded-xl grid place-items-center bg-accent2 text-accent"><cm-icon :name="icon || 'clock'" :size="22"/></div>
      <div class="text-[15px] font-semibold">{{ title || t('common.comingSoon') }}</div>
      <p class="m-0 text-[13px] text-muted">{{ body || t('common.comingSoonBody') }}</p>
      <slot></slot>
    </div>
  </div>`,
};
