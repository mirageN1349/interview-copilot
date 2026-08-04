<script setup lang="ts">
import { ExternalLink, PanelTopOpen } from '@lucide/vue'
import { useQuery } from '@tanstack/vue-query'
import { computed, ref } from 'vue'
import { onBeforeUnmount, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'

import { meetingGateway, meetingQuery } from '@/entities/meeting'
import { sendMeetingEnvelope } from '@/app/providers/meeting-socket'
import { nativeGateway } from '@/shared/api/native'
import Button from '@/shared/ui/button/Button.vue'
import Dialog from '@/shared/ui/dialog/Dialog.vue'
import MeetingControls from '@/features/run-meeting/ui/MeetingControls.vue'
import { defaultMeetingBindings, useKeyboardStore, type MeetingActionId } from '@/features/run-meeting/model/keyboard-store'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

const { t } = useI18n()
const route = useRoute()
const meetingId = computed(() => String(route.params.meetingId ?? ''))
const meeting = useQuery(computed(() => meetingQuery(meetingId.value)))
const keyboard = useKeyboardStore()
const hotkeyConflicts = computed(() => Object.entries(keyboard.bindings).filter(([, binding]) => binding.status === 'conflict'))
let unlistenHotkey: UnlistenFn | undefined
let unlistenArtifact: UnlistenFn | undefined
const resetOpen = ref(false)

async function showOverlay() {
  await nativeGateway.invoke('overlay_show', { input: { meetingId: meetingId.value } })
}

onMounted(async () => {
  if (!('__TAURI_INTERNALS__' in window)) return
  await meeting.refetch()
  const current = meeting.data.value
  if (current) {
    unlistenArtifact = await listen<string>('capture://artifact', ({ payload: artifactId }) => {
      const endedAtMs = Date.now()
      sendMeetingEnvelope({
        v: 1,
        id: crypto.randomUUID(),
        type: 'audio.fragment.ready',
        sentAt: new Date().toISOString(),
        meetingId: current.id,
        launchPolicyId: current.launchPolicyId,
        payload: { artifactId, startedAtMs: endedAtMs - 3_000, endedAtMs, source: 'system', contentStatus: 'allowed' },
      })
    })
    await nativeGateway.invoke('capture_start', { meetingId: current.id })
    await meeting.refetch()
  }
  const results = await nativeGateway.invoke<Array<{ actionId: MeetingActionId; registered: boolean; conflictWith?: string }>>('hotkeys_register', {
    bindings: defaultMeetingBindings(),
  })
  for (const result of results) keyboard.registrationResult(result.actionId, result)
  unlistenHotkey = await listen<{ actionId: MeetingActionId }>('hotkey://action', async ({ payload }) => {
    if (payload.actionId === 'overlay.toggle') await showOverlay()
    if (payload.actionId === 'meeting.stop') await meetingGateway.stop(meetingId.value, 'user')
    if (payload.actionId === 'meeting.emergency-stop') await meetingGateway.emergencyStop()
    if (payload.actionId === 'context.reset') resetOpen.value = true
    await meeting.refetch()
  })
})
onBeforeUnmount(() => {
  unlistenHotkey?.()
  unlistenArtifact?.()
  if ('__TAURI_INTERNALS__' in window) void nativeGateway.invoke('hotkeys_unregister_all')
})
</script>

<template>
  <main class="min-h-screen bg-(--background) px-6 py-10 text-(--foreground)">
    <section class="mx-auto grid w-full max-w-4xl gap-7">
      <p
        v-if="meeting.isPending.value"
        class="text-(--muted-foreground)"
      >
        {{ t('common.loading') }}
      </p>
      <p
        v-else-if="meeting.isError.value"
        role="alert"
        class="text-(--danger)"
      >
        {{ t('meeting.active.loadError') }}
      </p>
      <template v-else-if="meeting.data.value">
        <header class="flex flex-wrap items-start justify-between gap-4">
          <div class="grid gap-1">
            <p class="text-sm font-medium text-(--accent)">
              {{ t('meeting.active.eyebrow') }}
            </p>
            <h1 class="text-3xl font-semibold tracking-tight">
              {{ meeting.data.value.title }}
            </h1>
            <p class="text-sm text-(--muted-foreground)">
              {{ t('meeting.active.description') }}
            </p>
          </div>
          <Button
            variant="secondary"
            @click="showOverlay"
          >
            <PanelTopOpen
              :size="16"
              aria-hidden="true"
            />{{ t('meeting.active.showOverlay') }}
          </Button>
        </header>
        <MeetingControls :meeting="meeting.data.value" />
        <section
          v-if="hotkeyConflicts.length"
          role="status"
          class="rounded-(--radius-md) border border-(--border) bg-(--surface) p-4 text-sm"
        >
          <p class="font-medium">
            {{ t('meeting.active.hotkeyConflict') }}
          </p>
          <ul class="mt-2 grid gap-1 text-(--muted-foreground)">
            <li
              v-for="[action, binding] in hotkeyConflicts"
              :key="action"
            >
              {{ t('meeting.active.hotkeyConflictItem', { action, shortcut: binding.accelerator }) }}
            </li>
          </ul>
        </section>
        <section class="grid gap-3 rounded-(--radius-lg) border border-(--border) bg-(--surface) p-5">
          <h2 class="font-semibold">
            {{ t('meeting.active.localRecording') }}
          </h2>
          <p class="text-sm text-(--muted-foreground)">
            {{ t('meeting.active.localRecordingDescription') }}
          </p>
          <span class="inline-flex items-center gap-2 text-sm"><ExternalLink
            :size="16"
            aria-hidden="true"
          />{{ t('meeting.active.overlayHint') }}</span>
        </section>
      </template>
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
        <Button @click="meetingGateway.resetContext(meetingId).then(() => { resetOpen = false })">
          {{ t('overlay.reset.confirm') }}
        </Button>
      </template>
    </Dialog>
  </main>
</template>
