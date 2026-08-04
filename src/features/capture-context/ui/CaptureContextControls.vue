<script setup lang="ts">
import { Crop, Monitor, ScanLine } from '@lucide/vue'
import { useQuery } from '@tanstack/vue-query'
import { storeToRefs } from 'pinia'
import { watch } from 'vue'
import { useI18n } from 'vue-i18n'

import { meetingGateway } from '@/entities/meeting'
import { nativeGateway } from '@/shared/api/native'
import Button from '@/shared/ui/button/Button.vue'
import type { ScreenshotArtifact } from '../model/capture-actions'
import { useCaptureUiStore } from '../model/capture-ui-store'

const props = defineProps<{ meetingId: string; initialDisplayId?: number; thread: 'live' | 'side' }>()
const emit = defineEmits<{ captured: [artifact: ScreenshotArtifact] }>()
const { t } = useI18n()
const store = useCaptureUiStore()
const { selectedDisplayId, areaDraft, autoScreenshotMode, lastRedactionSummary } = storeToRefs(store)
const displays = useQuery({ queryKey: ['displays'], queryFn: meetingGateway.listDisplays })

watch(() => [props.initialDisplayId, displays.data.value] as const, ([initial, items]) => {
  if (selectedDisplayId.value !== null) return
  const display = items?.find((item) => item.displayId === initial) ?? items?.find((item) => item.isPrimary) ?? items?.[0]
  if (display) store.selectDisplay(display.displayId, display.backingScale)
}, { immediate: true })

async function capture(mode: 'display' | 'area') {
  if (selectedDisplayId.value === null) return
  const artifact = await nativeGateway.invoke<ScreenshotArtifact>('capture_screenshot', {
    input: {
      meetingId: props.meetingId,
      displayId: selectedDisplayId.value,
      mode,
      ...(mode === 'area' && areaDraft.value ? { area: areaDraft.value } : {}),
      chatThread: props.thread,
    },
  })
  lastRedactionSummary.value = artifact.redactionSummary ?? null
  emit('captured', artifact)
}
</script>

<template>
  <section class="grid gap-3 text-sm">
    <label class="grid gap-1 font-medium">
      <span class="inline-flex items-center gap-2"><Monitor
        :size="15"
        aria-hidden="true"
      />{{ t('capture.display') }}</span>
      <select
        :value="selectedDisplayId ?? ''"
        class="h-9 rounded-(--radius-sm) border border-(--input) bg-(--surface-raised) px-2 font-normal"
        @change="(() => { const display = displays.data.value?.find((item) => item.displayId === Number(($event.target as HTMLSelectElement).value)); if (display) store.selectDisplay(display.displayId, display.backingScale) })()"
      >
        <option
          v-for="display in displays.data.value"
          :key="display.displayId"
          :value="display.displayId"
        >
          {{ t('meeting.new.displayName', { number: display.label }) }}
        </option>
      </select>
    </label>

    <fieldset class="grid grid-cols-4 gap-2">
      <legend class="col-span-4 mb-1 font-medium">
        {{ t('capture.area') }}
      </legend>
      <label
        v-for="field in ['x', 'y', 'width', 'height'] as const"
        :key="field"
        class="grid gap-1 text-xs text-(--muted-foreground)"
      >
        {{ t(`capture.coordinate.${field}`) }}
        <input
          type="number"
          class="h-8 min-w-0 rounded-(--radius-sm) border border-(--input) bg-(--surface-raised) px-2 text-(--foreground)"
          :min="field === 'width' || field === 'height' ? 1 : undefined"
          :value="areaDraft?.[field] ?? (field === 'width' ? 900 : field === 'height' ? 600 : 0)"
          @input="store.setArea({ x: areaDraft?.x ?? 0, y: areaDraft?.y ?? 0, width: areaDraft?.width ?? 900, height: areaDraft?.height ?? 600, [field]: Number(($event.target as HTMLInputElement).value) })"
        >
      </label>
    </fieldset>

    <label class="grid gap-1 font-medium">
      {{ t('capture.auto') }}
      <select
        v-model="autoScreenshotMode"
        class="h-9 rounded-(--radius-sm) border border-(--input) bg-(--surface-raised) px-2 font-normal"
      >
        <option value="off">{{ t('capture.autoMode.off') }}</option>
        <option value="display">{{ t('capture.autoMode.display') }}</option>
        <option value="area">{{ t('capture.autoMode.area') }}</option>
      </select>
    </label>

    <div class="flex gap-2">
      <Button
        size="sm"
        variant="secondary"
        :disabled="selectedDisplayId === null"
        @click="capture('display')"
      >
        <ScanLine
          :size="14"
          aria-hidden="true"
        />{{ t('capture.full') }}
      </Button>
      <Button
        size="sm"
        variant="secondary"
        :disabled="selectedDisplayId === null || !areaDraft"
        @click="capture('area')"
      >
        <Crop
          :size="14"
          aria-hidden="true"
        />{{ t('capture.areaAction') }}
      </Button>
    </div>
    <p
      v-if="lastRedactionSummary"
      role="status"
      class="text-xs text-(--muted-foreground)"
    >
      {{ lastRedactionSummary }}
    </p>
  </section>
</template>
