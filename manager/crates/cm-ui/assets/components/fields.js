/* Form fields.
   <cm-field :label="t('x')" :hint="..." :error="..."> <input class="input" v-model="..."> </cm-field>
   Inputs/selects/textareas use the `.input` class (app.css). <cm-switch v-model="b" :label="..."/> for booleans. */

CM.components.CmField = {
  props: { label: String, hint: String, error: String },
  template: `<label class="flex flex-col gap-1 text-xs text-muted">
    <span v-if="label">{{ label }}</span>
    <slot></slot>
    <span v-if="error" class="text-bad">{{ error }}</span>
    <span v-else-if="hint" class="text-faint">{{ hint }}</span>
  </label>`,
};

CM.components.CmSwitch = {
  props: { modelValue: Boolean, label: String, disabled: Boolean },
  emits: ['update:modelValue'],
  template: `<label class="inline-flex items-center gap-2 text-[13px] cursor-pointer select-none" :class="{ 'opacity-50 cursor-default': disabled }">
    <button type="button" role="switch" :aria-checked="modelValue" :disabled="disabled" @click="$emit('update:modelValue', !modelValue)"
            class="relative w-8 h-[18px] rounded-full border-0 transition" :class="modelValue ? 'bg-accent' : 'bg-line'">
      <span class="absolute top-[2px] size-[14px] rounded-full bg-white shadow transition-all" :class="modelValue ? 'left-[16px]' : 'left-[2px]'"></span>
    </button>
    <span v-if="label">{{ label }}</span>
  </label>`,
};
