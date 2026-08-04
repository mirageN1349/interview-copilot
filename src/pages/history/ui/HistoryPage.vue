<script setup lang="ts">
import { ChevronRight, History } from '@lucide/vue'
import { useInfiniteQuery } from '@tanstack/vue-query'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

import { historyQuery, type HistoryFilters as HistoryFilterValues } from '@/entities/meeting'
import Button from '@/shared/ui/button/Button.vue'
import HistoryFilters from './HistoryFilters.vue'

const { t, d } = useI18n()
const router = useRouter()
const filters = ref<Partial<HistoryFilterValues>>({ field: 'any', limit: 30 })
const options = computed(() => historyQuery(filters.value))
const history = useInfiniteQuery(options)
const meetings = computed(() => history.data.value?.pages.flatMap((page) => page.items) ?? [])
</script>

<template>
  <main class="min-h-screen bg-(--background) px-6 py-10 text-(--foreground)">
    <section class="mx-auto grid w-full max-w-5xl gap-6">
      <header class="grid gap-1">
        <p class="text-sm font-medium text-(--accent)">
          {{ t('history.eyebrow') }}
        </p>
        <h1 class="text-3xl font-semibold tracking-tight">
          {{ t('history.title') }}
        </h1>
        <p class="text-sm text-(--muted-foreground)">
          {{ t('history.description') }}
        </p>
      </header>
      <HistoryFilters v-model="filters" />
      <p
        v-if="history.isPending.value"
        class="text-(--muted-foreground)"
      >
        {{ t('common.loading') }}
      </p>
      <div
        v-else-if="history.isError.value"
        role="alert"
        class="rounded-(--radius-md) border border-(--danger) p-5 text-(--danger)"
      >
        <p>{{ t('history.loadError') }}</p>
        <Button
          class="mt-3"
          variant="secondary"
          @click="history.refetch()"
        >
          {{ t('common.retry') }}
        </Button>
      </div>
      <section
        v-else-if="meetings.length === 0"
        class="grid place-items-center gap-2 rounded-(--radius-lg) border border-dashed border-(--border) p-14 text-center"
      >
        <History
          :size="22"
          aria-hidden="true"
        />
        <h2 class="font-semibold">
          {{ t('history.empty.title') }}
        </h2>
        <p class="text-sm text-(--muted-foreground)">
          {{ t('history.empty.description') }}
        </p>
      </section>
      <ul
        v-else
        class="grid gap-3"
      >
        <li
          v-for="meeting in meetings"
          :key="meeting.id"
        >
          <button
            class="grid w-full grid-cols-[1fr_auto] items-center gap-4 rounded-(--radius-md) border border-(--border) bg-(--surface) p-4 text-left hover:bg-(--surface-raised)"
            type="button"
            @click="router.push(`/history/${meeting.id}`)"
          >
            <span class="grid gap-1">
              <span class="font-medium">{{ meeting.title }}</span>
              <span class="text-sm text-(--muted-foreground)">{{ meeting.profileName }}<template v-if="meeting.vacancyRole"> · {{ meeting.vacancyRole }}</template></span>
              <span class="text-xs text-(--muted-foreground)">{{ d(new Date(meeting.createdAtMs), 'short') }} · {{ t(`history.status.${meeting.status}`) }}</span>
            </span>
            <ChevronRight
              :size="18"
              aria-hidden="true"
            />
          </button>
        </li>
      </ul>
      <Button
        v-if="history.hasNextPage.value"
        class="justify-self-center"
        variant="secondary"
        :disabled="history.isFetchingNextPage.value"
        @click="history.fetchNextPage()"
      >
        {{ history.isFetchingNextPage.value ? t('common.loading') : t('history.loadMore') }}
      </Button>
    </section>
  </main>
</template>
