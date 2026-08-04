import { describe, expect, it, vi } from 'vitest'

import { createHttpGateway } from '@/shared/api/http'
import { decodeEnvelope, encodeEnvelope, ProtocolError } from '@/shared/api/ws/protocol'
import { assertPublicCopy } from '@/shared/lib/public-copy'
import { evaluateProfileReadiness } from '@/pages/profiles/model/profile-readiness'
import type { ProfileDetails } from '@/entities/interview-profile'

describe('final security boundaries', () => {
  it('blocks network egress outside same-origin mock paths', async () => {
    const outbound = vi.fn()
    const gateway = createHttpGateway({ fetch: outbound as typeof fetch })
    await expect(gateway.get('https://example.test/private')).rejects.toThrow('same-origin')
    await expect(gateway.get('//example.test/private')).rejects.toThrow('same-origin')
    expect(outbound).not.toHaveBeenCalled()
  })

  it('accepts only opaque vetted artifact IDs in chat frames', () => {
    const safe = encodeEnvelope({
      v: 1, id: 'command-1', type: 'chat.send', sentAt: '2026-08-04T10:00:00.000Z',
      meetingId: 'meeting-1', launchPolicyId: 'policy-1',
      payload: { thread: 'side', content: 'Review this', artifactIds: ['artifact-1'], contextGeneration: 0 },
    })
    expect(decodeEnvelope(safe).payload.artifactIds).toEqual(['artifact-1'])
    expect(() => decodeEnvelope(JSON.stringify({
      ...JSON.parse(safe), payload: { thread: 'side', content: 'Review', artifactIds: ['artifact-1'], contextGeneration: 0, rawPath: '/tmp/private.png' },
    }))).toThrowError(ProtocolError)
  })

  it('never silently replaces an unavailable selected model', () => {
    const profile = {
      name: 'Synthetic profile',
      vacancy: { reviewStatus: 'confirmed' },
      sources: [{ contentStatus: 'allowed' }],
      modelConfiguration: { responseModelId: 'selected-unavailable', transcriptionModelId: 'whisper', translationLanguage: null, answerDepth: 'balanced' },
    } as unknown as ProfileDetails
    expect(evaluateProfileReadiness(profile, new Set(['other-model', 'whisper']))).toMatchObject({
      ready: false, reasons: ['response_model_unavailable'],
    })
    expect(profile.modelConfiguration?.responseModelId).toBe('selected-unavailable')
  })

  it('rejects restricted operational terminology from ordinary copy', () => {
    expect(assertPublicCopy('Check your setup and try again.')).toBe('Check your setup and try again.')
    expect(() => assertPublicCopy('Capture matrix row failed')).toThrow('RESTRICTED_COPY')
  })
})
