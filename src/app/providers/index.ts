import type { App } from 'vue'
import { VueQueryPlugin } from '@tanstack/vue-query'

import { router } from '@/app/routes'
import { i18n } from '@/app/providers/i18n'
import { pinia } from '@/app/providers/pinia'
import { createQueryClient } from '@/app/providers/query'
import { initializeAppearanceProvider } from '@/app/providers/appearance'

export function installProviders(app: App): void {
  initializeAppearanceProvider()
  app.use(pinia)
  app.use(i18n)
  app.use(VueQueryPlugin, { queryClient: createQueryClient() })
  app.use(router)
}
