<script setup lang="ts">
import { shallowRef, watch } from 'vue'

import {
  highlightCode,
  type CodeTheme,
  type HighlightResult,
} from '@/shared/lib/highlight-code'

const props = withDefaults(
  defineProps<{
    code: string
    language: string
    theme?: CodeTheme
    ariaLabel?: string
  }>(),
  { theme: 'github-dark', ariaLabel: undefined },
)

const rendered = shallowRef<HighlightResult>({ kind: 'plain', text: props.code })

watch(
  () => ({ code: props.code, language: props.language, theme: props.theme }),
  async (input, _, onCleanup) => {
    let current = true
    onCleanup(() => { current = false })
    const result = await highlightCode(input.code, input.language, { theme: input.theme })
    if (current) rendered.value = result
  },
  { immediate: true },
)
</script>

<template>
  <!-- eslint-disable vue/no-v-html -->
  <div
    class="overflow-auto rounded-xl border border-[var(--border)] bg-[var(--surface-raised)] text-sm [&_.shiki]:m-0 [&_.shiki]:overflow-auto [&_.shiki]:p-4 [&_code]:font-mono [&_code]:leading-6"
    :aria-label="ariaLabel"
    :data-highlighted="rendered.kind === 'highlighted'"
  >
    <!-- Shiki output is reduced to pre/code/span and safe attributes before insertion. -->
    <div
      v-if="rendered.kind === 'highlighted'"
      v-html="rendered.html"
    />
    <pre
      v-else
      class="m-0 overflow-auto p-4"
    ><code>{{ rendered.text }}</code></pre>
  </div>
  <!-- eslint-enable vue/no-v-html -->
</template>
