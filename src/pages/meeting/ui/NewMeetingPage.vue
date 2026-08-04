<script setup lang="ts">
import { Mic, Monitor, Play, SlidersHorizontal } from '@lucide/vue'
import { useMutation, useQuery } from '@tanstack/vue-query'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

import { meetingGateway, type DisplayDescriptor } from '@/entities/meeting'
import { productClient, profileQuery, profilesQuery, type ModelCatalogEntry, type ProfileSummary } from '@/entities/interview-profile'
import { queryKeys } from '@/shared/api/query-keys'
import { sendMeetingEnvelope } from '@/app/providers/meeting-socket'
import Button from '@/shared/ui/button/Button.vue'
import { evaluateMeetingReadiness } from '../model/meeting-readiness'

const { t } = useI18n()
const router = useRouter()
const selectedProfileId = ref('')
const selectedDisplayId = ref<number | null>(null)
const title = ref('')
const soundThreshold = ref(0.18)
const profiles = useQuery(profilesQuery())
const selectedProfileDetails = useQuery(computed(() => profileQuery(selectedProfileId.value)))
const models = useQuery<ModelCatalogEntry[]>({ queryKey: queryKeys.models(), queryFn: () => productClient.listModels() })
const displays = useQuery<DisplayDescriptor[]>({ queryKey: ['displays'], queryFn: meetingGateway.listDisplays })
const captureConfigurationId = computed(() => selectedDisplayId.value === null ? '' : `display-${selectedDisplayId.value}-vad-${soundThreshold.value.toFixed(2)}`)
const gate = useQuery(computed(() => ({
  queryKey: ['run-gate', 'default-meeting-policy', captureConfigurationId.value],
  queryFn: () => meetingGateway.evaluateGate({
    launchPolicyId: 'default-meeting-policy',
    requestedMode: 'standard_lab',
    captureConfigurationId: captureConfigurationId.value,
  }),
  enabled: Boolean(selectedProfileId.value && captureConfigurationId.value),
})))

watch(() => profiles.data.value, (items) => {
  if (!selectedProfileId.value) {
    selectedProfileId.value = items?.find((profile) => profile.status === 'ready')?.id
      ?? items?.find((profile) => profile.status !== 'archived')?.id
      ?? ''
  }
}, { immediate: true })
watch(() => displays.data.value, (items) => {
  if (selectedDisplayId.value === null) selectedDisplayId.value = items?.find((display) => display.isPrimary)?.displayId ?? items?.[0]?.displayId ?? null
}, { immediate: true })

const selectedProfile = computed<ProfileSummary | undefined>(() => profiles.data.value?.find((profile) => profile.id === selectedProfileId.value))
const availableModelIds = computed(() => new Set(models.data.value?.filter((model) => model.availability === 'available').map((model) => model.id) ?? []))
const readiness = computed(() => evaluateMeetingReadiness({
  profileReady: selectedProfile.value?.status === 'ready',
  responseModelAvailable: availableModelIds.value.has(selectedProfileDetails.data.value?.modelConfiguration?.responseModelId ?? ''),
  transcriptionModelAvailable: availableModelIds.value.has(selectedProfileDetails.data.value?.modelConfiguration?.transcriptionModelId ?? ''),
  translationSupported: true,
  displaySelected: selectedDisplayId.value !== null,
  permissionsGranted: true,
  runGateAllowed: gate.data.value?.allowed === true,
}))
const setupPending = computed(() => Boolean(
  profiles.isPending.value
  || models.isPending.value
  || displays.isPending.value
  || (selectedProfileId.value && selectedProfileDetails.isPending.value)
  || (gate.isEnabled.value && gate.isPending.value),
))
const canConfigureProfile = computed(() => Boolean(
  selectedProfile.value
  && readiness.value.reasons.some((reason) => ['profile', 'response_model', 'transcription_model'].includes(reason)),
))

const start = useMutation({
  mutationFn: () => {
    if (!selectedProfile.value || selectedDisplayId.value === null) throw new Error('Meeting configuration incomplete')
    return meetingGateway.start({
      launchPolicyId: 'default-meeting-policy',
      profileId: selectedProfile.value.id,
      profileRevision: selectedProfileDetails.data.value?.revision ?? selectedProfile.value.revision,
      captureConfigurationId: captureConfigurationId.value,
      mode: 'standard_lab',
      title: title.value.trim() || selectedProfile.value.name,
    })
  },
  onSuccess: (meeting) => {
    try {
      sendMeetingEnvelope({
        v: 1,
        id: crypto.randomUUID(),
        type: 'meeting.start',
        sentAt: new Date().toISOString(),
        meetingId: meeting.id,
        launchPolicyId: 'default-meeting-policy',
        payload: {
          profileId: meeting.profileId,
          profileRevision: meeting.profileRevision,
          captureConfigurationId: captureConfigurationId.value,
          mode: meeting.mode,
        },
      })
    } catch {
      // The native meeting is already running; transport recovery happens independently.
    }
    void router.push(`/meetings/${meeting.id}`)
  },
})

function primaryAction() {
  if (setupPending.value || start.isPending.value) return
  if (canConfigureProfile.value && selectedProfile.value) {
    return router.push({ path: `/profiles/${selectedProfile.value.id}`, query: { returnTo: 'meeting' } })
  }
  if (readiness.value.ready) start.mutate()
}
</script>

<template>
  <main class="min-h-screen bg-(--background) px-6 py-10 text-(--foreground)">
    <section class="mx-auto grid w-full max-w-3xl gap-7">
      <header class="grid gap-1">
        <p class="text-sm font-medium text-(--accent)">
          {{ t('meeting.new.eyebrow') }}
        </p>
        <h1 class="text-3xl font-semibold tracking-tight">
          {{ t('meeting.new.title') }}
        </h1>
        <p class="text-sm text-(--muted-foreground)">
          {{ t('meeting.new.description') }}
        </p>
      </header>

      <section class="grid gap-5 rounded-(--radius-lg) border border-(--border) bg-(--surface) p-5">
        <label class="grid gap-1 text-sm font-medium">
          {{ t('meeting.new.meetingTitle') }}
          <input
            v-model="title"
            class="h-10 rounded-(--radius-sm) border border-(--input) bg-(--surface-raised) px-3 font-normal"
            :placeholder="t('meeting.new.meetingTitlePlaceholder')"
          >
        </label>
        <label class="grid gap-1 text-sm font-medium">
          {{ t('meeting.new.profile') }}
          <select
            v-model="selectedProfileId"
            class="h-10 rounded-(--radius-sm) border border-(--input) bg-(--surface-raised) px-3 font-normal"
          >
            <option
              value=""
              disabled
              hidden
            >
              {{ t('meeting.new.selectProfile') }}
            </option>
            <option
              v-for="profile in profiles.data.value"
              :key="profile.id"
              :value="profile.id"
              :disabled="profile.status === 'archived'"
            >
              {{ profile.name }}{{ profile.status !== 'ready' ? ` — ${t('profiles.status.draft')}` : '' }}
            </option>
          </select>
        </label>
        <label class="grid gap-1 text-sm font-medium">
          <span class="inline-flex items-center gap-2"><Monitor
            :size="16"
            aria-hidden="true"
          />{{ t('meeting.new.display') }}</span>
          <select
            v-model.number="selectedDisplayId"
            class="h-10 rounded-(--radius-sm) border border-(--input) bg-(--surface-raised) px-3 font-normal"
          >
            <option
              :value="null"
              disabled
              hidden
            >
              {{ t('meeting.new.selectDisplay') }}
            </option>
            <option
              v-for="display in displays.data.value"
              :key="display.displayId"
              :value="display.displayId"
            >
              {{ t('meeting.new.displayName', { number: display.label }) }} · {{ display.width }}×{{ display.height }}
            </option>
          </select>
        </label>
        <label class="grid gap-2 text-sm font-medium">
          <span class="flex items-center justify-between gap-4">
            <span class="inline-flex items-center gap-2"><Mic
              :size="16"
              aria-hidden="true"
            />{{ t('meeting.new.soundThreshold') }}</span>
            <span class="font-normal text-(--muted-foreground)">{{ Math.round(soundThreshold * 100) }}%</span>
          </span>
          <input
            v-model.number="soundThreshold"
            type="range"
            min="0.05"
            max="0.8"
            step="0.01"
            :aria-label="t('meeting.new.soundThreshold')"
          >
          <span class="text-xs font-normal text-(--muted-foreground)">{{ t('meeting.new.soundHint') }}</span>
        </label>
      </section>

      <section class="grid gap-3 rounded-(--radius-md) border border-(--border) px-4 py-3 sm:grid-cols-[1fr_auto] sm:items-center">
        <div class="grid gap-2 text-sm">
          <div class="flex items-center gap-2">
            <SlidersHorizontal
              :size="16"
              aria-hidden="true"
            />
            <span>{{ readiness.ready ? t('meeting.new.ready') : t('meeting.new.blocked', { count: readiness.reasons.length }) }}</span>
          </div>
          <ul
            v-if="!readiness.ready"
            class="flex flex-wrap gap-x-3 gap-y-1 pl-6 text-xs text-(--muted-foreground)"
          >
            <li
              v-for="reason in readiness.reasons"
              :key="reason"
            >
              {{ t(`meeting.new.reason.${reason}`) }}
            </li>
          </ul>
        </div>
        <Button
          size="lg"
          :disabled="setupPending || (!readiness.ready && !canConfigureProfile) || start.isPending.value"
          @click="primaryAction"
        >
          <Play
            :size="16"
            aria-hidden="true"
          />
          {{ start.isPending.value ? t('meeting.new.starting') : setupPending ? t('meeting.new.checking') : canConfigureProfile ? t('meeting.new.configureProfile') : t('meeting.new.start') }}
        </Button>
      </section>
      <p
        v-if="start.isError.value"
        role="alert"
        class="text-sm text-(--danger)"
      >
        {{ t('meeting.new.startError') }}
      </p>
    </section>
  </main>
</template>
