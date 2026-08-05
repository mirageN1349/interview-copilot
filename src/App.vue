<script setup lang="ts">
import { CircleUserRound, History, Plus, UserRound } from '@lucide/vue'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { RouterLink, RouterView, useRoute } from 'vue-router'

const route = useRoute()
const { t } = useI18n()
const showNavigation = computed(() => !['/sign-in', '/auth/', '/permissions', '/overlay/'].some((prefix) => route.path.startsWith(prefix)))
const links = [
  { to: '/profiles', key: 'navigation.profiles', icon: UserRound },
  { to: '/meetings/new', key: 'navigation.newMeeting', icon: Plus },
  { to: '/history', key: 'navigation.history', icon: History },
]
const accountLink = { to: '/account', key: 'navigation.account', icon: CircleUserRound }
</script>

<template>
  <nav
    v-if="showNavigation"
    class="app-navigation material-shell fixed left-1/2 top-3 z-40 grid h-14 w-[calc(100%-1.5rem)] max-w-5xl -translate-x-1/2 grid-cols-[1fr_auto_1fr] items-center px-2.5 text-(--foreground)"
    :aria-label="t('navigation.title')"
  >
    <div
      class="app-brand flex min-w-0 items-center gap-2 px-2"
      aria-label="Interview Copilot"
    >
      <img
        class="app-brand-logo"
        src="/brand/interview-copilot-pixel-icon.png"
        alt=""
        aria-hidden="true"
      >
      <span class="app-brand-name truncate text-sm font-semibold tracking-tight">Interview Copilot</span>
    </div>
    <div class="flex min-w-0 items-center justify-center gap-0.5">
      <RouterLink
        v-for="link in links"
        :key="link.to"
        :to="link.to"
        class="app-navigation-link inline-flex min-h-9 min-w-9 items-center justify-center gap-2 rounded-full px-2.5 text-sm font-medium text-(--muted-foreground) hover:bg-(--surface-hover) hover:text-(--foreground)"
        active-class="app-navigation-link--active"
        :aria-label="t(link.key)"
        :title="t(link.key)"
      >
        <component
          :is="link.icon"
          :size="16"
          aria-hidden="true"
        />
        <span class="app-navigation-label">{{ t(link.key) }}</span>
      </RouterLink>
    </div>
    <RouterLink
      :to="accountLink.to"
      class="app-navigation-link inline-flex size-9 items-center justify-center justify-self-end rounded-full text-(--muted-foreground) hover:bg-(--surface-hover) hover:text-(--foreground)"
      active-class="app-navigation-link--active"
      :aria-label="t(accountLink.key)"
      :title="t(accountLink.key)"
    >
      <component
        :is="accountLink.icon"
        :size="16"
        aria-hidden="true"
      />
    </RouterLink>
  </nav>
  <div :class="{ 'app-route-frame--nav': showNavigation }">
    <RouterView v-slot="{ Component, route: currentRoute }">
      <component
        :is="Component"
        :key="currentRoute.path"
        class="route-page-content"
      />
    </RouterView>
  </div>
</template>

<style>
.app-navigation {
  border-radius: 1.75rem;
  background: color-mix(in oklch, var(--surface) 78%, transparent);
  box-shadow:
    0 0.75rem 2.25rem oklch(0.18 0.02 250 / 0.14),
    inset 0 1px 0 color-mix(in oklch, var(--surface-raised) 72%, transparent);
  backdrop-filter: blur(1.25rem) saturate(135%);
  -webkit-backdrop-filter: blur(1.25rem) saturate(135%);
}

.app-brand-logo {
  display: block;
  width: 1.375rem;
  height: 1.375rem;
  flex: 0 0 auto;
  border-radius: 0.45rem;
  object-fit: cover;
}

.app-navigation-link--active {
  background: color-mix(in oklch, var(--accent) 13%, var(--surface-raised));
  color: var(--foreground);
  box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--accent) 25%, transparent);
}

.app-route-frame--nav {
  padding-top: 5.75rem;
}

.route-page-content { animation: route-page-enter var(--motion-medium) var(--ease-out-expo); }

@keyframes route-page-enter {
  from { opacity: 0; transform: translateY(0.25rem); }
}

@media (max-width: 44rem) {
  .app-brand-name {
    display: none;
  }

}

@media (max-width: 34rem) {
  .app-navigation-label {
    display: none;
  }
}

@media (prefers-reduced-transparency: reduce) {
  .app-navigation {
    background: var(--surface);
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
  }
}

@media (prefers-reduced-motion: reduce) {
  .route-page-content { animation: none; }
}
</style>
