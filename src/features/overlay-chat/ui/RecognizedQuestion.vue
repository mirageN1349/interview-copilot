<script setup lang="ts">
import { AudioWaveform, Check } from '@lucide/vue'
import { useI18n } from 'vue-i18n'

import type { RecognizedQuestion } from '../model/meeting-reducer'
import Button from '@/shared/ui/button/Button.vue'

defineProps<{ question: RecognizedQuestion }>()
defineEmits<{ confirm: [segmentId: string] }>()
const { t, n } = useI18n()
</script>

<template>
  <section class="grid justify-items-end gap-1.5">
    <span class="px-1 text-[0.625rem] font-medium text-(--muted-foreground)">{{ t('overlay.live.interviewer') }}</span>
    <div class="question-bubble flex max-w-[88%] items-start gap-2 rounded-[1.15rem] rounded-br-md px-3.5 py-2.5">
      <AudioWaveform
        :size="16"
        aria-hidden="true"
        class="mt-0.5 shrink-0"
      />
      <p class="m-0 flex-1 text-sm leading-5">
        {{ question.text }}
      </p>
      <span class="rounded-full bg-[color-mix(in_oklch,var(--foreground)_10%,transparent)] px-1.5 py-0.5 text-[0.625rem]">{{ n(question.confidence, 'percent') }}</span>
    </div>
    <div
      v-if="question.requiresConfirmation"
      class="flex items-center justify-between gap-3"
    >
      <span class="text-xs text-(--muted-foreground)">{{ t('overlay.question.confirmHint') }}</span>
      <Button
        size="sm"
        variant="secondary"
        @click="$emit('confirm', question.segmentId)"
      >
        <Check
          :size="14"
          aria-hidden="true"
        />{{ t('overlay.question.confirm') }}
      </Button>
    </div>
  </section>
</template>

<style scoped>
.question-bubble {
  background: color-mix(in oklch, var(--accent) 78%, transparent);
  color: var(--accent-foreground);
  box-shadow: inset 0 1px 0 color-mix(in oklch, var(--accent-foreground) 20%, transparent);
}
</style>
