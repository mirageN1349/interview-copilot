<script setup lang="ts">
import { useI18n } from 'vue-i18n'

defineProps<{ messages: Array<{ email: string; url: string }> }>()
const { t } = useI18n()
const enabled = import.meta.env.DEV
</script>

<template>
  <aside
    v-if="enabled"
    class="mt-4 grid gap-2 border-t border-(--border) pt-4 text-sm text-(--muted-foreground)"
    :aria-label="t('auth.devInbox.label')"
  >
    <strong>{{ t('auth.devInbox.title') }}</strong>
    <p
      v-if="messages.length === 0"
      class="m-0"
    >
      {{ t('auth.devInbox.empty') }}
    </p>
    <a
      v-for="message in messages"
      :key="message.url"
      class="text-(--accent)"
      :href="message.url"
    >
      {{ t('auth.devInbox.openFor', { email: message.email }) }}
    </a>
  </aside>
</template>
