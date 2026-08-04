<script setup lang="ts">
import type { DateValue } from '@internationalized/date'
import { CalendarDate, getLocalTimeZone, today } from '@internationalized/date'
import { CalendarDays } from '@lucide/vue'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'

import Button from '@/shared/ui/button/Button.vue'
import { Calendar } from '@/shared/ui/calendar'
import { Popover, PopoverContent, PopoverTrigger } from '@/shared/ui/popover'

const props = defineProps<{ modelValue?: number; placeholder: string; ariaLabel: string }>()
const emit = defineEmits<{ 'update:modelValue': [value: number | undefined] }>()
const { locale } = useI18n()
const open = ref(false)
const calendarLocale = computed(() => locale.value === 'ru' ? 'ru-RU' : 'en-US')
const date = computed<DateValue | undefined>(() => {
  if (props.modelValue === undefined) return undefined
  const value = new Date(props.modelValue)
  return new CalendarDate(value.getFullYear(), value.getMonth() + 1, value.getDate())
})
const valueLabel = computed(() => props.modelValue === undefined
  ? props.placeholder
  : new Intl.DateTimeFormat(calendarLocale.value, { day: 'numeric', month: 'short', year: 'numeric' }).format(props.modelValue))

function select(value: DateValue | undefined) {
  if (!value) return
  const selected = value.toDate(getLocalTimeZone())
  emit('update:modelValue', new Date(selected.getFullYear(), selected.getMonth(), selected.getDate()).getTime())
  open.value = false
}
</script>

<template>
  <Popover v-model:open="open">
    <PopoverTrigger as-child>
      <Button
        variant="secondary"
        class="w-full justify-between font-normal sm:w-48"
        :class="{ 'text-(--muted-foreground)': modelValue === undefined }"
        :aria-label="ariaLabel"
      >
        {{ valueLabel }}
        <CalendarDays
          :size="16"
          aria-hidden="true"
          class="text-(--muted-foreground)"
        />
      </Button>
    </PopoverTrigger>
    <PopoverContent
      class="w-auto overflow-hidden border-(--border) bg-(--surface-raised) p-0 text-(--foreground)"
      align="start"
      :aria-label="ariaLabel"
    >
      <Calendar
        :model-value="date"
        :default-placeholder="date ?? today(getLocalTimeZone())"
        :locale="calendarLocale"
        initial-focus
        @update:model-value="select"
      />
    </PopoverContent>
  </Popover>
</template>
