import { QueryClient, VueQueryPlugin } from '@tanstack/vue-query'
import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createMemoryHistory, createRouter } from 'vue-router'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import ProfilePage from '@/pages/profiles/ui/ProfilePage.vue'
import { en } from '@/shared/config/i18n/en'

const mocks = vi.hoisted(() => ({
  profile: {} as Record<string, unknown>,
  getProfile: vi.fn(),
  save: vi.fn(),
  importSource: vi.fn(),
  invalidate: vi.fn(),
  parseVacancy: vi.fn(),
}))

vi.mock('@/entities/interview-profile', () => ({
  profileQuery: (profileId: string) => ({
    queryKey: ['profiles', profileId],
    queryFn: () => mocks.getProfile(),
    enabled: Boolean(profileId),
  }),
  productClient: {
    listModels: async () => [
      { id: 'response-fast', kind: 'response', name: 'Response Fast', availability: 'available', languages: [], capabilities: [] },
      { id: 'transcribe-fast', kind: 'transcription', name: 'Transcribe Fast', availability: 'available', languages: [], capabilities: [] },
    ],
    parseVacancy: mocks.parseVacancy,
  },
  profileGateway: { save: mocks.save, importSource: mocks.importSource },
  invalidateProfileQueries: mocks.invalidate,
}))

function baseProfile() {
  return {
    id: 'profile-backend',
    name: 'Backend interview',
    status: 'draft',
    manualContext: '',
    revision: 1,
    createdAtMs: 1,
    updatedAtMs: 1,
    vacancy: null,
    sources: [],
    modelConfiguration: null,
  }
}

async function mountPage(returnToMeeting = false) {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/profiles', component: { template: '<div>Profiles</div>' } },
      { path: '/profiles/:profileId', component: ProfilePage },
      { path: '/meetings/new', component: { template: '<div>Meeting setup</div>' } },
    ],
  })
  await router.push({ path: '/profiles/profile-backend', query: returnToMeeting ? { returnTo: 'meeting' } : {} })
  await router.isReady()
  const wrapper = mount(ProfilePage, {
    global: {
      plugins: [
        router,
        createI18n({ legacy: false, locale: 'en', messages: { en } }),
        [VueQueryPlugin, { queryClient: new QueryClient({ defaultOptions: { queries: { retry: false } } }) }],
      ],
    },
  })
  await flushPromises()
  return { wrapper, router }
}

describe('profile page', () => {
  beforeEach(() => {
    mocks.profile = baseProfile()
    mocks.getProfile.mockReset().mockImplementation(async () => JSON.parse(JSON.stringify(mocks.profile)))
    mocks.invalidate.mockReset().mockResolvedValue(undefined)
    mocks.save.mockReset().mockImplementation(async (input) => {
      mocks.profile = {
        ...mocks.profile,
        ...input,
        revision: Number(mocks.profile.revision) + 1,
        vacancy: input.vacancy ? { id: 'vacancy-1', ...input.vacancy } : null,
        modelConfiguration: input.modelConfiguration ? { id: 'models-1', ...input.modelConfiguration } : null,
      }
      return JSON.parse(JSON.stringify(mocks.profile))
    })
    mocks.importSource.mockReset().mockImplementation(async () => {
      mocks.profile = {
        ...mocks.profile,
        revision: Number(mocks.profile.revision) + 1,
        sources: [{
          id: 'source-1',
          kind: 'resume',
          displayName: 'Resume.pdf',
          mimeType: 'application/pdf',
          extractedFacts: [],
          contentStatus: 'allowed',
          redactionSummary: null,
          checksum: 'checksum',
        }],
      }
      return JSON.parse(JSON.stringify(mocks.profile))
    })
    mocks.parseVacancy.mockReset().mockResolvedValue({
      title: 'Senior Backend Engineer',
      company: 'Example',
      responsibilities: ['Own services'],
      requirements: ['TypeScript'],
      provenance: { fixtureId: 'vacancy-fixture', extractionModelId: 'extractor', extractedAt: '2026-08-05T10:00:00Z' },
    })
  })

  it('loads the profile from the current route id and exposes a working back action', async () => {
    const { wrapper, router } = await mountPage()

    expect(mocks.getProfile).toHaveBeenCalledOnce()
    expect(wrapper.get('input').element.value).toBe('Backend interview')
    await wrapper.get('button').trigger('click')
    await flushPromises()
    expect(router.currentRoute.value.path).toBe('/profiles')
    wrapper.unmount()
  })

  it('persists and restores every editable profile field', async () => {
    const { wrapper } = await mountPage()
    const name = wrapper.get('input')
    await name.setValue('Platform interview')
    const textareas = wrapper.findAll('textarea')
    await textareas[0]!.setValue('https://jobs.example.test/platform')
    await wrapper.findAll('button').find((button) => button.text().includes('Extract'))!.trigger('click')
    await flushPromises()

    const vacancyInputs = wrapper.findAll('input')
    await vacancyInputs[1]!.setValue('Staff Platform Engineer')
    await vacancyInputs[2]!.setValue('Acme Platform')
    const updatedTextareas = wrapper.findAll('textarea')
    await updatedTextareas[1]!.setValue('Lead reliability\nImprove tooling')
    await updatedTextareas[2]!.setValue('Distributed systems\nTypeScript')
    await updatedTextareas[3]!.setValue('Led a migration and reduced incidents by 40%.')
    await wrapper.findAll('button').find((button) => button.text().includes('Confirm fields'))!.trigger('click')

    const selects = wrapper.findAll('select')
    await selects[2]!.setValue('ru')
    await selects[3]!.setValue('detailed')
    await flushPromises()

    expect(wrapper.text()).toContain('Unsaved changes')
    await wrapper.findAll('button').find((button) => button.text() === 'Save')!.trigger('click')
    await flushPromises()

    expect(mocks.save).toHaveBeenCalledWith(expect.objectContaining({
      id: 'profile-backend',
      expectedRevision: 1,
      name: 'Platform interview',
      manualContext: 'Led a migration and reduced incidents by 40%.',
      vacancy: expect.objectContaining({
        roleTitle: 'Staff Platform Engineer',
        companyContext: 'Acme Platform',
        responsibilities: ['Lead reliability', 'Improve tooling'],
        requirements: ['Distributed systems', 'TypeScript'],
        reviewStatus: 'confirmed',
      }),
      modelConfiguration: expect.objectContaining({
        responseModelId: 'response-fast',
        transcriptionModelId: 'transcribe-fast',
        translationLanguage: 'ru',
        answerDepth: 'detailed',
      }),
    }))
    expect(wrapper.get('input').element.value).toBe('Platform interview')
    expect(wrapper.findAll('textarea')[3]!.element.value).toContain('reduced incidents')
    expect(wrapper.text()).toContain('Saved')
    wrapper.unmount()
  })

  it('saves pending edits before importing a source and uses the new revision', async () => {
    const { wrapper } = await mountPage()
    await wrapper.get('input').setValue('Edited before import')
    await wrapper.findAll('textarea')[1]!.setValue('Context that must not be lost')

    await wrapper.findAll('button').find((button) => button.text().includes('Add sample resume'))!.trigger('click')
    await flushPromises()

    expect(mocks.save).toHaveBeenCalledWith(expect.objectContaining({
      expectedRevision: 1,
      name: 'Edited before import',
      manualContext: 'Context that must not be lost',
    }))
    expect(mocks.importSource).toHaveBeenCalledWith(expect.objectContaining({
      profileId: 'profile-backend',
      expectedRevision: 2,
      fixtureId: 'resume-product-engineer',
    }))
    expect(wrapper.get('input').element.value).toBe('Edited before import')
    expect(wrapper.text()).toContain('Resume.pdf')
    wrapper.unmount()
  })

  it('keeps the saved revision when an import fails', async () => {
    mocks.importSource.mockRejectedValueOnce(new Error('Import unavailable'))
    const { wrapper } = await mountPage()
    await wrapper.get('input').setValue('Saved before failed import')

    await wrapper.findAll('button').find((button) => button.text().includes('Add sample resume'))!.trigger('click')
    await flushPromises()
    expect(wrapper.text()).toContain('Saved profile changes were preserved')

    await wrapper.get('input').setValue('Retry after failed import')
    await wrapper.findAll('button').find((button) => button.text() === 'Save')!.trigger('click')
    await flushPromises()

    expect(mocks.save).toHaveBeenLastCalledWith(expect.objectContaining({
      expectedRevision: 2,
      name: 'Retry after failed import',
    }))
    wrapper.unmount()
  })

  it('saves a ready profile and returns directly to meeting setup', async () => {
    mocks.profile = {
      ...baseProfile(),
      modelConfiguration: {
        id: 'models-1',
        responseModelId: 'response-fast',
        transcriptionModelId: 'transcribe-fast',
        translationLanguage: 'none',
        answerDepth: 'balanced',
        questionConfidenceThreshold: 0.72,
        processingBoundaryId: 'mock-local-boundary',
      },
    }
    const { wrapper, router } = await mountPage(true)
    await wrapper.get('input').setValue('Ready profile')

    await wrapper.findAll('button').find((button) => button.text().includes('Save and continue'))!.trigger('click')
    await flushPromises()

    expect(mocks.save).toHaveBeenCalledOnce()
    expect(router.currentRoute.value.path).toBe('/meetings/new')
    wrapper.unmount()
  })
})
