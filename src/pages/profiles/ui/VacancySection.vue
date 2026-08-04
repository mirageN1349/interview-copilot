<script setup lang="ts">
import { Check, Link, LoaderCircle, RefreshCw } from '@lucide/vue'
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { productClient, type VacancySource } from '@/entities/interview-profile'
import Button from '@/shared/ui/button/Button.vue'

const props = defineProps<{ vacancy: VacancySource | null }>()
const emit = defineEmits<{ update: [value: VacancySource] }>()
const { t } = useI18n()
const sourceValue = ref('')
const pending = ref(false)
const error = ref<string | null>(null)
const draft = ref<VacancySource | null>(null)

watch(() => props.vacancy, (vacancy) => {
  draft.value = vacancy ? {
    ...vacancy,
    responsibilities: [...vacancy.responsibilities],
    requirements: [...vacancy.requirements],
    provenance: { ...vacancy.provenance },
  } : null
  sourceValue.value = vacancy?.sourceValue ?? ''
}, { immediate: true })

const sourceKind = computed<'url' | 'text'>(() => /^https?:\/\//i.test(sourceValue.value.trim()) ? 'url' : 'text')

async function extract() {
  const value = sourceValue.value.trim()
  if (!value) return
  pending.value = true
  error.value = null
  try {
    const result = await productClient.parseVacancy(sourceKind.value === 'url'
      ? { kind: 'url', url: value }
      : { kind: 'text', text: value })
    draft.value = {
      id: props.vacancy?.id ?? 'pending-vacancy',
      sourceKind: sourceKind.value === 'url' ? 'url' : 'pasted_text',
      sourceValue: value,
      roleTitle: result.title,
      companyContext: result.company,
      responsibilities: result.responsibilities,
      requirements: result.requirements,
      reviewStatus: 'needs_review',
      provenance: {
        fixtureId: result.provenance.fixtureId,
        extractionModelId: result.provenance.extractionModelId,
        extractedAtMs: Date.parse(result.provenance.extractedAt),
      },
    }
    emit('update', draft.value)
  } catch {
    error.value = 'profiles.vacancy.unsupported'
  } finally {
    pending.value = false
  }
}

function patchDraft(patch: Partial<VacancySource>) {
  if (!draft.value) return
  draft.value = { ...draft.value, ...patch }
  emit('update', draft.value)
}

function confirm() {
  patchDraft({ reviewStatus: 'confirmed' })
}
</script>

<template>
  <section class="grid gap-5 rounded-(--radius-lg) border border-(--border) bg-(--surface) p-5 sm:p-6">
    <header class="grid gap-1">
      <h2 class="text-base font-semibold">
        {{ t('profiles.vacancy.title') }}
      </h2>
      <p class="text-sm text-(--muted-foreground)">
        {{ t('profiles.vacancy.description') }}
      </p>
    </header>

    <div class="grid gap-2">
      <label
        for="vacancy-source"
        class="text-sm font-medium"
      >{{ t('profiles.vacancy.source') }}</label>
      <div class="grid gap-3 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end">
        <textarea
          id="vacancy-source"
          v-model="sourceValue"
          class="min-h-24 resize-y rounded-(--radius-md) border border-(--input) bg-(--surface-raised) px-4 py-3 outline-none transition-[border-color,box-shadow] focus:border-(--ring) focus:ring-3 focus:ring-(--ring)/15"
          :placeholder="t('profiles.vacancy.placeholder')"
          maxlength="20000"
        />
        <Button
          class="!min-h-12 !rounded-full !px-5"
          :disabled="pending || !sourceValue.trim()"
          @click="extract"
        >
          <LoaderCircle
            v-if="pending"
            :size="16"
            class="animate-spin"
            aria-hidden="true"
          />
          <Link
            v-else
            :size="16"
            aria-hidden="true"
          />
          {{ t('profiles.vacancy.extract') }}
        </Button>
      </div>
      <p
        v-if="error"
        role="alert"
        class="text-sm text-(--danger)"
      >
        {{ t(error) }}
      </p>
    </div>

    <div
      v-if="draft"
      class="grid gap-4 border-t border-(--border) pt-4"
    >
      <div class="grid gap-2 sm:grid-cols-2">
        <label class="grid gap-1 text-sm font-medium">
          {{ t('profiles.vacancy.role') }}
          <input
            class="h-12 rounded-(--radius-md) border border-(--input) bg-(--surface-raised) px-4 font-normal outline-none focus:border-(--ring) focus:ring-3 focus:ring-(--ring)/15"
            :value="draft.roleTitle"
            @input="patchDraft({ roleTitle: ($event.target as HTMLInputElement).value })"
          >
        </label>
        <label class="grid gap-1 text-sm font-medium">
          {{ t('profiles.vacancy.company') }}
          <input
            class="h-12 rounded-(--radius-md) border border-(--input) bg-(--surface-raised) px-4 font-normal outline-none focus:border-(--ring) focus:ring-3 focus:ring-(--ring)/15"
            :value="draft.companyContext"
            @input="patchDraft({ companyContext: ($event.target as HTMLInputElement).value })"
          >
        </label>
      </div>
      <div class="grid gap-2 sm:grid-cols-2">
        <label class="grid gap-1 text-sm font-medium">
          {{ t('profiles.vacancy.responsibilities') }}
          <textarea
            class="min-h-32 rounded-(--radius-md) border border-(--input) bg-(--surface-raised) px-4 py-3 font-normal outline-none focus:border-(--ring) focus:ring-3 focus:ring-(--ring)/15"
            :value="draft.responsibilities.join('\n')"
            @input="patchDraft({ responsibilities: ($event.target as HTMLTextAreaElement).value.split('\n').filter(Boolean) })"
          />
        </label>
        <label class="grid gap-1 text-sm font-medium">
          {{ t('profiles.vacancy.requirements') }}
          <textarea
            class="min-h-32 rounded-(--radius-md) border border-(--input) bg-(--surface-raised) px-4 py-3 font-normal outline-none focus:border-(--ring) focus:ring-3 focus:ring-(--ring)/15"
            :value="draft.requirements.join('\n')"
            @input="patchDraft({ requirements: ($event.target as HTMLTextAreaElement).value.split('\n').filter(Boolean) })"
          />
        </label>
      </div>
      <div class="flex items-center justify-between gap-3">
        <p class="text-xs text-(--muted-foreground)">
          {{ t('profiles.vacancy.provenance', { source: draft.provenance.fixtureId }) }}
        </p>
        <span
          v-if="draft.reviewStatus === 'confirmed'"
          class="inline-flex items-center gap-1 text-sm text-emerald-700 dark:text-emerald-400"
        ><Check
          :size="16"
          aria-hidden="true"
        />{{ t('profiles.vacancy.confirmed') }}</span>
        <Button
          v-else
          class="!min-h-11 !rounded-full !px-5"
          variant="secondary"
          @click="confirm"
        >
          <RefreshCw
            :size="16"
            aria-hidden="true"
          />{{ t('profiles.vacancy.confirm') }}
        </Button>
      </div>
    </div>
  </section>
</template>
