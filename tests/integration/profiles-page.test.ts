import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createMemoryHistory, createRouter } from 'vue-router'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import ProfilesPage from '@/pages/profiles/ui/ProfilesPage.vue'
import { en } from '@/shared/config/i18n/en'

const mocks = vi.hoisted(() => ({
  profiles: [] as Array<{ id: string; name: string; status: 'draft' | 'ready' | 'archived'; revision: number; updatedAtMs: number }>,
  save: vi.fn(),
  archive: vi.fn(),
  restore: vi.fn(),
  invalidate: vi.fn(),
}))

vi.mock('@/entities/interview-profile', () => ({
  profilesQuery: () => ({ queryKey: ['profiles'], queryFn: async () => mocks.profiles }),
  profileGateway: { save: mocks.save, archive: mocks.archive, restore: mocks.restore },
  invalidateProfileQueries: mocks.invalidate,
}))

async function mountPage(stubCreate = false) {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/profiles', component: ProfilesPage },
      { path: '/profiles/:profileId', component: { template: '<main>Profile</main>' } },
    ],
  })
  await router.push('/profiles')
  await router.isReady()
  const wrapper = mount(ProfilesPage, {
    global: {
      plugins: [
        router,
        createI18n({ legacy: false, locale: 'en', messages: { en } }),
        [VueQueryPlugin, { queryClient: new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } }) }],
      ],
      stubs: stubCreate ? {
        ProfileCreateDialog: {
          props: ['error'],
          emits: ['create', 'update:open'],
          template: '<div><button data-testid="create-submit" @click="$emit(\'create\', \'New profile\')">Create now</button><p v-if="error" role="alert">{{ error }}</p></div>',
        },
      } : undefined,
    },
  })
  await flushPromises()
  return { wrapper, router }
}

describe('profiles page live recovery paths', () => {
  beforeEach(() => {
    mocks.profiles = []
    mocks.save.mockReset()
    mocks.archive.mockReset()
    mocks.restore.mockReset()
    mocks.invalidate.mockReset().mockResolvedValue(undefined)
  })

  it('separates an archived profile and restores it before editing', async () => {
    mocks.profiles = [{ id: 'archived-1', name: 'Backend', status: 'archived', revision: 14, updatedAtMs: 1 }]
    mocks.restore.mockResolvedValue({ ...mocks.profiles[0], status: 'draft', revision: 15 })
    const { wrapper, router } = await mountPage()

    expect(wrapper.text()).toContain(en.profiles.archivedDescription)
    await wrapper.findAll('button').find((button) => button.text().includes(en.profiles.restore))!.trigger('click')
    await flushPromises()

    expect(mocks.restore).toHaveBeenCalledWith('archived-1', 14)
    expect(router.currentRoute.value.path).toBe('/profiles/archived-1')
    wrapper.unmount()
  })

  it('shows a profile creation error instead of silently closing or doing nothing', async () => {
    mocks.save.mockRejectedValue(new Error('AUTH_REQUIRED'))
    const { wrapper } = await mountPage(true)

    await wrapper.get('[data-testid="create-submit"]').trigger('click')
    await flushPromises()

    expect(wrapper.get('[role="alert"]').text()).toBe(en.profiles.create.error)
    wrapper.unmount()
  })
})
