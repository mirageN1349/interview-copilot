<script setup lang="ts">
import { CircleStop, GripHorizontal, MessageCircle, MessagesSquare, Network, RotateCcw, Settings2 } from '@lucide/vue'
import { storeToRefs } from 'pinia'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'
import { useQuery } from '@tanstack/vue-query'
import { listen } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { TabsIndicator, TabsList, TabsRoot, TabsTrigger } from 'reka-ui'

import { meetingGateway, meetingQuery } from '@/entities/meeting'
import type { AnswerDepth } from '@/entities/interview-profile'
import { sendMeetingEnvelope, subscribeMeetingEnvelopes } from '@/app/providers/meeting-socket'
import { nativeGateway } from '@/shared/api/native'
import Button from '@/shared/ui/button/Button.vue'
import Dialog from '@/shared/ui/dialog/Dialog.vue'
import CaptureContextControls from '@/features/capture-context/ui/CaptureContextControls.vue'
import { createCaptureActions, type ScreenshotArtifact } from '@/features/capture-context/model/capture-actions'
import { useCaptureUiStore } from '@/features/capture-context/model/capture-ui-store'
import { createDiagram, type Diagram } from '@/features/edit-diagram/model/diagram'
import { parseDiagramProposal, type DiagramProposal } from '@/features/edit-diagram/model/proposals'
import DiagramEditor from '@/features/edit-diagram/ui/DiagramEditor.vue'
import { createDevAudioLoop, createMeetingChatActions } from '../model/chat-actions'
import { applyMeetingEvent, createMeetingViewState } from '../model/meeting-reducer'
import { useOverlayUiStore, type OverlaySection } from '../model/overlay-ui-store'
import LiveChat from './LiveChat.vue'
import SideChat from './SideChat.vue'

const { t } = useI18n()
const route = useRoute()
const meetingId = computed(() => String(route.params.meetingId ?? ''))
const meeting = useQuery(computed(() => meetingQuery(meetingId.value)))
const store = useOverlayUiStore()
const captureUi = useCaptureUiStore()
const { activeSection, visibility } = storeToRefs(store)
const view = ref(createMeetingViewState(meetingId.value, 0.72))
type LiveTimelineEntry =
  | { id: string; role: 'interviewer'; question: { segmentId: string; text: string; confidence: number; requiresConfirmation: boolean } }
  | { id: string; role: 'assistant' }
type SideTimelineEntry =
  | { id: string; role: 'user'; content: string }
  | { id: string; role: 'assistant' }
const liveTimeline = ref<LiveTimelineEntry[]>([])
const sideTimeline = ref<SideTimelineEntry[]>([])
const diagram = ref<Diagram>(createDiagram({ revision: 0, nodes: [], edges: [] }))
const diagramProposal = ref<DiagramProposal | null>(null)
const resetOpen = ref(false)
const stopping = ref(false)
const chatActions = createMeetingChatActions({
  send: sendMeetingEnvelope,
  resetNative: meetingGateway.resetContext,
})
const captureActions = createCaptureActions({
  capture: (input) => nativeGateway.invoke('capture_screenshot', { input }),
  send: async (input) => {
    const current = meeting.data.value
    if (!current) throw new Error('Meeting unavailable')
    chatActions.sendMessage({ ...input, launchPolicyId: current.launchPolicyId })
  },
})
const tabs: Array<{ id: OverlaySection; icon: typeof MessageCircle }> = [
  { id: 'live', icon: MessageCircle },
  { id: 'side', icon: MessagesSquare },
  { id: 'design', icon: Network },
  { id: 'status', icon: Settings2 },
]
let unlistenEnvelope: (() => void) | undefined
let unlistenHotkey: (() => void) | undefined
let unlistenMeetingState: (() => void) | undefined
let demoFragment = 0
let transportReady = false
let resumeSent = false
const liveDemo = createDevAudioLoop(() => {
  const current = meeting.data.value
  if (!import.meta.env.DEV || !current || stopping.value) return
  const endedAtMs = Date.now()
  demoFragment += 1
  sendMeetingEnvelope({
    v: 1,
    id: crypto.randomUUID(),
    type: 'audio.fragment.ready',
    sentAt: new Date(endedAtMs).toISOString(),
    meetingId: current.id,
    launchPolicyId: current.launchPolicyId,
    payload: {
      artifactId: `audio-${endedAtMs}-${demoFragment}`,
      startedAtMs: endedAtMs - 2_400,
      endedAtMs,
      source: 'system',
      contentStatus: 'allowed',
    },
  })
})

function selectRelative(direction: 1 | -1) {
  const index = tabs.findIndex((tab) => tab.id === activeSection.value)
  store.setSection(tabs[(index + direction + tabs.length) % tabs.length]!.id)
}

async function move(dx: number, dy: number) {
  store.move(dx, dy)
  await nativeGateway.invoke('overlay_move', { input: { meetingId: meetingId.value, dx, dy } }).catch(() => undefined)
}

function keydown(event: KeyboardEvent) {
  if (event.ctrlKey && event.key === 'Tab') {
    event.preventDefault()
    selectRelative(event.shiftKey ? -1 : 1)
    return
  }
  if (event.ctrlKey && event.altKey && event.key.startsWith('Arrow')) {
    event.preventDefault()
    const step = event.shiftKey ? 48 : 12
    const delta = {
      ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step],
    }[event.key]
    if (delta) void move(delta[0], delta[1])
  }
}

async function beginDrag(event: PointerEvent) {
  if (event.button !== 0 || !('__TAURI_INTERNALS__' in window)) return
  event.preventDefault()
  await getCurrentWindow().startDragging()
}

async function beginResize(event: PointerEvent) {
  event.preventDefault()
  if (!('__TAURI_INTERNALS__' in window)) return
  await getCurrentWindow().startResizeDragging('SouthEast')
}

async function stop() {
  if (stopping.value) return
  liveDemo.stop()
  stopping.value = true
  try {
    const stopped = await meetingGateway.stop(meetingId.value, 'user')
    try {
      sendMeetingEnvelope({
        v: 1, id: crypto.randomUUID(), type: 'meeting.stop', sentAt: new Date().toISOString(),
        meetingId: stopped.id, launchPolicyId: stopped.launchPolicyId, payload: { reason: 'user' },
      })
    } catch {
      // Native stop is authoritative; transport recovery is independent.
    }
    store.hide()
    stopping.value = false
    void getCurrentWindow().hide()
  } finally {
    stopping.value = false
  }
}

function sendChat(thread: 'live' | 'side', content: string, artifactIds: string[] = [], answerDepth?: AnswerDepth) {
  const current = meeting.data.value
  if (!current) return
  chatActions.sendMessage({
    meetingId: current.id,
    launchPolicyId: current.launchPolicyId,
    thread,
    content,
    artifactIds,
    contextGeneration: view.value.contextGeneration,
    ...(answerDepth ? { answerDepth } : {}),
  })
}

async function sendUserMessage(thread: 'live' | 'side', content: string, answerDepth?: AnswerDepth) {
  const current = meeting.data.value
  if (thread === 'side') {
    sideTimeline.value.push({ id: crypto.randomUUID(), role: 'user', content })
  }
  if (!current || captureUi.autoScreenshotMode === 'off' || captureUi.selectedDisplayId === null) {
    sendChat(thread, content, [], answerDepth)
    return
  }
  const artifact = await captureActions.sendWithContext({
    meetingId: current.id,
    thread,
    content,
    contextGeneration: view.value.contextGeneration,
    answerDepth,
    displayId: captureUi.selectedDisplayId,
    mode: captureUi.autoScreenshotMode,
    ...(captureUi.autoScreenshotMode === 'area' && captureUi.areaDraft ? { area: captureUi.areaDraft } : {}),
  })
  captureUi.lastRedactionSummary = artifact.redactionSummary ?? null
}

function analyzeCapture(artifact: ScreenshotArtifact) {
  sendChat(activeSection.value === 'side' ? 'side' : 'live', t('capture.analyze'), [artifact.id])
}

async function hotkeyCapture(mode: 'display' | 'area') {
  const current = meeting.data.value
  if (!current || captureUi.selectedDisplayId === null || (mode === 'area' && !captureUi.areaDraft)) return
  const artifact = await captureActions.capture({
    meetingId: current.id,
    displayId: captureUi.selectedDisplayId,
    mode,
    ...(mode === 'area' && captureUi.areaDraft ? { area: captureUi.areaDraft } : {}),
    thread: activeSection.value === 'side' ? 'side' : 'live',
  })
  analyzeCapture(artifact)
}

async function resetContext() {
  const current = meeting.data.value
  if (!current) return
  const generation = await chatActions.resetContext({
    meetingId: current.id,
    launchPolicyId: current.launchPolicyId,
    currentGeneration: view.value.contextGeneration,
  })
  view.value = { ...view.value, contextGeneration: generation, pendingQuestion: null, outbound: [] }
  resetOpen.value = false
}

function applyEnvelope(envelope: { id: string; type: string; meetingId?: string; sequence?: number; payload: Record<string, unknown> }) {
  if (envelope.meetingId !== meetingId.value || envelope.sequence === undefined) return
  if (envelope.type === 'question.detected' && !liveTimeline.value.some((entry) => entry.id === String(envelope.payload.segmentId))) {
    const confidence = Number(envelope.payload.confidence)
    liveTimeline.value.push({
      id: String(envelope.payload.segmentId),
      role: 'interviewer',
      question: {
        segmentId: String(envelope.payload.segmentId),
        text: String(envelope.payload.text),
        confidence,
        requiresConfirmation: confidence < view.value.confidenceThreshold,
      },
    })
  }
  if (envelope.type === 'answer.started') {
    const thread = envelope.payload.thread === 'side' ? 'side' : 'live'
    const timeline = thread === 'side' ? sideTimeline.value : liveTimeline.value
    const messageId = String(envelope.payload.messageId)
    if (!timeline.some((entry) => entry.role === 'assistant' && entry.id === messageId)) {
      timeline.push({ id: messageId, role: 'assistant' })
    }
  }
  if (envelope.type === 'diagram.patch.proposed') {
    diagramProposal.value = parseDiagramProposal(envelope.payload)
  }
  if (!['question.detected', 'answer.started', 'answer.delta', 'answer.completed', 'context.reset.completed'].includes(envelope.type)) {
    if (envelope.type === 'meeting.completed') liveDemo.stop()
    if (envelope.sequence === view.value.lastAppliedSequence + 1) {
      view.value = {
        ...view.value,
        lastAppliedSequence: envelope.sequence,
        appliedEventIds: [...view.value.appliedEventIds, envelope.id],
      }
      if (import.meta.env.DEV && ['meeting.accepted', 'meeting.snapshot'].includes(envelope.type)) liveDemo.start()
    }
    return
  }
  const previousOutbound = view.value.outbound.length
  view.value = applyMeetingEvent(view.value, {
    id: envelope.id,
    sequence: envelope.sequence,
    type: envelope.type as Parameters<typeof applyMeetingEvent>[1]['type'],
    payload: envelope.payload,
  })
  const automatic = view.value.outbound.at(previousOutbound)
  if (automatic && envelope.type === 'question.detected') sendChat('live', String(envelope.payload.text ?? ''))
}

function requestDiagramProposal() {
  const current = meeting.data.value
  if (!current) return
  sendMeetingEnvelope({
    v: 1,
    id: crypto.randomUUID(),
    type: 'diagram.proposal.request',
    sentAt: new Date().toISOString(),
    meetingId: current.id,
    launchPolicyId: current.launchPolicyId,
    payload: { diagramRevision: diagram.value.revision },
  })
}

function resumeMeeting() {
  const current = meeting.data.value
  if (!transportReady || resumeSent || !current) return
  resumeSent = true
  sendMeetingEnvelope({
    v: 1,
    id: crypto.randomUUID(),
    type: 'meeting.resume',
    sentAt: new Date().toISOString(),
    meetingId: current.id,
    launchPolicyId: current.launchPolicyId,
    payload: { lastAppliedSequence: view.value.lastAppliedSequence, contextGeneration: view.value.contextGeneration },
  })
}

onMounted(async () => {
  window.addEventListener('keydown', keydown)
  unlistenEnvelope = await subscribeMeetingEnvelopes(applyEnvelope)
  transportReady = true
  resumeMeeting()
  if ('__TAURI_INTERNALS__' in window) {
    await nextTick()
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()))
    await nativeGateway.invoke('overlay_ready', { input: { meetingId: meetingId.value } })
  }
  if ('__TAURI_INTERNALS__' in window) {
    unlistenMeetingState = await listen<{ status: string }>('meeting://state', ({ payload }) => {
      if (payload.status !== 'running') liveDemo.stop()
    })
    unlistenHotkey = await listen<{ actionId: string }>('hotkey://action', async ({ payload }) => {
      if (payload.actionId === 'overlay.toggle') {
        if (visibility.value === 'hidden') {
          await nativeGateway.invoke('overlay_show', { input: { meetingId: meetingId.value } })
          store.show(true)
        } else {
          await nativeGateway.invoke('overlay_hide', { input: { meetingId: meetingId.value } })
          store.hide()
        }
      }
      if (payload.actionId === 'overlay.interactive') {
        const interactive = visibility.value !== 'visible_interactive'
        await nativeGateway.invoke('overlay_set_interactive', { input: { interactive } })
        store.show(interactive)
      }
      if (payload.actionId === 'chat.live.focus' || payload.actionId === 'chat.side.focus') {
        store.setSection(payload.actionId === 'chat.live.focus' ? 'live' : 'side')
        store.show(true)
      }
      if (payload.actionId === 'context.reset') resetOpen.value = true
      if (payload.actionId === 'capture.full') await hotkeyCapture('display')
      if (payload.actionId === 'capture.area') await hotkeyCapture('area')
    })
  }
})
watch(() => meeting.data.value, resumeMeeting)
watch(activeSection, (section) => {
  if (section === 'design' && !diagramProposal.value) requestDiagramProposal()
})
onBeforeUnmount(() => {
  liveDemo.stop()
  window.removeEventListener('keydown', keydown)
  unlistenEnvelope?.()
  unlistenHotkey?.()
  unlistenMeetingState?.()
})
</script>

<template>
  <main
    v-if="visibility !== 'hidden'"
    class="overlay-window-root h-screen overflow-hidden text-(--foreground)"
  >
    <section
      class="overlay-panel relative grid h-full grid-rows-[1fr_auto] overflow-hidden rounded-[1.375rem]"
      :aria-label="t('overlay.title')"
    >
      <div class="overlay-chrome absolute inset-x-0 top-0 z-20 pb-2">
        <header class="flex h-11 items-center justify-between gap-3 px-3.5">
          <button
            class="flex min-w-0 flex-1 cursor-grab items-center gap-2 text-[0.8125rem] font-semibold active:cursor-grabbing"
            type="button"
            :aria-label="t('overlay.move')"
            @pointerdown="beginDrag"
          >
            <GripHorizontal
              :size="16"
              aria-hidden="true"
            />
            <span>{{ meeting.data.value?.title ?? t('overlay.title') }}</span>
          </button>
          <span class="overlay-status inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[0.6875rem] font-medium text-(--muted-foreground)">
            <span class="size-1.5 rounded-full bg-emerald-400 shadow-[0_0_0.45rem_oklch(0.76_0.16_155/0.7)]" />
            {{ t(`meeting.capture.${meeting.data.value?.capturePhase ?? 'idle'}`) }}
          </span>
        </header>

        <TabsRoot
          v-model="activeSection"
          as-child
        >
          <TabsList
            as="nav"
            class="overlay-segments relative mx-3 grid grid-cols-4 rounded-full p-1"
            :aria-label="t('overlay.sections')"
          >
            <TabsIndicator
              class="overlay-segment-indicator pointer-events-none absolute inset-y-1 left-0"
              aria-hidden="true"
            />
            <TabsTrigger
              v-for="tab in tabs"
              :id="`overlay-${tab.id}-tab`"
              :key="tab.id"
              :value="tab.id"
              class="relative z-1 inline-flex min-h-8 items-center justify-center gap-1.5 rounded-full px-2 text-xs font-medium text-(--muted-foreground) transition-colors hover:text-(--foreground) data-[state=active]:text-(--foreground)"
            >
              <component
                :is="tab.icon"
                :size="16"
                aria-hidden="true"
              />
              {{ t(`overlay.tab.${tab.id}`) }}
            </TabsTrigger>
          </TabsList>
        </TabsRoot>
      </div>

      <Transition
        name="overlay-content"
        mode="out-in"
      >
        <LiveChat
          v-if="activeSection === 'live'"
          key="live"
          :messages="view.messages.live"
          :question="view.pendingQuestion"
          :timeline="liveTimeline"
          @confirm-question="view.pendingQuestion && sendUserMessage('live', view.pendingQuestion.text)"
        />
        <SideChat
          v-else-if="activeSection === 'side'"
          key="side"
          :messages="view.messages.side"
          :timeline="sideTimeline"
          @send="sendUserMessage('side', $event.content, $event.depth)"
          @attach="hotkeyCapture('display')"
        />
        <section
          v-else-if="activeSection === 'design'"
          key="design"
          class="min-h-0 overflow-hidden px-4 pb-4 pt-[6.25rem]"
        >
          <DiagramEditor
            :initial-diagram="diagram"
            :proposal="diagramProposal"
            @update:diagram="diagram = $event"
            @proposal:accepted="diagramProposal = $event"
            @proposal:rejected="diagramProposal = $event"
          />
        </section>
        <section
          v-else
          key="status"
          class="min-h-0 overflow-y-auto px-4 pb-4 pt-[6.25rem]"
        >
          <dl class="mb-4 grid content-start grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-sm">
            <dt class="text-(--muted-foreground)">
              {{ t('overlay.status.profile') }}
            </dt><dd>{{ meeting.data.value?.profileId ?? '—' }}</dd>
            <dt class="text-(--muted-foreground)">
              {{ t('overlay.status.connection') }}
            </dt><dd>{{ t('overlay.status.online') }}</dd>
            <dt class="text-(--muted-foreground)">
              {{ t('overlay.status.context') }}
            </dt><dd>{{ view.contextGeneration }}</dd>
          </dl>
          <CaptureContextControls
            v-if="meeting.data.value"
            :meeting-id="meeting.data.value.id"
            :initial-display-id="meeting.data.value.displayId"
            :thread="activeSection === 'side' ? 'side' : 'live'"
            @captured="analyzeCapture"
          />
        </section>
      </Transition>

      <footer class="overlay-footer relative z-20 flex items-center justify-between gap-3 px-3 py-2">
        <span class="truncate text-[0.6875rem] text-(--muted-foreground)">{{ t('overlay.commandHint') }}</span>
        <div class="flex gap-2">
          <Button
            size="icon"
            variant="ghost"
            :aria-label="t('overlay.reset.action')"
            :title="t('overlay.reset.action')"
            @click="resetOpen = true"
          >
            <RotateCcw
              :size="16"
              aria-hidden="true"
            />
          </Button>
          <Button
            size="icon"
            variant="danger"
            :aria-label="t('meeting.controls.stop')"
            :title="t('meeting.controls.stop')"
            :disabled="stopping"
            @click="stop"
          >
            <CircleStop
              :size="16"
              aria-hidden="true"
            />
          </Button>
        </div>
      </footer>
      <button
        class="absolute bottom-0 right-0 z-10 size-4 cursor-nwse-resize opacity-0"
        type="button"
        :aria-label="t('overlay.resize')"
        @pointerdown="beginResize"
      />
    </section>
    <Dialog
      v-model:open="resetOpen"
      :title="t('overlay.reset.title')"
      :description="t('overlay.reset.description')"
    >
      <template #footer>
        <Button
          variant="secondary"
          @click="resetOpen = false"
        >
          {{ t('common.cancel') }}
        </Button>
        <Button @click="resetContext">
          {{ t('overlay.reset.confirm') }}
        </Button>
      </template>
    </Dialog>
  </main>
</template>

<style scoped>
.overlay-window-root {
  border-radius: 1.375rem;
  clip-path: inset(0 round 1.375rem);
}

.overlay-panel {
  background: transparent;
}

.overlay-status {
  background: color-mix(in oklch, var(--surface-raised) 28%, transparent);
  box-shadow: inset 0 1px 0 color-mix(in oklch, var(--surface-raised) 24%, transparent);
}

.overlay-chrome {
  isolation: isolate;
  background: transparent;
}

.overlay-chrome::before {
  position: absolute;
  z-index: -1;
  inset: -0.5rem 0 -2.75rem;
  background: linear-gradient(to bottom, color-mix(in oklch, var(--surface) 34%, transparent), transparent);
  backdrop-filter: blur(18px) saturate(135%);
  -webkit-backdrop-filter: blur(18px) saturate(135%);
  mask-image: linear-gradient(to bottom, black 0 58%, transparent 100%);
  -webkit-mask-image: linear-gradient(to bottom, black 0 58%, transparent 100%);
  pointer-events: none;
  content: "";
}

.overlay-segments {
  background: color-mix(in oklch, var(--surface-raised) 32%, transparent);
  box-shadow: inset 0 1px 0 color-mix(in oklch, var(--surface-raised) 28%, transparent);
}

.overlay-segment-indicator {
  width: var(--reka-tabs-indicator-size);
  border-radius: 999px;
  background: color-mix(in oklch, var(--surface-raised) 82%, transparent);
  box-shadow: 0 2px 9px oklch(0.15 0.015 250 / 0.24), inset 0 1px 0 color-mix(in oklch, var(--surface-raised) 58%, transparent);
  transform: translate3d(var(--reka-tabs-indicator-position), 0, 0);
  transition: transform 320ms cubic-bezier(0.22, 1, 0.36, 1), width 320ms cubic-bezier(0.22, 1, 0.36, 1);
  will-change: transform;
}

.overlay-footer::before {
  position: absolute;
  z-index: -1;
  inset: -2.75rem 0 0;
  background: linear-gradient(to bottom, transparent, color-mix(in oklch, var(--surface) 34%, transparent));
  backdrop-filter: blur(18px) saturate(135%);
  -webkit-backdrop-filter: blur(18px) saturate(135%);
  mask-image: linear-gradient(to bottom, transparent 0%, black 42% 100%);
  -webkit-mask-image: linear-gradient(to bottom, transparent 0%, black 42% 100%);
  pointer-events: none;
  content: "";
}

.overlay-content-enter-active,
.overlay-content-leave-active {
  transition: opacity 210ms cubic-bezier(0.22, 1, 0.36, 1), transform 210ms cubic-bezier(0.22, 1, 0.36, 1);
}

.overlay-content-enter-from {
  opacity: 0;
  transform: translate3d(12px, 0, 0);
}

.overlay-content-leave-to {
  opacity: 0;
  transform: translate3d(-10px, 0, 0);
}

@media (prefers-reduced-motion: reduce) {
  .overlay-segment-indicator,
  .overlay-content-enter-active,
  .overlay-content-leave-active {
    transition: none;
  }

  .overlay-content-enter-from,
  .overlay-content-leave-to {
    transform: none;
  }
}

@media (prefers-reduced-transparency: reduce) {
  .overlay-panel {
    background: var(--surface);
  }
}
</style>
