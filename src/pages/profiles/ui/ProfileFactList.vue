<script setup lang="ts">
import { FileCheck2 } from '@lucide/vue'
import { useI18n } from 'vue-i18n'
import type { ProfileSource } from '@/entities/interview-profile'

defineProps<{ source: ProfileSource }>()
const { t } = useI18n()
</script>

<template>
  <article class="grid gap-3 rounded-(--radius-md) border border-(--border) bg-(--surface-raised) p-4">
    <header class="flex items-start justify-between gap-3">
      <div class="flex items-center gap-2">
        <FileCheck2
          :size="17"
          aria-hidden="true"
        />
        <div>
          <h3 class="font-medium">
            {{ source.displayName }}
          </h3>
          <p class="text-xs text-(--muted-foreground)">
            {{ t(`profiles.sources.kind.${source.kind}`) }}
          </p>
        </div>
      </div>
      <span class="rounded-full bg-(--muted) px-2 py-1 text-xs">{{ t(`profiles.sources.status.${source.contentStatus}`) }}</span>
    </header>
    <p
      v-if="source.redactionSummary"
      class="text-sm text-(--muted-foreground)"
    >
      {{ source.redactionSummary }}
    </p>
    <ul class="grid gap-2">
      <li
        v-for="fact in source.extractedFacts"
        :key="fact.id"
        class="grid grid-cols-[1fr_auto] gap-3 text-sm"
      >
        <span>{{ fact.text }}</span>
        <span class="text-xs text-(--muted-foreground)">{{ fact.sourceRange }}</span>
      </li>
    </ul>
    <p
      v-if="source.extractedFacts.length === 0"
      class="text-sm text-(--muted-foreground)"
    >
      {{ t('profiles.sources.noFacts') }}
    </p>
  </article>
</template>
