<script setup lang="ts">
import { FolderUp, UserRoundPen } from '@lucide/vue'
import { useI18n } from 'vue-i18n'

import type { ProfileSource } from '@/entities/interview-profile'
import Button from '@/shared/ui/button/Button.vue'
import ProfileFactList from './ProfileFactList.vue'

defineProps<{ manualContext: string; sources: ProfileSource[]; importing?: boolean }>()
const emit = defineEmits<{
  'update:manualContext': [value: string]
  import: [payload: { fixtureId: string; kind: 'resume' | 'project' }]
}>()
const { t } = useI18n()
</script>

<template>
  <section class="grid gap-5 rounded-(--radius-lg) border border-(--border) bg-(--surface) p-5 sm:p-6">
    <header class="grid gap-1">
      <h2 class="text-base font-semibold">
        {{ t('profiles.sources.title') }}
      </h2>
      <p class="text-sm text-(--muted-foreground)">
        {{ t('profiles.sources.description') }}
      </p>
    </header>
    <label class="grid gap-2 text-sm font-medium">
      <span class="inline-flex items-center gap-2"><UserRoundPen
        :size="16"
        aria-hidden="true"
      />{{ t('profiles.sources.manual') }}</span>
      <textarea
        class="min-h-36 resize-y rounded-(--radius-md) border border-(--input) bg-(--surface-raised) px-4 py-3 font-normal outline-none transition-[border-color,box-shadow] focus:border-(--ring) focus:ring-3 focus:ring-(--ring)/15"
        :value="manualContext"
        maxlength="20000"
        :placeholder="t('profiles.sources.manualPlaceholder')"
        @input="emit('update:manualContext', ($event.target as HTMLTextAreaElement).value)"
      />
    </label>
    <div class="flex flex-wrap gap-3">
      <Button
        class="!min-h-11 !rounded-full !px-5"
        variant="secondary"
        :disabled="importing"
        @click="emit('import', { fixtureId: 'resume-product-engineer', kind: 'resume' })"
      >
        <FolderUp
          :size="16"
          aria-hidden="true"
        />{{ t('profiles.sources.addResume') }}
      </Button>
      <Button
        class="!min-h-11 !rounded-full !px-5"
        variant="secondary"
        :disabled="importing"
        @click="emit('import', { fixtureId: 'project-performance', kind: 'project' })"
      >
        <FolderUp
          :size="16"
          aria-hidden="true"
        />{{ t('profiles.sources.addProject') }}
      </Button>
    </div>
    <div class="grid gap-3">
      <ProfileFactList
        v-for="source in sources"
        :key="source.id"
        :source="source"
      />
      <p
        v-if="sources.length === 0"
        class="rounded-(--radius-md) border border-dashed border-(--border) p-5 text-sm text-(--muted-foreground)"
      >
        {{ t('profiles.sources.empty') }}
      </p>
    </div>
  </section>
</template>
