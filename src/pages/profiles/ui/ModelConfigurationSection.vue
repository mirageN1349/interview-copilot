<script setup lang="ts">
import { useQuery } from '@tanstack/vue-query'
import { computed, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { productClient, type ModelConfiguration } from '@/entities/interview-profile'
import { queryKeys } from '@/shared/api/query-keys'

const props = defineProps<{ configuration: ModelConfiguration | null }>()
const emit = defineEmits<{ update: [value: ModelConfiguration] }>()
const { t } = useI18n()
const models = useQuery({ queryKey: queryKeys.models(), queryFn: () => productClient.listModels() })

const responseModels = computed(() => models.data.value?.filter((model) => model.kind === 'response') ?? [])
const transcriptionModels = computed(() => models.data.value?.filter((model) => model.kind === 'transcription') ?? [])

const value = computed<ModelConfiguration>(() => props.configuration ?? {
  id: 'pending-model-configuration',
  responseModelId: '',
  transcriptionModelId: '',
  translationLanguage: 'none',
  answerDepth: 'balanced',
  questionConfidenceThreshold: 0.72,
  processingBoundaryId: 'mock-local-boundary',
})

function patch(configuration: Partial<ModelConfiguration>) {
  emit('update', { ...value.value, ...configuration })
}

watch(() => models.data.value, (catalog) => {
  if (!catalog) return
  const responseModelId = value.value.responseModelId || catalog.find((model) => model.kind === 'response' && model.availability === 'available')?.id
  const transcriptionModelId = value.value.transcriptionModelId || catalog.find((model) => model.kind === 'transcription' && model.availability === 'available')?.id
  if (!responseModelId || !transcriptionModelId) return
  if (responseModelId !== value.value.responseModelId || transcriptionModelId !== value.value.transcriptionModelId) {
    patch({ responseModelId, transcriptionModelId })
  }
}, { immediate: true })
</script>

<template>
  <section class="grid gap-5 rounded-(--radius-lg) border border-(--border) bg-(--surface) p-5 sm:p-6">
    <header class="grid gap-1">
      <h2 class="text-base font-semibold">
        {{ t('profiles.models.title') }}
      </h2>
      <p class="text-sm text-(--muted-foreground)">
        {{ t('profiles.models.description') }}
      </p>
    </header>
    <p
      v-if="models.isPending.value"
      class="text-sm text-(--muted-foreground)"
    >
      {{ t('common.loading') }}
    </p>
    <p
      v-else-if="models.isError.value"
      role="alert"
      class="text-sm text-(--danger)"
    >
      {{ t('profiles.models.loadError') }}
    </p>
    <div
      v-else
      class="grid gap-4 sm:grid-cols-2"
    >
      <label class="grid gap-1 text-sm font-medium">
        {{ t('profiles.models.response') }}
        <select
          class="h-12 rounded-(--radius-md) border border-(--input) bg-(--surface-raised) px-4 font-normal"
          :value="value.responseModelId"
          @change="patch({ responseModelId: ($event.target as HTMLSelectElement).value })"
        >
          <option
            value=""
            disabled
          >{{ t('profiles.models.select') }}</option>
          <option
            v-for="model in responseModels"
            :key="model.id"
            :value="model.id"
            :disabled="model.availability !== 'available'"
          >
            {{ model.name }}{{ model.availability !== 'available' ? ` — ${t('profiles.models.unavailable')}` : '' }}
          </option>
        </select>
      </label>
      <label class="grid gap-1 text-sm font-medium">
        {{ t('profiles.models.transcription') }}
        <select
          class="h-12 rounded-(--radius-md) border border-(--input) bg-(--surface-raised) px-4 font-normal"
          :value="value.transcriptionModelId"
          @change="patch({ transcriptionModelId: ($event.target as HTMLSelectElement).value })"
        >
          <option
            value=""
            disabled
          >{{ t('profiles.models.select') }}</option>
          <option
            v-for="model in transcriptionModels"
            :key="model.id"
            :value="model.id"
            :disabled="model.availability !== 'available'"
          >
            {{ model.name }}{{ model.availability !== 'available' ? ` — ${t('profiles.models.unavailable')}` : '' }}
          </option>
        </select>
      </label>
      <label class="grid gap-1 text-sm font-medium">
        {{ t('profiles.models.translation') }}
        <select
          class="h-12 rounded-(--radius-md) border border-(--input) bg-(--surface-raised) px-4 font-normal"
          :value="value.translationLanguage"
          @change="patch({ translationLanguage: ($event.target as HTMLSelectElement).value })"
        >
          <option value="none">{{ t('profiles.models.noTranslation') }}</option>
          <option value="ru">{{ t('profiles.models.language.ru') }}</option>
          <option value="en">{{ t('profiles.models.language.en') }}</option>
        </select>
      </label>
      <label class="grid gap-1 text-sm font-medium">
        {{ t('profiles.models.depth') }}
        <select
          class="h-12 rounded-(--radius-md) border border-(--input) bg-(--surface-raised) px-4 font-normal"
          :value="value.answerDepth"
          @change="patch({ answerDepth: ($event.target as HTMLSelectElement).value as ModelConfiguration['answerDepth'] })"
        >
          <option value="brief">{{ t('profiles.models.depthOption.brief') }}</option>
          <option value="balanced">{{ t('profiles.models.depthOption.balanced') }}</option>
          <option value="detailed">{{ t('profiles.models.depthOption.detailed') }}</option>
        </select>
      </label>
    </div>
  </section>
</template>
