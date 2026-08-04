import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { describe, expect, it, vi } from 'vitest'

import ModelConfigurationSection from '@/pages/profiles/ui/ModelConfigurationSection.vue'
import { en } from '@/shared/config/i18n/en'

vi.mock('@/entities/interview-profile', () => ({
  productClient: {
    listModels: async () => [
      { id: 'response-disabled', kind: 'response', name: 'Disabled', availability: 'disabled' },
      { id: 'response-default', kind: 'response', name: 'Response', availability: 'available' },
      { id: 'response-other', kind: 'response', name: 'Other response', availability: 'available' },
      { id: 'transcription-default', kind: 'transcription', name: 'Transcription', availability: 'available' },
    ],
  },
}))

describe('model configuration defaults', () => {
  it('emits available defaults and still allows changing them', async () => {
    const wrapper = mount(ModelConfigurationSection, {
      props: { configuration: null },
      global: {
        plugins: [
          createI18n({ legacy: false, locale: 'en', messages: { en } }),
          [VueQueryPlugin, { queryClient: new QueryClient() }],
        ],
      },
    })
    await flushPromises()

    expect(wrapper.emitted('update')?.[0]?.[0]).toMatchObject({
      responseModelId: 'response-default',
      transcriptionModelId: 'transcription-default',
    })

    await wrapper.findAll('select')[0]!.setValue('response-other')
    expect(wrapper.emitted('update')?.at(-1)?.[0]).toMatchObject({ responseModelId: 'response-other' })
    wrapper.unmount()
  })
})
