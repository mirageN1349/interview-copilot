<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, useId, watch } from 'vue'

const props = withDefaults(
  defineProps<{
    open: boolean;
    title?: string;
    description?: string;
    dismissible?: boolean;
  }>(),
  { title: "Dialog", description: undefined, dismissible: true },
);

const emit = defineEmits<{
  "update:open": [open: boolean];
}>();

const dialog = ref<HTMLDialogElement>();
const previousFocus = ref<HTMLElement | null>(null)
const titleId = useId()
const descriptionId = useId()

watch(
  () => props.open,
  async (open) => {
    await nextTick();
    if (open && !dialog.value?.open) {
      previousFocus.value = document.activeElement instanceof HTMLElement ? document.activeElement : null
      dialog.value?.showModal()
    }
    if (!open && dialog.value?.open) dialog.value.close()
  },
  { immediate: true },
);

function close() {
  if (props.dismissible) emit("update:open", false);
}

function closed() {
  if (props.open) emit('update:open', false)
  previousFocus.value?.focus()
  previousFocus.value = null
}

onBeforeUnmount(() => {
  if (dialog.value?.open) dialog.value.close()
})
</script>

<template>
  <dialog
    ref="dialog"
    class="ui-dialog"
    :aria-labelledby="title ? titleId : undefined"
    :aria-describedby="description ? descriptionId : undefined"
    @cancel.prevent="close"
    @close="closed"
    @click.self="close"
  >
    <section class="ui-dialog__panel">
      <header
        v-if="title || description"
        class="ui-dialog__header"
      >
        <h2
          v-if="title"
          :id="titleId"
        >
          {{ title }}
        </h2>
        <p
          v-if="description"
          :id="descriptionId"
        >
          {{ description }}
        </p>
      </header>
      <slot />
      <footer
        v-if="$slots.footer"
        class="ui-dialog__footer"
      >
        <slot name="footer" />
      </footer>
    </section>
  </dialog>
</template>

<style scoped>
.ui-dialog {
  width: min(30rem, calc(100vw - 2rem));
  max-height: calc(100vh - 2rem);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 0;
  background: transparent;
  color: var(--foreground);
  box-shadow: var(--shadow-dialog);
  opacity: 0;
  transition: opacity var(--motion-medium) var(--ease-out-expo);
}

.ui-dialog:not([open]) {
  pointer-events: none;
}

.ui-dialog::backdrop {
  background: oklch(0.12 0.015 250 / 0.42);
  opacity: 0;
  transition: opacity var(--motion-medium) var(--ease-out-expo);
}

.ui-dialog[open],
.ui-dialog[open]::backdrop {
  opacity: 1;
}

.ui-dialog__panel {
  display: grid;
  gap: 1rem;
  padding: 1.25rem;
  background: var(--surface-raised);
  transform: translateY(0) scale(1);
  transition: transform var(--motion-medium) var(--ease-out-expo);
}

@starting-style {
  .ui-dialog[open],
  .ui-dialog[open]::backdrop {
    opacity: 0;
  }

  .ui-dialog[open] .ui-dialog__panel {
    transform: translateY(0.5rem) scale(0.985);
  }
}

.ui-dialog__header {
  display: grid;
  gap: 0.25rem;
}

.ui-dialog__header h2,
.ui-dialog__header p {
  margin: 0;
}

.ui-dialog__header h2 {
  font-size: 1rem;
}

.ui-dialog__header p {
  color: var(--muted-foreground);
}

.ui-dialog__footer {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
}
</style>
