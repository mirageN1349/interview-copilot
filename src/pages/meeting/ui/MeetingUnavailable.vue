<script setup lang="ts">
import { useI18n } from 'vue-i18n'

import { publicMeetingUnavailable } from '@/shared/lib/public-copy'

defineProps<{ reasonCode?: string }>()
const { t } = useI18n()
const copy = publicMeetingUnavailable()
</script>

<template>
  <section
    role="status"
    class="grid gap-3 rounded-(--radius-md) border border-(--border) bg-(--surface-raised) p-5"
    :aria-label="t(copy.titleKey)"
  >
    <h2 class="text-base font-semibold">
      {{ t(copy.titleKey) }}
    </h2>
    <p class="text-sm text-(--muted-foreground)">
      {{ t(copy.detailKey) }}
    </p>
    <RouterLink
      to="/meetings/new"
      class="w-fit text-sm font-medium text-(--accent-foreground) underline underline-offset-4"
    >
      {{ t(copy.actionKey) }}
    </RouterLink>
  </section>
</template>
