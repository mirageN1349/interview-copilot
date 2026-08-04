<script setup lang="ts">
import type { DiagramNode } from '../model/diagram'

defineProps<{
  node: DiagramNode
  selected: boolean
  relationshipId: string
}>()

const emit = defineEmits<{
  focus: [nodeId: string]
}>()
</script>

<template>
  <button
    type="button"
    data-diagram-node
    :data-node-id="node.id"
    class="absolute min-w-28 rounded-lg border bg-(--surface-elevated) px-3 py-2 text-left text-sm font-medium shadow-sm outline-none transition-[border-color,box-shadow] duration-150 focus-visible:ring-2 focus-visible:ring-(--focus-ring)"
    :class="selected ? 'border-(--accent) ring-1 ring-(--accent)' : 'border-(--border)'"
    :style="{ transform: `translate(${node.x}px, ${node.y}px)` }"
    :aria-describedby="relationshipId"
    aria-keyshortcuts="ArrowLeft ArrowRight ArrowUp ArrowDown Shift+ArrowLeft Shift+ArrowRight Shift+ArrowUp Shift+ArrowDown Enter Delete C"
    @focus="emit('focus', node.id)"
    @click="emit('focus', node.id)"
  >
    {{ node.label }}
  </button>
</template>
