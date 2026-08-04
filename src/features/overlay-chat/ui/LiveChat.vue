<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import type { AssistantMessage, RecognizedQuestion } from '../model/meeting-reducer'
import RecognizedQuestionView from './RecognizedQuestion.vue'
import MessageContent from '@/shared/ui/code/MessageContent.vue'

const props = defineProps<{
  messages: AssistantMessage[]
  question: RecognizedQuestion | null
  timeline: Array<
    | { id: string; role: 'interviewer'; question: RecognizedQuestion }
    | { id: string; role: 'assistant' }
  >
}>()
defineEmits<{ confirmQuestion: [segmentId: string] }>()
const { t } = useI18n()
const messagesById = computed(() => new Map(props.messages.map((message) => [message.id, message])))
const questionFor = (entry: Extract<(typeof props.timeline)[number], { role: 'interviewer' }>) => ({
  ...entry.question,
  requiresConfirmation: props.question?.segmentId === entry.id && props.question.requiresConfirmation,
})
</script>

<template>
  <section
    aria-labelledby="overlay-live-tab"
    aria-live="polite"
    class="min-h-0 overflow-y-auto"
  >
    <div
      v-if="timeline.length"
      class="grid gap-3 px-4 pb-4 pt-[6.25rem]"
    >
      <template
        v-for="entry in timeline"
        :key="`${entry.role}-${entry.id}`"
      >
        <RecognizedQuestionView
          v-if="entry.role === 'interviewer'"
          data-timeline-role="interviewer"
          :question="questionFor(entry)"
          @confirm="$emit('confirmQuestion', $event)"
        />
        <div
          v-else-if="messagesById.get(entry.id)"
          data-timeline-role="assistant"
          class="grid justify-items-start gap-1"
        >
          <span class="px-1 text-[0.625rem] font-medium text-(--muted-foreground)">{{ t('overlay.live.assistant') }}</span>
          <article
            class="assistant-bubble max-w-[88%] rounded-[1.15rem] rounded-bl-md px-3.5 py-2.5 text-sm leading-5"
            :data-status="messagesById.get(entry.id)!.status"
          >
            <MessageContent :content="messagesById.get(entry.id)!.content" />
            <p
              v-if="messagesById.get(entry.id)!.profileSourceIds.length"
              class="mt-2 text-xs text-(--muted-foreground)"
            >
              {{ t('overlay.sources', { count: messagesById.get(entry.id)!.profileSourceIds.length }) }}
            </p>
          </article>
        </div>
      </template>
    </div>
    <p
      v-else
      class="px-5 pb-5 pt-[6.5rem] text-sm text-(--muted-foreground)"
    >
      {{ t('overlay.live.empty') }}
    </p>
  </section>
</template>

<style scoped>
.assistant-bubble {
  background: color-mix(in oklch, var(--surface-raised) 48%, transparent);
  box-shadow: inset 0 1px 0 color-mix(in oklch, var(--surface-raised) 30%, transparent);
}

.assistant-bubble[data-status="streaming"]::after {
  content: "";
  display: inline-block;
  width: 0.35rem;
  height: 0.9rem;
  margin-left: 0.2rem;
  border-radius: 999px;
  background: var(--accent);
  vertical-align: -0.1rem;
  animation: cursor-pulse 0.8s ease-in-out infinite alternate;
}

@keyframes cursor-pulse {
  to { opacity: 0.2; }
}
</style>
