<script setup lang="ts">
import { Archive, ArchiveRestore, ChevronRight, Plus, UserRound } from '@lucide/vue'
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

import { invalidateProfileQueries, profileGateway, profilesQuery } from '@/entities/interview-profile'
import Button from '@/shared/ui/button/Button.vue'
import Dialog from '@/shared/ui/dialog/Dialog.vue'
import ProfileCreateDialog from './ProfileCreateDialog.vue'

const { t, d } = useI18n()
const router = useRouter()
const queryClient = useQueryClient()
const createOpen = ref(false)
const archiveTarget = ref<{ id: string; name: string; revision: number } | null>(null)
const profiles = useQuery(profilesQuery())
const activeProfiles = computed(() => profiles.data.value?.filter((profile) => profile.status !== 'archived') ?? [])
const archivedProfiles = computed(() => profiles.data.value?.filter((profile) => profile.status === 'archived') ?? [])

function openCreate() {
  createProfile.reset()
  createOpen.value = true
}

const createProfile = useMutation({
  mutationFn: (name: string) => profileGateway.save({ name, manualContext: '', vacancy: null, modelConfiguration: null }),
  onSuccess: async (profile) => {
    await invalidateProfileQueries(queryClient, profile.id)
    createOpen.value = false
    await router.push(`/profiles/${profile.id}`)
  },
})

const archiveProfile = useMutation({
  mutationFn: ({ id, revision }: { id: string; revision: number }) => profileGateway.archive(id, revision),
  onSuccess: async () => {
    archiveTarget.value = null
    await invalidateProfileQueries(queryClient)
  },
})

const restoreProfile = useMutation({
  mutationFn: ({ id, revision }: { id: string; revision: number }) => profileGateway.restore(id, revision),
  onSuccess: async (profile) => {
    await invalidateProfileQueries(queryClient, profile.id)
    await router.push(`/profiles/${profile.id}`)
  },
})
</script>

<template>
  <main class="min-h-screen bg-(--background) px-6 py-10 text-(--foreground)">
    <section class="mx-auto grid w-full max-w-5xl gap-7">
      <header class="flex items-start justify-between gap-5">
        <div class="grid gap-1">
          <p class="text-sm font-medium text-(--accent)">
            {{ t('profiles.eyebrow') }}
          </p>
          <h1 class="text-3xl font-semibold tracking-tight">
            {{ t('profiles.title') }}
          </h1>
          <p class="text-sm text-(--muted-foreground)">
            {{ t('profiles.description') }}
          </p>
        </div>
        <Button
          size="lg"
          @click="openCreate"
        >
          <Plus
            :size="16"
            aria-hidden="true"
          />{{ t('profiles.new') }}
        </Button>
      </header>

      <p
        v-if="profiles.isPending.value"
        class="rounded-(--radius-md) border border-(--border) p-6 text-(--muted-foreground)"
      >
        {{ t('common.loading') }}
      </p>
      <div
        v-else-if="profiles.isError.value"
        role="alert"
        class="rounded-(--radius-md) border border-(--danger) p-6 text-(--danger)"
      >
        <p>{{ t('profiles.loadError') }}</p>
        <Button
          class="mt-4"
          variant="secondary"
          @click="profiles.refetch()"
        >
          {{ t('common.retry') }}
        </Button>
      </div>
      <section
        v-else-if="activeProfiles.length === 0"
        class="grid place-items-center gap-3 rounded-(--radius-lg) border border-dashed border-(--border) bg-(--surface) px-8 py-16 text-center"
      >
        <span class="grid size-11 place-items-center rounded-full bg-(--muted)"><UserRound
          :size="20"
          aria-hidden="true"
        /></span>
        <h2 class="text-lg font-semibold">
          {{ t('profiles.empty.title') }}
        </h2>
        <p class="max-w-md text-sm text-(--muted-foreground)">
          {{ t('profiles.empty.description') }}
        </p>
        <Button
          size="lg"
          @click="openCreate"
        >
          {{ t('profiles.empty.action') }}
        </Button>
      </section>
      <ul
        v-if="activeProfiles.length"
        class="grid gap-3"
      >
        <li
          v-for="profile in activeProfiles"
          :key="profile.id"
          class="group grid grid-cols-[1fr_auto] items-center gap-4 rounded-(--radius-md) border border-(--border) bg-(--surface) p-4 hover:bg-(--surface-raised)"
        >
          <button
            class="grid gap-1 text-left"
            type="button"
            @click="router.push(`/profiles/${profile.id}`)"
          >
            <span class="font-medium">{{ profile.name }}</span>
            <span class="text-xs text-(--muted-foreground)">
              {{ t(`profiles.status.${profile.status}`) }} · {{ d(new Date(profile.updatedAtMs), 'short') }}
            </span>
          </button>
          <div class="flex items-center gap-1">
            <Button
              variant="ghost"
              size="icon"
              :aria-label="t('profiles.archive')"
              :disabled="archiveProfile.isPending.value"
              @click="archiveTarget = { id: profile.id, name: profile.name, revision: profile.revision }"
            >
              <Archive
                :size="16"
                aria-hidden="true"
              />
            </Button>
            <ChevronRight
              :size="18"
              aria-hidden="true"
              class="text-(--muted-foreground)"
            />
          </div>
        </li>
      </ul>

      <section
        v-if="archivedProfiles.length"
        class="grid gap-3 border-t border-(--border) pt-6"
      >
        <div>
          <h2 class="font-semibold">
            {{ t('profiles.archivedTitle') }}
          </h2>
          <p class="text-sm text-(--muted-foreground)">
            {{ t('profiles.archivedDescription') }}
          </p>
        </div>
        <ul class="grid gap-2">
          <li
            v-for="profile in archivedProfiles"
            :key="profile.id"
            class="flex items-center justify-between gap-4 rounded-(--radius-md) border border-(--border) bg-(--surface) p-4"
          >
            <div class="grid gap-1">
              <span class="font-medium">{{ profile.name }}</span>
              <span class="text-xs text-(--muted-foreground)">{{ t('profiles.status.archived') }} · {{ d(new Date(profile.updatedAtMs), 'short') }}</span>
            </div>
            <Button
              variant="secondary"
              :disabled="restoreProfile.isPending.value"
              @click="restoreProfile.mutate({ id: profile.id, revision: profile.revision })"
            >
              <ArchiveRestore
                :size="16"
                aria-hidden="true"
              />
              {{ restoreProfile.isPending.value ? t('common.saving') : t('profiles.restore') }}
            </Button>
          </li>
        </ul>
        <p
          v-if="restoreProfile.isError.value"
          role="alert"
          class="text-sm text-(--danger)"
        >
          {{ t('profiles.restoreError') }}
        </p>
      </section>
    </section>

    <ProfileCreateDialog
      v-model:open="createOpen"
      :pending="createProfile.isPending.value"
      :error="createProfile.isError.value ? t('profiles.create.error') : undefined"
      @create="createProfile.mutate"
    />
    <Dialog
      :open="Boolean(archiveTarget)"
      :title="t('profiles.archiveConfirm.title')"
      :description="t('profiles.archiveConfirm.description', { name: archiveTarget?.name })"
      @update:open="archiveTarget = null"
    >
      <template #footer>
        <Button
          variant="secondary"
          @click="archiveTarget = null"
        >
          {{ t('common.cancel') }}
        </Button>
        <Button
          variant="danger"
          :disabled="archiveProfile.isPending.value"
          @click="archiveTarget && archiveProfile.mutate(archiveTarget)"
        >
          {{ archiveProfile.isPending.value ? t('common.saving') : t('profiles.archive') }}
        </Button>
      </template>
    </Dialog>
  </main>
</template>
