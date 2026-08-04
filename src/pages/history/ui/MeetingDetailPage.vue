<script setup lang="ts">
import { FileAudio, Image, Trash2 } from '@lucide/vue'
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'

import { historyGateway, historyMeetingQuery, invalidateHistory } from '@/entities/meeting'
import Button from '@/shared/ui/button/Button.vue'
import Dialog from '@/shared/ui/dialog/Dialog.vue'

const { t, d } = useI18n()
const route = useRoute()
const router = useRouter()
const queryClient = useQueryClient()
const meetingId = computed(() => String(route.params.meetingId ?? ''))
const meeting = useQuery(computed(() => historyMeetingQuery(meetingId.value)))
const confirmDelete = ref(false)
const remove = useMutation({
  mutationFn: () => historyGateway.deleteContent(meetingId.value),
  onSuccess: async () => {
    confirmDelete.value = false
    await invalidateHistory(queryClient, meetingId.value)
    await router.replace('/history')
  },
})
</script>

<template>
  <main class="min-h-screen bg-(--background) px-6 py-10 text-(--foreground)">
    <section class="mx-auto grid w-full max-w-5xl gap-6">
      <p
        v-if="meeting.isPending.value"
        class="text-(--muted-foreground)"
      >
        {{ t('common.loading') }}
      </p>
      <div
        v-else-if="meeting.isError.value"
        role="alert"
        class="text-(--danger)"
      >
        {{ t('history.detail.loadError') }}
      </div>
      <template v-else-if="meeting.data.value">
        <header class="flex flex-wrap items-start justify-between gap-4">
          <div class="grid gap-1">
            <Button
              class="w-fit px-0"
              variant="ghost"
              @click="router.push('/history')"
            >
              {{ t('history.detail.back') }}
            </Button>
            <h1 class="text-3xl font-semibold tracking-tight">
              {{ meeting.data.value.title }}
            </h1>
            <p class="text-sm text-(--muted-foreground)">
              {{ meeting.data.value.profileName }} · {{ d(new Date(meeting.data.value.createdAtMs), 'short') }}
            </p>
          </div>
          <Button
            variant="secondary"
            @click="confirmDelete = true"
          >
            <Trash2
              :size="16"
              aria-hidden="true"
            />{{ t('history.detail.delete') }}
          </Button>
        </header>
        <p class="rounded-(--radius-md) border border-(--border) bg-(--surface) p-4 text-sm">
          {{ t('history.detail.retainedUntil', { date: d(new Date(meeting.data.value.retentionExpiresAtMs), 'short') }) }}
        </p>
        <section class="grid gap-2 rounded-(--radius-lg) border border-(--border) bg-(--surface) p-5">
          <h2 class="font-semibold">
            {{ t('history.detail.transcript') }}
          </h2>
          <p
            v-if="meeting.data.value.transcript.length === 0"
            class="text-sm text-(--muted-foreground)"
          >
            {{ t('history.detail.noTranscript') }}
          </p>
          <ol
            v-else
            class="grid gap-2"
          >
            <li
              v-for="segment in meeting.data.value.transcript"
              :key="segment.id"
              class="text-sm"
            >
              <strong>{{ segment.speaker }}:</strong> {{ segment.text }}
            </li>
          </ol>
        </section>
        <section class="grid gap-3 rounded-(--radius-lg) border border-(--border) bg-(--surface) p-5">
          <h2 class="font-semibold">
            {{ t('history.detail.chats') }}
          </h2>
          <p
            v-if="meeting.data.value.chats.every((chat) => chat.messages.length === 0)"
            class="text-sm text-(--muted-foreground)"
          >
            {{ t('history.detail.noChats') }}
          </p>
          <article
            v-for="chat in meeting.data.value.chats"
            :key="chat.kind"
            class="grid gap-2"
          >
            <h3 class="text-sm font-semibold">
              {{ t(`history.detail.thread.${chat.kind}`) }}
            </h3>
            <div
              v-for="message in chat.messages"
              :key="message.id"
              class="grid gap-1 border-l-2 border-(--border) pl-3 text-sm"
            >
              <strong>{{ t(`history.detail.role.${message.role}`) }}</strong>
              <p>{{ message.content }}</p>
            </div>
          </article>
        </section>
        <section class="grid gap-3 rounded-(--radius-lg) border border-(--border) bg-(--surface) p-5">
          <h2 class="font-semibold">
            {{ t('history.detail.artifacts') }}
          </h2>
          <p
            v-if="meeting.data.value.artifacts.length === 0"
            class="text-sm text-(--muted-foreground)"
          >
            {{ t('history.detail.noArtifacts') }}
          </p>
          <ul class="grid gap-2 sm:grid-cols-2">
            <li
              v-for="artifact in meeting.data.value.artifacts"
              :key="artifact.id"
              class="flex items-center gap-2 rounded-(--radius-md) bg-(--muted) p-3 text-sm"
            >
              <FileAudio
                v-if="artifact.kind.includes('recording') || artifact.kind.includes('audio')"
                :size="16"
                aria-hidden="true"
              /><Image
                v-else
                :size="16"
                aria-hidden="true"
              />{{ t(artifact.kind.includes('recording') || artifact.kind.includes('audio') ? 'history.detail.recording' : 'history.detail.screenshot') }}
            </li>
          </ul>
        </section>
      </template>
    </section>
    <Dialog
      v-model:open="confirmDelete"
      :title="t('history.delete.title')"
      :description="t('history.delete.description')"
    >
      <template #footer>
        <Button
          variant="secondary"
          @click="confirmDelete = false"
        >
          {{ t('common.cancel') }}
        </Button>
        <Button
          :disabled="remove.isPending.value"
          @click="remove.mutate()"
        >
          {{ t('history.delete.confirm') }}
        </Button>
      </template>
    </Dialog>
  </main>
</template>
