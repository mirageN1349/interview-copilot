<script setup lang="ts">
import { CircleStop, Radio, ShieldAlert } from '@lucide/vue'
import { useMutation, useQueryClient } from '@tanstack/vue-query'
import { useI18n } from 'vue-i18n'

import { meetingGateway, updateMeetingCache, type MeetingRuntimeSummary } from '@/entities/meeting'
import { sendMeetingEnvelope } from '@/app/providers/meeting-socket'
import Button from '@/shared/ui/button/Button.vue'

const props = defineProps<{ meeting: MeetingRuntimeSummary }>()
const { t } = useI18n()
const queryClient = useQueryClient()

const stop = useMutation({
  mutationFn: () => meetingGateway.stop(props.meeting.id, 'user'),
  onSuccess: (meeting) => {
    sendMeetingEnvelope({ v: 1, id: crypto.randomUUID(), type: 'meeting.stop', sentAt: new Date().toISOString(), meetingId: meeting.id, launchPolicyId: meeting.launchPolicyId, payload: { reason: 'user' } })
    return updateMeetingCache(queryClient, meeting)
  },
})
const emergency = useMutation({
  mutationFn: meetingGateway.emergencyStop,
  onSuccess: (meeting) => {
    if (!meeting) return
    sendMeetingEnvelope({ v: 1, id: crypto.randomUUID(), type: 'meeting.stop', sentAt: new Date().toISOString(), meetingId: meeting.id, launchPolicyId: meeting.launchPolicyId, payload: { reason: 'kill_switch' } })
    return updateMeetingCache(queryClient, meeting)
  },
})
</script>

<template>
  <section class="flex flex-wrap items-center justify-between gap-3 rounded-(--radius-md) border border-(--border) bg-(--surface) p-3">
    <div class="flex items-center gap-2 text-sm">
      <Radio
        :size="16"
        aria-hidden="true"
      />
      <span>{{ t(`meeting.capture.${meeting.capturePhase}`) }}</span>
      <span class="text-(--muted-foreground)">· {{ t(`meeting.status.${meeting.status}`) }}</span>
    </div>
    <div class="flex gap-2">
      <Button
        variant="secondary"
        :disabled="stop.isPending.value || meeting.status !== 'running'"
        @click="stop.mutate()"
      >
        <CircleStop
          :size="16"
          aria-hidden="true"
        />
        {{ t('meeting.controls.stop') }}
      </Button>
      <Button
        variant="danger"
        :disabled="emergency.isPending.value"
        @click="emergency.mutate()"
      >
        <ShieldAlert
          :size="16"
          aria-hidden="true"
        />
        {{ t('meeting.controls.emergency') }}
      </Button>
    </div>
  </section>
</template>
