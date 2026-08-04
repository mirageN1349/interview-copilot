import { describe, expect, it, vi } from 'vitest'

import {
  createProfileGateway,
  type ProfileDetails,
} from '@/entities/interview-profile'
import { evaluateProfileReadiness } from '@/pages/profiles/model/profile-readiness'

const readyProfile: ProfileDetails = {
  id: 'profile-frontend',
  name: 'Frontend role',
  status: 'ready',
  manualContext: 'I build accessible Vue products.',
  revision: 4,
  createdAtMs: Date.parse('2026-08-04T10:00:00.000Z'),
  updatedAtMs: Date.parse('2026-08-04T10:05:00.000Z'),
  vacancy: {
    id: 'vacancy-frontend',
    sourceKind: 'url',
    sourceValue: 'https://jobs.example.test/frontend',
    roleTitle: 'Senior Frontend Engineer',
    companyContext: 'Synthetic product company',
    responsibilities: ['Build product interfaces'],
    requirements: ['Vue 3'],
    reviewStatus: 'confirmed',
    provenance: { fixtureId: 'vacancy-frontend', extractionModelId: 'fixture-extractor', extractedAtMs: Date.parse('2026-08-04T10:01:00.000Z') },
  },
  sources: [{
    id: 'source-resume-frontend',
    kind: 'resume',
    displayName: 'Frontend resume',
    mimeType: 'application/pdf',
    extractedFacts: [{ id: 'fact-vue', category: 'skill', text: 'Vue 3', sourceRange: 'page:1' }],
    contentStatus: 'allowed',
    redactionSummary: null,
    checksum: 'a'.repeat(64),
  }],
  modelConfiguration: {
    id: 'config-frontend',
    responseModelId: 'mock/response-balanced',
    transcriptionModelId: 'openai/whisper-large-v3-turbo',
    translationLanguage: 'none',
    answerDepth: 'balanced',
    questionConfidenceThreshold: 0.72,
    processingBoundaryId: 'mock-local-boundary',
  },
}

describe('profile preparation', () => {
  it('allows no vacancy but requires review when one is provided', () => {
    expect(evaluateProfileReadiness(readyProfile, new Set([
      'mock/response-balanced',
      'openai/whisper-large-v3-turbo',
    ]))).toEqual({ ready: true, reasons: [] })
    expect(evaluateProfileReadiness({ ...readyProfile, sources: [] }, new Set([
      'mock/response-balanced',
      'openai/whisper-large-v3-turbo',
    ]))).toEqual({ ready: true, reasons: [] })
    expect(evaluateProfileReadiness({ ...readyProfile, vacancy: null }, new Set([
      'mock/response-balanced',
      'openai/whisper-large-v3-turbo',
    ]))).toEqual({ ready: true, reasons: [] })

    const incomplete: ProfileDetails = {
      ...readyProfile,
      vacancy: { ...readyProfile.vacancy!, reviewStatus: 'needs_review' },
      sources: [{ ...readyProfile.sources[0]!, contentStatus: 'pending' }],
      modelConfiguration: { ...readyProfile.modelConfiguration!, responseModelId: 'unavailable/model' },
    }

    expect(evaluateProfileReadiness(incomplete, new Set([
      'openai/whisper-large-v3-turbo',
    ]))).toEqual({
      ready: false,
      reasons: ['vacancy_review', 'source_review', 'response_model_unavailable'],
    })
  })

  it('loads each profile by its own id without copying or merging source state', async () => {
    const backend = new Map<string, ProfileDetails>([
      [readyProfile.id, readyProfile],
      ['profile-backend', {
        ...readyProfile,
        id: 'profile-backend',
        name: 'Backend role',
        manualContext: 'I build Rust services.',
        sources: [{ ...readyProfile.sources[0]!, id: 'source-resume-backend', extractedFacts: [{ id: 'fact-rust', category: 'skill', text: 'Rust', sourceRange: 'page:1' }] }],
      }],
    ])
    const invoke = vi.fn(async <T>(_command: string, args?: Record<string, unknown>) => backend.get(String(args?.profileId)) as T)
    const gateway = createProfileGateway({ invoke })

    const frontend = await gateway.get('profile-frontend')
    const backendProfile = await gateway.get('profile-backend')

    expect(invoke).toHaveBeenNthCalledWith(1, 'profile_get', { profileId: 'profile-frontend' })
    expect(invoke).toHaveBeenNthCalledWith(2, 'profile_get', { profileId: 'profile-backend' })
    expect(frontend.sources.map((source) => source.id)).toEqual(['source-resume-frontend'])
    expect(backendProfile.sources.map((source) => source.id)).toEqual(['source-resume-backend'])
  })
})
