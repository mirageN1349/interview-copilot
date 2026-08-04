<script setup lang="ts">
import { Check, LogOut, Sparkles } from '@lucide/vue'
import { useMutation, useQuery, useQueryClient } from '@tanstack/vue-query'
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'

import { subscriptionGateway, subscriptionQuery, updateSubscriptionCache } from '@/entities/subscription'
import { authClient, signOut } from '@/shared/api/auth/client'
import Button from '@/shared/ui/button/Button.vue'
import AppearanceSettings from './AppearanceSettings.vue'

const { t, locale } = useI18n()
const router = useRouter()
const queryClient = useQueryClient()
const subscription = useQuery(subscriptionQuery())
const user = ref<{ name: string; email: string } | null>(null)
const signOutError = ref(false)

onMounted(async () => {
  const session = await authClient.getSession()
  if (session.data) user.value = { name: session.data.user.name, email: session.data.user.email }
})

const activate = useMutation({
  mutationFn: subscriptionGateway.activate,
  onSuccess: (value) => updateSubscriptionCache(queryClient, value),
})

async function leave() {
  signOutError.value = false
  try {
    await signOut()
    queryClient.clear()
    await router.replace('/sign-in')
  } catch {
    signOutError.value = true
  }
}

function setLocale(value: 'en' | 'ru') {
  locale.value = value
  localStorage.setItem('locale', value)
  document.documentElement.lang = value
}

const featureKeys: Record<string, string> = {
  'Live assistant': 'account.subscription.features.live',
  Profiles: 'account.subscription.features.profiles',
  'Meeting history': 'account.subscription.features.history',
}
</script>

<template>
  <main class="min-h-screen bg-(--background) px-6 py-10 text-(--foreground)">
    <section class="mx-auto grid w-full max-w-3xl gap-6">
      <header class="grid gap-1">
        <p class="text-sm font-medium text-(--accent)">
          {{ t('account.eyebrow') }}
        </p>
        <h1 class="text-3xl font-semibold tracking-tight">
          {{ t('account.title') }}
        </h1>
      </header>
      <section class="grid gap-4 rounded-(--radius-lg) border border-(--border) bg-(--surface) p-5">
        <h2 class="font-semibold">
          {{ t('account.profile.title') }}
        </h2>
        <dl
          v-if="user"
          class="grid gap-3 text-sm sm:grid-cols-2"
        >
          <div>
            <dt class="text-(--muted-foreground)">
              {{ t('account.profile.name') }}
            </dt><dd class="font-medium">
              {{ user.name }}
            </dd>
          </div>
          <div>
            <dt class="text-(--muted-foreground)">
              {{ t('account.profile.email') }}
            </dt><dd class="font-medium">
              {{ user.email }}
            </dd>
          </div>
        </dl>
      </section>
      <section class="grid gap-4 rounded-(--radius-lg) border border-(--border) bg-(--surface) p-5">
        <div class="flex items-start justify-between gap-4">
          <div>
            <h2 class="font-semibold">
              {{ t('account.subscription.title') }}
            </h2><p class="text-sm text-(--muted-foreground)">
              {{ t('account.subscription.description') }}
            </p>
          </div>
          <span class="rounded-full bg-(--muted) px-3 py-1 text-sm font-medium">{{ t('account.subscription.demo') }}</span>
        </div>
        <p
          v-if="subscription.isError.value"
          role="alert"
          class="text-sm text-(--danger)"
        >
          {{ t('account.subscription.loadError') }}
        </p>
        <template v-else-if="subscription.data.value">
          <ul class="grid gap-2 text-sm">
            <li
              v-for="feature in subscription.data.value.features"
              :key="feature"
              class="flex items-center gap-2"
            >
              <Check
                :size="16"
                aria-hidden="true"
              />{{ t(featureKeys[feature] ?? feature) }}
            </li>
          </ul>
          <p
            v-if="subscription.data.value.status === 'active'"
            role="status"
            class="text-sm font-medium text-(--accent)"
          >
            {{ t('account.subscription.active') }}
          </p>
          <Button
            v-else
            class="w-fit"
            :disabled="activate.isPending.value"
            @click="activate.mutate()"
          >
            <Sparkles
              :size="16"
              aria-hidden="true"
            />{{ t('account.subscription.activate') }}
          </Button>
        </template>
      </section>
      <AppearanceSettings />
      <section class="grid gap-3 rounded-(--radius-lg) border border-(--border) bg-(--surface) p-5">
        <div>
          <h2 class="font-semibold">
            {{ t('account.language.title') }}
          </h2><p class="text-sm text-(--muted-foreground)">
            {{ t('account.language.description') }}
          </p>
        </div>
        <select
          class="h-9 w-fit rounded-(--radius-sm) border border-(--input) bg-(--surface-raised) px-3 text-sm"
          :value="locale"
          @change="setLocale(($event.target as HTMLSelectElement).value as 'en' | 'ru')"
        >
          <option value="ru">
            {{ t('account.language.ru') }}
          </option>
          <option value="en">
            {{ t('account.language.en') }}
          </option>
        </select>
      </section>
      <section class="grid gap-3 rounded-(--radius-lg) border border-(--border) bg-(--surface) p-5">
        <Button
          class="w-fit"
          variant="secondary"
          @click="leave"
        >
          <LogOut
            :size="16"
            aria-hidden="true"
          />{{ t('account.signOut') }}
        </Button>
        <p
          v-if="signOutError"
          role="alert"
          class="text-sm text-(--danger)"
        >
          {{ t('account.signOutError') }}
        </p>
      </section>
    </section>
  </main>
</template>
