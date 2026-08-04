<script setup lang="ts">
import { Check, ExternalLink, Mic, Monitor, ScanFace } from '@lucide/vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

import {
  stateFor,
  usePermissions,
  type PermissionKind,
} from '@/pages/permissions/model/use-permissions'

const router = useRouter()
const { t } = useI18n()
const { snapshot, phase, error, nextMissing, request, openSettings } = usePermissions()

const permissions: Array<{
  kind: PermissionKind
  title: string
  description: string
  icon: typeof Monitor
}> = [
  {
    kind: 'screen',
    title: 'permissions.screen.title',
    description: 'permissions.screen.description',
    icon: Monitor,
  },
  {
    kind: 'microphone',
    title: 'permissions.microphone.title',
    description: 'permissions.microphone.description',
    icon: Mic,
  },
  {
    kind: 'accessibility',
    title: 'permissions.accessibility.title',
    description: 'permissions.accessibility.description',
    icon: ScanFace,
  },
]
</script>

<template>
  <main class="min-h-screen bg-(--background) px-6 py-12 text-(--foreground)">
    <section class="mx-auto grid w-full max-w-2xl gap-8">
      <header class="grid gap-2">
        <p class="text-sm font-medium text-(--accent)">
          {{ t('permissions.eyebrow') }}
        </p>
        <h1 class="text-3xl font-semibold tracking-tight">
          {{ t('permissions.title') }}
        </h1>
        <p class="max-w-xl text-sm leading-6 text-(--muted-foreground)">
          {{ t('permissions.intro') }}
        </p>
      </header>

      <ol class="overflow-hidden rounded-(--radius-lg) border border-(--border) bg-(--surface)">
        <li
          v-for="permission in permissions"
          :key="permission.kind"
          class="grid grid-cols-[auto_1fr_auto] items-center gap-4 border-b border-(--border) p-5 last:border-b-0"
        >
          <component
            :is="permission.icon"
            :size="18"
            aria-hidden="true"
          />
          <div class="grid gap-1">
            <h2 class="font-medium">
              {{ t(permission.title) }}
            </h2>
            <p class="text-sm text-(--muted-foreground)">
              {{ t(permission.description) }}
            </p>
          </div>

          <div
            v-if="snapshot"
            class="flex items-center gap-2"
          >
            <span
              v-if="stateFor(snapshot, permission.kind) === 'granted'"
              class="inline-flex items-center gap-1.5 text-sm font-medium text-emerald-700 dark:text-emerald-400"
            >
              <Check
                :size="16"
                aria-hidden="true"
              />
              {{ t('common.allowed') }}
            </span>
            <button
              v-else-if="stateFor(snapshot, permission.kind) === 'not_determined'"
              class="h-9 rounded-(--radius-sm) bg-(--accent) px-4 text-sm font-medium text-(--accent-foreground) hover:opacity-90 disabled:opacity-50"
              :disabled="phase === 'requesting' || nextMissing !== permission.kind"
              type="button"
              @click="request(permission.kind)"
            >
              {{ t('common.allow') }}
            </button>
            <button
              v-else
              class="inline-flex h-9 items-center gap-2 rounded-(--radius-sm) border border-(--border) px-3 text-sm hover:bg-(--muted)"
              type="button"
              @click="openSettings(permission.kind)"
            >
              {{ t('common.settings') }}
              <ExternalLink
                :size="14"
                aria-hidden="true"
              />
            </button>
          </div>
        </li>
      </ol>

      <p
        v-if="error"
        role="alert"
        class="text-sm text-(--danger)"
      >
        {{ t(error) }}
      </p>

      <div class="flex justify-end">
        <button
          class="h-10 rounded-(--radius-sm) bg-(--accent) px-5 text-sm font-medium text-(--accent-foreground) disabled:opacity-40"
          type="button"
          :disabled="phase !== 'complete'"
          @click="router.replace('/profiles')"
        >
          {{ t('common.continue') }}
        </button>
      </div>
    </section>
  </main>
</template>
