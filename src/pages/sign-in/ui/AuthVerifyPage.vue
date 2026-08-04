<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink, useRoute, useRouter } from 'vue-router'
import { syncNativeSession, verifyMagicLink } from '@/shared/api/auth/client'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const state = ref<'pending' | 'expired' | 'used' | 'error'>('pending')

onMounted(async () => {
  const token = typeof route.query.token === 'string' ? route.query.token : ''
  if (!token) {
    state.value = 'error'
    return
  }
  try {
    await verifyMagicLink(token)
    await syncNativeSession()
    await router.replace('/permissions')
  } catch (cause) {
    const code = (cause as { code?: string }).code
    state.value = code === 'AUTH_TOKEN_EXPIRED' ? 'expired' : code === 'AUTH_TOKEN_USED' ? 'used' : 'error'
  }
})
</script>

<template>
  <main class="grid min-h-screen place-items-center bg-(--background) p-8 text-(--foreground)">
    <section
      class="w-full max-w-md text-center"
      aria-live="polite"
    >
      <h1 class="text-3xl font-semibold tracking-tight">
        {{ t(`auth.verify.${state}.title`) }}
      </h1>
      <p class="mt-3 text-sm text-(--muted-foreground)">
        {{ t(`auth.verify.${state}.description`) }}
      </p>
      <RouterLink
        v-if="state !== 'pending'"
        class="mt-6 inline-block font-semibold text-(--accent)"
        to="/sign-in"
      >
        {{ t('auth.verify.tryAgain') }}
      </RouterLink>
    </section>
  </main>
</template>
