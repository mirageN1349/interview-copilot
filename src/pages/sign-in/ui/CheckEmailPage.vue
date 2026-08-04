<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink, useRoute } from 'vue-router'
import DevInbox from '@/pages/sign-in/ui/DevInbox.vue'

const { t } = useI18n()
const route = useRoute()
const email = computed(() => typeof route.query.email === 'string' ? route.query.email : '')
const messages = ref<Array<{ email: string; url: string }>>([])

onMounted(async () => {
  if (!import.meta.env.DEV) return
  const { authScenario } = await import('@/mocks/scenarios/auth')
  messages.value = authScenario.devInbox()
})
</script>

<template>
  <main class="grid min-h-screen place-items-center bg-(--background) p-8 text-(--foreground)">
    <section
      class="w-full max-w-md"
      aria-labelledby="check-email-title"
    >
      <p class="font-semibold text-(--accent)">
        {{ t('auth.productName') }}
      </p>
      <h1
        id="check-email-title"
        class="mt-1 text-3xl font-semibold tracking-tight"
      >
        {{ t('auth.checkEmail.title') }}
      </h1>
      <p class="mt-4 text-sm">
        {{ email ? t('auth.checkEmail.sentTo', { email }) : t('auth.checkEmail.description') }}
      </p>
      <p class="mt-2 text-sm text-(--muted-foreground)">
        {{ t('auth.checkEmail.hint') }}
      </p>
      <RouterLink
        class="mt-6 inline-block font-semibold text-(--accent)"
        to="/sign-in"
      >
        {{ t('auth.checkEmail.useAnotherEmail') }}
      </RouterLink>
      <DevInbox :messages="messages" />
    </section>
  </main>
</template>
