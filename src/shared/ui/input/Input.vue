<script setup lang="ts">
defineOptions({ inheritAttrs: false });

defineProps<{
  modelValue?: string;
  type?: string;
}>();

defineEmits<{
  "update:modelValue": [value: string];
}>();
</script>

<template>
  <input
    v-bind="$attrs"
    class="ui-input"
    :type="type ?? 'text'"
    :value="modelValue"
    @input="$emit('update:modelValue', ($event.target as HTMLInputElement).value)"
  >
</template>

<style scoped>
.ui-input {
  width: 100%;
  min-height: var(--control-height);
  border: 1px solid var(--input);
  border-radius: var(--radius-sm);
  padding: 0 0.75rem;
  background: var(--surface-raised);
}

.ui-input::placeholder {
  color: var(--muted-foreground);
}

.ui-input:disabled {
  cursor: not-allowed;
  opacity: 0.55;
}
</style>
