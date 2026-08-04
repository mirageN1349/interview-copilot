<script setup lang="ts">
withDefaults(
  defineProps<{
    type?: "button" | "submit" | "reset";
    variant?: "primary" | "secondary" | "ghost" | "danger";
    size?: "sm" | "md" | "lg" | "icon";
    disabled?: boolean;
  }>(),
  { type: "button", variant: "primary", size: "md" },
);
</script>

<template>
  <button
    class="ui-button"
    :data-variant="variant"
    :data-size="size"
    :type="type"
    :disabled="disabled"
  >
    <slot />
  </button>
</template>

<style scoped>
.ui-button {
  display: inline-flex;
  min-height: var(--control-height);
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  border: 1px solid transparent;
  border-radius: 999px;
  padding: 0 1.125rem;
  background: var(--accent);
  color: var(--accent-foreground);
  cursor: pointer;
  font-weight: 600;
  white-space: nowrap;
}

.ui-button:hover:not(:disabled) {
  filter: brightness(0.96);
}

.ui-button:active:not(:disabled) {
  transform: scale(0.975);
}

.ui-button:disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

.ui-button[data-variant="secondary"] {
  border-color: var(--border);
  background: var(--surface-raised);
  color: var(--foreground);
}

.ui-button[data-variant="ghost"] {
  background: transparent;
  color: var(--foreground);
}

.ui-button[data-variant="danger"] {
  background: var(--danger);
  color: var(--danger-foreground);
}

.ui-button[data-size="sm"] {
  min-height: 2.25rem;
  padding-inline: 0.625rem;
  font-size: 0.8125rem;
}

.ui-button[data-size="lg"] {
  min-height: 3.25rem;
  padding-inline: 1.5rem;
  font-size: 1rem;
}

.ui-button[data-size="icon"] {
  width: var(--control-height);
  padding: 0;
}
</style>
