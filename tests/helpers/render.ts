import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { createTestingPinia } from '@pinia/testing'
import { mount, type ComponentMountingOptions } from '@vue/test-utils'
import type { Component } from 'vue'
import { vi } from 'vitest'

export function render<T extends Component>(component: T, options: ComponentMountingOptions<T> = {}) {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 } },
  })

  return mount(component, {
    ...options,
    global: {
      ...options.global,
      plugins: [
        createTestingPinia({ createSpy: vi.fn }),
        [VueQueryPlugin, { queryClient }],
        ...(options.global?.plugins ?? []),
      ],
    },
  })
}
