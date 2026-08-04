<script setup lang="ts">
import { Check, RotateCcw, X } from '@lucide/vue'
import { computed, nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { DiagramError, type Diagram, type DiagramNode } from '../model/diagram'
import {
  commitDiagramChange,
  createDiagramHistory,
  undoDiagramChange,
  type DiagramHistory,
  type DiagramOperation,
} from '../model/operations'
import {
  acceptDiagramProposal,
  rejectDiagramProposal,
  type DiagramProposal,
} from '../model/proposals'
import DiagramNodeComponent from './DiagramNode.vue'

const props = defineProps<{
  initialDiagram: Diagram
  proposal?: DiagramProposal | null
}>()

const emit = defineEmits<{
  'update:diagram': [diagram: Diagram]
  'proposal:accepted': [proposal: DiagramProposal]
  'proposal:rejected': [proposal: DiagramProposal]
}>()

const { t } = useI18n()
const editor = ref<HTMLElement>()
const history = ref<DiagramHistory>(createDiagramHistory(props.initialDiagram))
const selectedId = ref(props.initialDiagram.nodes[0]?.id ?? '')
const connectSourceId = ref<string | null>(null)
const renamingId = ref<string | null>(null)
const renameValue = ref('')
const statusKey = ref('')
const currentProposal = ref<DiagramProposal | null>(props.proposal ?? null)

watch(() => props.proposal, (proposal) => { currentProposal.value = proposal ?? null })

const selectedNode = computed(() => history.value.diagram.nodes.find(({ id }) => id === selectedId.value))

function relationshipId(nodeId: string) {
  return `diagram-relationships-${nodeId}`
}

function relationships(node: DiagramNode) {
  const labels = new Map(history.value.diagram.nodes.map((node) => [node.id, node.label]))
  const related = history.value.diagram.edges.flatMap((edge) => {
    if (edge.source === node.id) return [t('diagram.connectedTo', { label: labels.get(edge.target) })]
    if (edge.target === node.id) return [t('diagram.connectedFrom', { label: labels.get(edge.source) })]
    return []
  })
  return related.length ? related.join(', ') : t('diagram.noRelationships')
}

function commit(operations: DiagramOperation[]) {
  history.value = commitDiagramChange(history.value, operations)
  emit('update:diagram', history.value.diagram)
}

function uniqueId(prefix: string, existing: readonly { id: string }[]) {
  let suffix = existing.length + 1
  while (existing.some(({ id }) => id === `${prefix}-${suffix}`)) suffix += 1
  return `${prefix}-${suffix}`
}

async function createNode() {
  const nodes = history.value.diagram.nodes
  const id = uniqueId('node', nodes)
  commit([{ type: 'node.add', node: { id, label: t('diagram.newNode'), x: 24 + nodes.length * 32, y: 24 + nodes.length * 24 } }])
  selectedId.value = id
  await focusNode(id)
}

function moveNode(key: string, fast: boolean) {
  const node = selectedNode.value
  if (!node) return
  const step = fast ? 40 : 8
  const delta: Record<string, [number, number]> = {
    ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step],
  }
  const [dx, dy] = delta[key] ?? [0, 0]
  commit([{ type: 'node.move', nodeId: node.id, x: node.x + dx, y: node.y + dy }])
}

function startRename() {
  if (!selectedNode.value) return
  renamingId.value = selectedNode.value.id
  renameValue.value = selectedNode.value.label
  void nextTick(() => document.querySelector<HTMLInputElement>('[data-testid="diagram-rename"]')?.focus())
}

function finishRename() {
  if (!renamingId.value) return
  commit([{ type: 'node.rename', nodeId: renamingId.value, label: renameValue.value }])
  const nodeId = renamingId.value
  renamingId.value = null
  void focusNode(nodeId)
}

function deleteSelected() {
  const node = selectedNode.value
  if (!node) return
  const index = history.value.diagram.nodes.findIndex(({ id }) => id === node.id)
  commit([{ type: 'node.delete', nodeId: node.id }])
  selectedId.value = history.value.diagram.nodes[Math.min(index, history.value.diagram.nodes.length - 1)]?.id ?? ''
  if (selectedId.value) void focusNode(selectedId.value)
}

function startOrFinishConnection() {
  if (!selectedNode.value) return
  if (!connectSourceId.value) {
    connectSourceId.value = selectedNode.value.id
    statusKey.value = 'diagram.status.connect'
    return
  }
  if (connectSourceId.value === selectedNode.value.id) return
  const edges = history.value.diagram.edges
  commit([{ type: 'edge.add', edge: {
    id: uniqueId('edge', edges), source: connectSourceId.value, target: selectedNode.value.id,
  } }])
  connectSourceId.value = null
  statusKey.value = 'diagram.status.connected'
}

function undo() {
  const previous = history.value
  history.value = undoDiagramChange(previous)
  if (history.value === previous) return
  if (selectedId.value && !history.value.diagram.nodes.some(({ id }) => id === selectedId.value)) {
    selectedId.value = history.value.diagram.nodes[0]?.id ?? ''
  }
  statusKey.value = 'diagram.status.undone'
  emit('update:diagram', history.value.diagram)
}

function acceptProposal() {
  const proposal = currentProposal.value
  if (!proposal) return
  try {
    const accepted = acceptDiagramProposal(history.value, proposal)
    history.value = accepted.history
    currentProposal.value = accepted.proposal
    statusKey.value = 'diagram.status.accepted'
    emit('update:diagram', history.value.diagram)
    emit('proposal:accepted', accepted.proposal)
  } catch (error) {
    if (error instanceof DiagramError && error.code === 'diagram_revision_stale') {
      statusKey.value = 'diagram.status.stale'
      return
    }
    throw error
  }
}

function rejectProposal() {
  const proposal = currentProposal.value
  if (!proposal) return
  const rejected = rejectDiagramProposal(history.value, proposal)
  currentProposal.value = rejected.proposal
  statusKey.value = 'diagram.status.rejected'
  emit('proposal:rejected', rejected.proposal)
}

async function focusNode(nodeId: string) {
  await nextTick()
  Array.from(editor.value?.querySelectorAll<HTMLElement>('[data-node-id]') ?? [])
    .find((node) => node.dataset.nodeId === nodeId)?.focus()
}

function onKeydown(event: KeyboardEvent) {
  if (event.target instanceof HTMLInputElement) {
    if (event.key === 'Enter') { event.preventDefault(); finishRename() }
    if (event.key === 'Escape') { event.preventDefault(); renamingId.value = null; void focusNode(selectedId.value) }
    return
  }
  if (event.metaKey && event.key === 'Enter') { event.preventDefault(); acceptProposal(); return }
  if (event.metaKey && event.key === 'Backspace') { event.preventDefault(); rejectProposal(); return }
  if (event.metaKey && event.key.toLowerCase() === 'z') { event.preventDefault(); undo(); return }
  if (event.key.startsWith('Arrow')) { event.preventDefault(); moveNode(event.key, event.shiftKey); return }
  if (!event.metaKey && !event.ctrlKey && !event.altKey && event.key.toLowerCase() === 'n') { event.preventDefault(); void createNode(); return }
  if (!event.metaKey && !event.ctrlKey && !event.altKey && event.key.toLowerCase() === 'c') { event.preventDefault(); startOrFinishConnection(); return }
  if (event.key === 'Enter') {
    event.preventDefault()
    if (connectSourceId.value) startOrFinishConnection()
    else startRename()
    return
  }
  if (event.key === 'Delete' || event.key === 'Backspace') { event.preventDefault(); deleteSelected() }
}
</script>

<template>
  <section
    ref="editor"
    data-testid="diagram-editor"
    class="grid h-full min-h-80 grid-rows-[auto_1fr_auto] gap-2 outline-none"
    tabindex="0"
    role="region"
    :aria-label="t('diagram.title')"
    @keydown="onKeydown"
  >
    <header class="flex items-center justify-between gap-3">
      <h2 class="text-sm font-semibold">
        {{ t('diagram.title') }}
      </h2>
      <button
        type="button"
        class="inline-flex items-center gap-1 rounded-md px-2 py-1 text-xs text-(--muted-foreground) hover:bg-(--surface-hover) focus-visible:outline-2 focus-visible:outline-(--focus-ring)"
        :disabled="history.undoStack.length === 0"
        aria-keyshortcuts="Meta+Z"
        @click="undo"
      >
        <RotateCcw
          :size="14"
          aria-hidden="true"
        />{{ t('diagram.undo') }}
      </button>
    </header>

    <div
      class="relative min-h-64 overflow-auto rounded-lg border border-(--border) bg-(--surface-subtle)"
      role="group"
      :aria-label="t('diagram.canvas')"
    >
      <svg
        class="pointer-events-none absolute inset-0 h-full min-h-64 w-full"
        aria-hidden="true"
      >
        <line
          v-for="edge in history.diagram.edges"
          :key="edge.id"
          :x1="(history.diagram.nodes.find((node) => node.id === edge.source)?.x ?? 0) + 56"
          :y1="(history.diagram.nodes.find((node) => node.id === edge.source)?.y ?? 0) + 20"
          :x2="(history.diagram.nodes.find((node) => node.id === edge.target)?.x ?? 0) + 56"
          :y2="(history.diagram.nodes.find((node) => node.id === edge.target)?.y ?? 0) + 20"
          stroke="currentColor"
          class="text-(--border-strong)"
        />
      </svg>

      <DiagramNodeComponent
        v-for="node in history.diagram.nodes"
        :key="node.id"
        :node="node"
        :selected="selectedId === node.id"
        :relationship-id="relationshipId(node.id)"
        @focus="selectedId = $event"
      />
      <span
        v-for="node in history.diagram.nodes"
        :id="relationshipId(node.id)"
        :key="`relationship-${node.id}`"
        class="sr-only"
      >{{ t('diagram.relationships', { label: node.label, relationships: relationships(node) }) }}</span>

      <label
        v-if="renamingId"
        class="absolute bottom-3 left-3 right-3 grid gap-1 rounded-lg border border-(--border) bg-(--surface-elevated) p-3 shadow-(--shadow-dialog)"
      >
        <span class="text-xs font-medium">{{ t('diagram.rename') }}</span>
        <input
          v-model="renameValue"
          data-testid="diagram-rename"
          class="rounded-md border border-(--border) bg-transparent px-2 py-1 text-sm outline-none focus-visible:ring-2 focus-visible:ring-(--focus-ring)"
        >
      </label>
    </div>

    <aside
      v-if="currentProposal?.status === 'pending'"
      class="flex items-center justify-between gap-3 rounded-lg border border-(--border) bg-(--surface-elevated) p-2"
    >
      <span class="text-xs">{{ t('diagram.proposal') }}</span>
      <span class="flex gap-1">
        <button
          type="button"
          class="rounded-md p-1.5 hover:bg-(--surface-hover) focus-visible:outline-2 focus-visible:outline-(--focus-ring)"
          :aria-label="t('diagram.reject')"
          aria-keyshortcuts="Meta+Backspace"
          @click="rejectProposal"
        >
          <X
            :size="16"
            aria-hidden="true"
          />
        </button>
        <button
          type="button"
          class="rounded-md p-1.5 hover:bg-(--surface-hover) focus-visible:outline-2 focus-visible:outline-(--focus-ring)"
          :aria-label="t('diagram.accept')"
          aria-keyshortcuts="Meta+Enter"
          @click="acceptProposal"
        >
          <Check
            :size="16"
            aria-hidden="true"
          />
        </button>
      </span>
    </aside>
    <p
      class="sr-only"
      aria-live="polite"
    >
      {{ statusKey ? t(statusKey) : '' }}
    </p>
  </section>
</template>
