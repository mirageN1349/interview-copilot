<script setup lang="ts">
import { ArrowLeft, CheckCircle2, Save, Video } from '@lucide/vue'
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import { computed, onBeforeUnmount, ref, toRaw, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'

import {
  invalidateProfileQueries,
  productClient,
  profileGateway,
  profileQuery,
  type ModelCatalogEntry,
  type ModelConfiguration,
  type ProfileDetails,
  type VacancySource,
} from '@/entities/interview-profile'
import { queryKeys } from '@/shared/api/query-keys'
import Button from '@/shared/ui/button/Button.vue'
import { evaluateProfileReadiness } from '../model/profile-readiness'
import ModelConfigurationSection from './ModelConfigurationSection.vue'
import ProfileSourcesSection from './ProfileSourcesSection.vue'
import VacancySection from './VacancySection.vue'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const queryClient = useQueryClient()
const profileId = String(route.params.profileId ?? '')
const query = useQuery(profileQuery(profileId))
const models = useQuery<ModelCatalogEntry[]>({ queryKey: queryKeys.models(), queryFn: () => productClient.listModels() })
const draft = ref<ProfileDetails | null>(null)
const saved = ref(false)
const returnAfterSave = ref(false)
const persistedState = ref('')
let savedTimer: number | undefined

const availableModelIds = computed(() => new Set(models.data.value?.filter((model) => model.availability === 'available').map((model) => model.id) ?? []))
const readiness = computed(() => draft.value ? evaluateProfileReadiness(draft.value, availableModelIds.value) : { ready: false, reasons: [] })
const dirty = computed(() => Boolean(draft.value) && serializeDraft(draft.value) !== persistedState.value)

watch(() => query.data.value, (profile) => {
  if (profile && (!draft.value || !dirty.value)) hydrate(profile)
}, { immediate: true })

function buildSaveInput(profile: ProfileDetails) {
  return {
    id: profile.id,
    expectedRevision: profile.revision,
    name: profile.name,
    manualContext: profile.manualContext,
    vacancy: profile.vacancy ? omitId(profile.vacancy) : null,
    modelConfiguration: profile.modelConfiguration ? omitId(profile.modelConfiguration) : null,
  }
}

const save = useMutation({
  mutationFn: () => {
    if (!draft.value) throw new Error('Profile draft missing')
    return profileGateway.save(buildSaveInput(draft.value))
  },
  onSuccess: async (profile) => {
    hydrate(profile, true)
    await invalidateProfileQueries(queryClient, profile.id)
    if (returnAfterSave.value) {
      returnAfterSave.value = false
      await router.push('/meetings/new')
    }
  },
})

const sourceImport = useMutation({
  mutationFn: async (payload: { fixtureId: string; kind: 'resume' | 'project' }) => {
    if (!draft.value) throw new Error('Profile draft missing')
    let profile = draft.value
    if (dirty.value) {
      profile = await profileGateway.save(buildSaveInput(draft.value))
      hydrate(profile, true)
    }
    return profileGateway.importSource({ profileId: profile.id, expectedRevision: profile.revision, ...payload })
  },
  onSuccess: async (profile) => {
    hydrate(profile, true)
    await invalidateProfileQueries(queryClient, profile.id)
  },
})

function cloneProfile(profile: ProfileDetails): ProfileDetails {
  return structuredClone(toRaw(profile))
}

function serializeDraft(profile: ProfileDetails): string {
  return JSON.stringify(buildSaveInput(profile))
}

function hydrate(profile: ProfileDetails, showSaved = false) {
  draft.value = cloneProfile(profile)
  persistedState.value = serializeDraft(profile)
  if (!showSaved) return
  saved.value = true
  window.clearTimeout(savedTimer)
  savedTimer = window.setTimeout(() => { saved.value = false }, 1600)
}

onBeforeUnmount(() => window.clearTimeout(savedTimer))

function omitId<T extends { id: string }>(value: T): Omit<T, 'id'> {
  const result = { ...value } as T & { id?: string }
  delete result.id
  return result
}

function updateVacancy(vacancy: VacancySource) {
  if (draft.value) draft.value.vacancy = vacancy
}

function updateModels(configuration: ModelConfiguration) {
  if (draft.value) draft.value.modelConfiguration = configuration
}

function continueToMeeting() {
  if (!readiness.value.ready) return
  if (!dirty.value) {
    void router.push('/meetings/new')
    return
  }
  returnAfterSave.value = true
  save.mutate()
}
</script>

<template>
  <main class="min-h-screen bg-(--background) px-5 py-7 text-(--foreground) sm:px-8">
    <section class="mx-auto grid w-full max-w-5xl gap-5">
      <Button
        class="!w-fit !rounded-full !px-4"
        variant="secondary"
        @click="router.push('/profiles')"
      >
        <ArrowLeft
          :size="16"
          aria-hidden="true"
        />{{ t('profiles.back') }}
      </Button>

      <p
        v-if="query.isPending.value"
        class="p-6 text-(--muted-foreground)"
      >
        {{ t('common.loading') }}
      </p>
      <div
        v-else-if="query.isError.value"
        role="alert"
        class="rounded-(--radius-md) border border-(--danger) p-6 text-(--danger)"
      >
        {{ t('profiles.loadError') }}
      </div>

      <template v-else-if="draft">
        <header class="grid gap-5 rounded-(--radius-lg) border border-(--border) bg-(--surface) p-5 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end sm:p-6">
          <label class="grid gap-2 text-sm font-medium">
            {{ t('profiles.fields.name') }}
            <input
              v-model="draft.name"
              maxlength="100"
              class="h-13 min-w-0 rounded-(--radius-md) border border-(--input) bg-(--surface-raised) px-4 text-xl font-semibold outline-none transition-[border-color,box-shadow] focus:border-(--ring) focus:ring-3 focus:ring-(--ring)/15"
            >
          </label>
          <div class="flex flex-wrap items-center justify-between gap-3 sm:justify-end">
            <span
              class="inline-flex min-h-9 items-center rounded-full bg-(--muted) px-3 text-sm"
              :class="readiness.ready && !dirty ? 'text-emerald-700 dark:text-emerald-400' : 'text-(--muted-foreground)'"
            >
              {{ dirty ? t('profiles.unsaved') : readiness.ready ? t('profiles.readiness.ready') : t('profiles.readiness.draft', { count: readiness.reasons.length }) }}
            </span>
            <Button
              class="!min-h-12 !rounded-full !px-6"
              :disabled="save.isPending.value || sourceImport.isPending.value || !dirty"
              @click="save.mutate()"
            >
              <CheckCircle2
                v-if="saved && !dirty"
                :size="16"
                aria-hidden="true"
              />
              <Save
                v-else
                :size="16"
                aria-hidden="true"
              />
              {{ save.isPending.value ? t('common.saving') : dirty ? t('common.save') : t('common.saved') }}
            </Button>
            <Button
              v-if="route.query.returnTo === 'meeting'"
              size="lg"
              variant="secondary"
              :disabled="!readiness.ready || save.isPending.value || sourceImport.isPending.value"
              @click="continueToMeeting"
            >
              <Video
                :size="16"
                aria-hidden="true"
              />
              {{ dirty ? t('profiles.saveAndContinue') : t('profiles.continueToMeeting') }}
            </Button>
          </div>
        </header>

        <section
          v-if="!readiness.ready"
          class="rounded-(--radius-md) border border-(--border) bg-(--surface) px-5 py-4"
          aria-labelledby="profile-checklist-title"
        >
          <h2
            id="profile-checklist-title"
            class="text-sm font-semibold"
          >
            {{ t('profiles.readiness.checklistTitle') }}
          </h2>
          <ul class="mt-2 grid gap-1.5 text-sm text-(--muted-foreground)">
            <li
              v-for="reason in readiness.reasons"
              :key="reason"
              class="flex items-center gap-2"
            >
              <span aria-hidden="true">•</span>
              {{ t(`profiles.readiness.reason.${reason}`) }}
            </li>
          </ul>
        </section>

        <p
          v-if="save.isError.value"
          role="alert"
          class="rounded-(--radius-md) border border-(--danger) p-3 text-sm text-(--danger)"
        >
          {{ t('profiles.saveError') }}
        </p>
        <p
          v-if="sourceImport.isError.value"
          role="alert"
          class="rounded-(--radius-md) border border-(--danger) p-3 text-sm text-(--danger)"
        >
          {{ t('profiles.sources.importError') }}
        </p>

        <VacancySection
          :vacancy="draft.vacancy"
          @update="updateVacancy"
        />
        <ProfileSourcesSection
          :manual-context="draft.manualContext"
          :sources="draft.sources"
          :importing="sourceImport.isPending.value"
          @update:manual-context="draft.manualContext = $event"
          @import="sourceImport.mutate"
        />
        <ModelConfigurationSection
          :configuration="draft.modelConfiguration"
          @update="updateModels"
        />
      </template>
    </section>
  </main>
</template>
