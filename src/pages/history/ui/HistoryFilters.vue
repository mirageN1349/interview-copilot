<script setup lang="ts">
import { Search, X } from '@lucide/vue'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

import type { HistoryFilters } from '@/entities/meeting'
import Button from '@/shared/ui/button/Button.vue'
import DatePicker from '@/shared/ui/date-picker/DatePicker.vue'
import Input from '@/shared/ui/input/Input.vue'

const filters = defineModel<Partial<HistoryFilters>>({ required: true })
const { t } = useI18n()
const dateFrom = computed({
  get: () => filters.value.fromMs,
  set: (value: number | undefined) => { filters.value = { ...filters.value, fromMs: value } },
})
const dateTo = computed({
  get: () => filters.value.toMs,
  set: (value: number | undefined) => {
    const endOfDay = value === undefined ? undefined : new Date(new Date(value).setHours(23, 59, 59, 999)).getTime()
    filters.value = { ...filters.value, toMs: endOfDay }
  },
})

function clear() {
  filters.value = { field: 'any', limit: filters.value.limit ?? 30 }
}
</script>

<template>
  <form
    class="grid gap-5 rounded-(--radius-lg) border border-(--border) bg-(--surface) p-5"
    role="search"
    @submit.prevent
  >
    <div class="grid items-end gap-4 md:grid-cols-[minmax(16rem,1fr)_11rem] lg:grid-cols-[minmax(18rem,1fr)_11rem_minmax(13rem,16rem)]">
      <label class="grid min-w-0 gap-1.5 text-sm">
        <span class="font-medium">{{ t('history.filters.query') }}</span>
        <span class="relative min-w-0">
          <Search
            class="pointer-events-none absolute left-3 top-1/2 z-10 -translate-y-1/2 text-(--muted-foreground)"
            :size="16"
            aria-hidden="true"
          />
          <Input
            v-model="filters.query"
            type="search"
            :placeholder="t('history.filters.queryPlaceholder')"
            style="padding-left: 2.5rem"
          />
        </span>
      </label>
      <label class="grid gap-1.5 text-sm">
        <span class="font-medium">{{ t('history.filters.field') }}</span>
        <select
          v-model="filters.field"
          class="h-(--control-height) rounded-(--radius-sm) border border-(--input) bg-(--surface-raised) px-3 outline-none focus-visible:ring-2 focus-visible:ring-(--focus)"
        >
          <option
            v-for="field in ['any', 'title', 'vacancy', 'transcript', 'chat']"
            :key="field"
            :value="field"
          >
            {{ t(`history.filters.fields.${field}`) }}
          </option>
        </select>
      </label>
      <label class="grid gap-1.5 text-sm md:col-span-2 lg:col-span-1">
        <span class="font-medium">{{ t('history.filters.profile') }}</span>
        <Input
          v-model="filters.profileQuery"
          :placeholder="t('history.filters.profilePlaceholder')"
        />
      </label>
    </div>
    <div class="flex flex-col gap-4 border-t border-(--border) pt-4 sm:flex-row sm:items-end sm:justify-between">
      <fieldset class="grid gap-3 sm:grid-cols-2">
        <legend class="mb-1 text-sm font-medium sm:col-span-2">
          {{ t('history.filters.period') }}
        </legend>
        <div class="grid gap-1.5 text-sm">
          <span class="text-(--muted-foreground)">{{ t('history.filters.from') }}</span>
          <DatePicker
            v-model="dateFrom"
            class="w-full sm:w-48"
            :placeholder="t('history.filters.chooseDate')"
            :aria-label="t('history.filters.fromDateLabel')"
          />
        </div>
        <div class="grid gap-1.5 text-sm">
          <span class="text-(--muted-foreground)">{{ t('history.filters.to') }}</span>
          <DatePicker
            v-model="dateTo"
            class="w-full sm:w-48"
            :placeholder="t('history.filters.chooseDate')"
            :aria-label="t('history.filters.toDateLabel')"
          />
        </div>
      </fieldset>
      <Button
        class="self-start sm:self-end"
        type="button"
        variant="ghost"
        @click="clear"
      >
        <X
          :size="16"
          aria-hidden="true"
        />{{ t('history.filters.clear') }}
      </Button>
    </div>
  </form>
</template>
