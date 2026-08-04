<script setup lang="ts">
import { Monitor, Moon, Sun } from '@lucide/vue'
import { useI18n } from 'vue-i18n'

import { appearanceState, initializeAppearanceProvider } from '@/app/providers/appearance'
import type { AppearanceTheme } from '@/shared/config/appearance'

const { t } = useI18n()
const appearance = initializeAppearanceProvider()
const choices: Array<{ value: AppearanceTheme; icon: typeof Sun }> = [
  { value: 'light', icon: Sun },
  { value: 'dark', icon: Moon },
  { value: 'auto', icon: Monitor },
]
</script>

<template>
  <section class="grid gap-4 rounded-(--radius-lg) border border-(--border) bg-(--surface) p-5">
    <div>
      <h2 class="font-semibold">
        {{ t('account.appearance.title') }}
      </h2><p class="text-sm text-(--muted-foreground)">
        {{ t('account.appearance.description') }}
      </p>
    </div>
    <fieldset class="grid gap-2 sm:grid-cols-3">
      <legend class="sr-only">
        {{ t('account.appearance.title') }}
      </legend>
      <label
        v-for="choice in choices"
        :key="choice.value"
        class="flex cursor-pointer items-center gap-2 rounded-(--radius-md) border border-(--border) p-3 has-checked:border-(--accent) has-checked:bg-(--muted)"
      >
        <input
          :checked="appearanceState.theme === choice.value"
          class="accent-(--accent)"
          type="radio"
          name="theme"
          :value="choice.value"
          @change="appearance.setTheme(choice.value)"
        >
        <component
          :is="choice.icon"
          :size="16"
          aria-hidden="true"
        />{{ t(`account.appearance.${choice.value}`) }}
      </label>
    </fieldset>
    <p class="text-xs text-(--muted-foreground)">
      {{ t('account.appearance.active', { theme: t(`account.appearance.${appearanceState.resolvedTheme}`) }) }}
    </p>
  </section>
</template>
