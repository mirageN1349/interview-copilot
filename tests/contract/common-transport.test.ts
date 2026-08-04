import { describe, expect, it, vi } from 'vitest'

import { parseApiEnvelope, readBoundedString } from '@/shared/api/contracts/common'
import { createHttpGateway } from '@/shared/api/http'
import { createNativeGateway } from '@/shared/api/native'
import { redact } from '@/shared/lib/redact'
import { createScenarioRuntime } from '@/mocks/scenarios/runtime'
import { createPackagedFetch, mockTransportTarget } from '@/mocks/packaged'
import { scenarioCorpus } from '../fixtures/scenario-corpus'

describe('common transport', () => {
  it('never uses a service worker inside Tauri', () => {
    expect(mockTransportTarget(true, true)).toBe('packaged')
    expect(mockTransportTarget(false, true)).toBe('browser')
    expect(mockTransportTarget(false, false)).toBeNull()
  })

  it('bounds DTO strings and parses stable envelopes', () => {
    expect(readBoundedString('ok', 'name', 2)).toBe('ok')
    expect(() => readBoundedString('long', 'name', 2)).toThrow('name')
    expect(parseApiEnvelope({ data: { ok: true }, requestId: 'req-1' })).toEqual({
      data: { ok: true },
      requestId: 'req-1',
    })
    expect(() => parseApiEnvelope({ error: { code: 'BAD' }, requestId: 'req-1' })).toThrow()
  })

  it('normalizes native failures to a public command error', async () => {
    const native = createNativeGateway(async () => {
      throw { code: 'DISPLAY_NOT_FOUND', message: 'Display is unavailable', retryable: true, privatePath: '/tmp/a' }
    })

    await expect(native.invoke('displays_list')).rejects.toEqual({
      code: 'DISPLAY_NOT_FOUND',
      message: 'Display is unavailable',
      retryable: true,
    })
  })

  it('keeps packaged scenarios byte-for-byte equivalent and rejects unknown routes', async () => {
    const runtime = createScenarioRuntime(scenarioCorpus)
    const packagedFetch = createPackagedFetch(runtime)

    const direct = await runtime.resolve({ method: 'GET', url: 'https://app.local/api/ping' })
    const response = await packagedFetch('/api/ping')
    expect(await response.json()).toEqual(direct.body)
    await expect(packagedFetch('/api/unknown')).rejects.toThrow('Unhandled mock request')
  })

  it('never falls through to an external host', async () => {
    const fallback = vi.fn()
    const gateway = createHttpGateway({ fetch: fallback as typeof fetch })

    await expect(gateway.get('https://example.com/private')).rejects.toThrow('same-origin')
    expect(fallback).not.toHaveBeenCalled()
  })

  it('redacts secrets, addresses and local paths without mutating input', () => {
    const input = { token: 'secret', email: 'person@example.com', nested: { path: '/Users/a/resume.pdf', keep: 'yes' } }
    expect(redact(input)).toEqual({ token: '[REDACTED]', email: '[REDACTED]', nested: { path: '[REDACTED]', keep: 'yes' } })
    expect(input.token).toBe('secret')
  })
})
