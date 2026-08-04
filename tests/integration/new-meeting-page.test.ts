import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { defineComponent } from 'vue'
import { createMemoryHistory, createRouter } from 'vue-router'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import NewMeetingPage from '@/pages/meeting/ui/NewMeetingPage.vue'
import { en } from '@/shared/config/i18n/en'

const state = vi.hoisted(() => ({
  profiles: [] as Array<{ id: string; name: string; status: 'draft' | 'ready' | 'archived'; revision: number; updatedAtMs: number }>,
  details: new Map<string, unknown>(),
  models: [] as Array<{ id: string; kind: string; name: string; description: string; availability: string; languages: string[]; capabilities: string[] }>,
  displays: [] as Array<{ displayId: number; label: string; width: number; height: number; backingScale: number; isPrimary: boolean }>,
  gate: { allowed: false, reasonCodes: [] as string[] },
  start: vi.fn(),
  sendEnvelope: vi.fn(),
}))

vi.mock('@/entities/interview-profile', () => ({
  profilesQuery: () => ({ queryKey: ['profiles'], queryFn: async () => state.profiles }),
  profileQuery: (profileId: string) => ({
    queryKey: ['profiles', profileId],
    queryFn: async () => state.details.get(profileId),
    enabled: Boolean(profileId),
  }),
  productClient: { listModels: async () => state.models },
}))

vi.mock('@/entities/meeting', () => ({
  meetingGateway: {
    listDisplays: async () => state.displays,
    evaluateGate: async () => state.gate,
    start: state.start,
  },
}))

vi.mock('@/app/providers/meeting-socket', () => ({ sendMeetingEnvelope: state.sendEnvelope }))

function createHarness() {
  const Target = defineComponent({ template: '<div data-testid="target" />' })
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/meetings/new', component: NewMeetingPage },
      { path: '/profiles/:profileId', component: Target },
      { path: '/meetings/:meetingId', component: Target },
    ],
  })
  const i18n = createI18n({ legacy: false, locale: 'en', messages: { en } })
  return { router, i18n }
}

async function mountPage() {
  const { router, i18n } = createHarness()
  await router.push('/meetings/new')
  await router.isReady()
  const wrapper = mount(NewMeetingPage, {
    global: { plugins: [router, i18n, [VueQueryPlugin, { queryClient: new QueryClient() }]] },
  })
  await flushPromises()
  await flushPromises()
  return { wrapper, router }
}

function prepareReadyMeeting() {
  state.profiles = [{ id: 'ready-profile', name: 'Backend', status: 'ready', revision: 4, updatedAtMs: 1 }]
  state.details.set('ready-profile', {
    id: 'ready-profile', name: 'Backend', status: 'ready', revision: 4, updatedAtMs: 1,
    modelConfiguration: { responseModelId: 'response-1', transcriptionModelId: 'stt-1' },
  })
  state.models = [
    { id: 'response-1', kind: 'response', name: 'Response', description: '', availability: 'available', languages: [], capabilities: [] },
    { id: 'stt-1', kind: 'transcription', name: 'STT', description: '', availability: 'available', languages: [], capabilities: [] },
  ]
  state.displays = [{ displayId: 7, label: 'Main', width: 1728, height: 1117, backingScale: 2, isPrimary: true }]
  state.gate = { allowed: true, reasonCodes: [] }
}

describe('new meeting page', () => {
  beforeEach(() => {
    state.profiles = []
    state.details.clear()
    state.models = []
    state.displays = []
    state.gate = { allowed: false, reasonCodes: [] }
    state.start.mockReset()
    state.sendEnvelope.mockReset()
  })

  it('mounts when no profile or display has loaded yet', async () => {
    const { wrapper } = await mountPage()
    expect(wrapper.get('h1').text()).toBe(en.meeting.new.title)
    wrapper.unmount()
  })

  it('starts a meeting with a ready profile after all checks finish', async () => {
    prepareReadyMeeting()
    state.start.mockResolvedValue({
      id: 'meeting-1', profileId: 'ready-profile', profileRevision: 4, mode: 'standard_lab',
    })

    const { wrapper, router } = await mountPage()
    const button = wrapper.get('button')
    expect(button.text()).toBe(en.meeting.new.start)
    expect(button.attributes('disabled')).toBeUndefined()

    await button.trigger('click')
    await flushPromises()

    expect(state.start).toHaveBeenCalledWith(expect.objectContaining({
      profileId: 'ready-profile',
      profileRevision: 4,
      captureConfigurationId: 'display-7-vad-0.18',
    }))
    expect(router.currentRoute.value.path).toBe('/meetings/meeting-1')
    wrapper.unmount()
  })

  it('ignores a rapid second start click while the first native call is pending', async () => {
    prepareReadyMeeting()
    let resolveStart!: (value: unknown) => void
    state.start.mockImplementation(() => new Promise((resolve) => { resolveStart = resolve }))
    const { wrapper } = await mountPage()
    const button = wrapper.get('button')

    await button.trigger('click')
    await button.trigger('click')
    expect(state.start).toHaveBeenCalledOnce()

    resolveStart({ id: 'meeting-1', profileId: 'ready-profile', profileRevision: 4, mode: 'standard_lab' })
    await flushPromises()
    wrapper.unmount()
  })

  it('opens the running meeting when the optional socket notification fails', async () => {
    prepareReadyMeeting()
    state.start.mockResolvedValue({
      id: 'meeting-1', profileId: 'ready-profile', profileRevision: 4, mode: 'standard_lab',
    })
    state.sendEnvelope.mockImplementation(() => { throw new Error('offline') })

    const { wrapper, router } = await mountPage()
    await wrapper.get('button').trigger('click')
    await flushPromises()

    expect(router.currentRoute.value.path).toBe('/meetings/meeting-1')
    expect(wrapper.find('[role="alert"]').exists()).toBe(false)
    wrapper.unmount()
  })

  it('takes a draft profile to configuration instead of attempting to start', async () => {
    state.profiles = [{ id: 'draft-profile', name: 'Frontend', status: 'draft', revision: 1, updatedAtMs: 1 }]
    state.details.set('draft-profile', { id: 'draft-profile', modelConfiguration: null })
    state.displays = [{ displayId: 1, label: 'Main', width: 1440, height: 900, backingScale: 2, isPrimary: true }]
    state.gate = { allowed: true, reasonCodes: [] }

    const { wrapper, router } = await mountPage()
    const button = wrapper.get('button')
    expect(button.text()).toBe(en.meeting.new.configureProfile)

    await button.trigger('click')
    await flushPromises()

    expect(state.start).not.toHaveBeenCalled()
    expect(router.currentRoute.value.path).toBe('/profiles/draft-profile')
    wrapper.unmount()
  })
})
