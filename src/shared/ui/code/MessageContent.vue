<script setup lang="ts">
import CodeBlock from './CodeBlock.vue'

type Segment =
  | { kind: 'text'; content: string }
  | { kind: 'code'; content: string; language: string }

const props = defineProps<{ content: string }>()

function segments(): Segment[] {
  const result: Segment[] = []
  const fence = /```([^\n`]*)\n([\s\S]*?)```/g
  let cursor = 0
  for (const match of props.content.matchAll(fence)) {
    const index = match.index ?? 0
    if (index > cursor) result.push({ kind: 'text', content: props.content.slice(cursor, index) })
    result.push({ kind: 'code', language: (match[1] ?? '').trim(), content: match[2] ?? '' })
    cursor = index + match[0].length
  }
  if (cursor < props.content.length) result.push({ kind: 'text', content: props.content.slice(cursor) })
  return result.length ? result : [{ kind: 'text', content: props.content }]
}
</script>

<template>
  <div class="grid gap-2">
    <template
      v-for="(segment, index) in segments()"
      :key="index"
    >
      <CodeBlock
        v-if="segment.kind === 'code'"
        :code="segment.content"
        :language="segment.language"
      />
      <p
        v-else
        class="m-0 whitespace-pre-wrap"
      >
        {{ segment.content }}
      </p>
    </template>
  </div>
</template>
