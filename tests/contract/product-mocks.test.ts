import { afterEach, describe, expect, it, vi } from 'vitest'
import { createProductHandlers } from '@/mocks/handlers/product'
import { createPackagedFetch } from '@/mocks/packaged'
import { createProductScenario } from '@/mocks/scenarios/product'
import { server } from '../setup'

async function json(response: Response) {
  return response.json() as Promise<Record<string, unknown>>
}

describe('product mock contract', () => {
  afterEach(() => vi.restoreAllMocks())

  it('returns a deterministic catalog and filters it by model kind', async () => {
    const product = createProductScenario()
    const all = await product.runtime.resolve({ method: 'GET', url: '/api/models' })
    const transcription = await product.runtime.resolve({ method: 'GET', url: '/api/models?kind=transcription' })

    expect(all).toMatchObject({ status: 200, body: { requestId: 'req-models-1' } })
    expect(transcription).toMatchObject({
      status: 200,
      body: {
        requestId: 'req-models-1',
        data: [
          { id: 'openai/whisper-large-v3-turbo', kind: 'transcription' },
          { id: 'nvidia/parakeet-tdt-0.6b-v3', kind: 'transcription' },
        ],
      },
    })
    expect(await product.runtime.resolve({ method: 'GET', url: '/api/models?kind=video' })).toMatchObject({
      status: 400,
      body: { error: { code: 'MODEL_KIND_INVALID', retryable: false }, requestId: 'req-models-1' },
    })
  })

  it('reports an unavailable selected model without substituting another ID', async () => {
    const product = createProductScenario()
    const response = await product.runtime.resolve({
      method: 'POST',
      url: '/api/models/select',
      body: { id: 'response/deep-reasoning-preview' },
    })

    expect(response).toMatchObject({
      status: 409,
      body: {
        error: { code: 'MODEL_UNAVAILABLE', retryable: true, field: 'id' },
        requestId: 'req-model-select-1',
      },
    })
    expect(JSON.stringify(response)).not.toContain('response/concise-v1')
  })

  it('parses only an allowlisted vacancy URL locally and always requires review', async () => {
    const outbound = vi.spyOn(globalThis, 'fetch')
    const product = createProductScenario()
    const response = await product.runtime.resolve({
      method: 'POST',
      url: '/api/vacancies/parse',
      body: { source: { kind: 'url', url: 'https://jobs.example.test/senior-frontend-engineer' } },
    })

    expect(response).toMatchObject({
      status: 200,
      body: {
        requestId: 'req-vacancy-1',
        data: {
          title: 'Senior Frontend Engineer',
          needsReview: true,
          provenance: {
            fixtureId: 'vacancy-senior-frontend-1',
            extractionModelId: 'mock/vacancy-extractor-v1',
            extractedAt: '2026-08-04T00:00:00.000Z',
          },
        },
      },
    })
    expect(outbound).not.toHaveBeenCalled()

    const unsupported = await product.runtime.resolve({
      method: 'POST',
      url: '/api/vacancies/parse',
      body: { source: { kind: 'url', url: 'https://private.example.com/jobs/1' } },
    })
    expect(unsupported).toMatchObject({
      status: 422,
      body: { error: { code: 'VACANCY_SOURCE_UNAVAILABLE', field: 'source.url' } },
    })
  })

  it('enforces one vacancy source, the text bound, and mutation field strictness', async () => {
    const product = createProductScenario()
    const invalid = await product.runtime.resolve({
      method: 'POST',
      url: '/api/vacancies/parse',
      body: { source: { kind: 'text', text: 'Vue role', url: 'https://jobs.example.test/senior-frontend-engineer' } },
    })
    const tooLarge = await product.runtime.resolve({
      method: 'POST',
      url: '/api/vacancies/parse',
      body: { source: { kind: 'text', text: 'x'.repeat(10_001) } },
    })

    expect(invalid).toMatchObject({ status: 400, body: { error: { code: 'REQUEST_INVALID' } } })
    expect(tooLarge).toMatchObject({ status: 413, body: { error: { code: 'VACANCY_TEXT_TOO_LARGE' } } })
  })

  it('extracts stable editable facts only from approved or redacted source IDs', async () => {
    const product = createProductScenario()
    const approved = await product.runtime.resolve({
      method: 'POST',
      url: '/api/profile-materials/extract',
      body: { sourceId: 'source-project-approved', kind: 'project', contentStatus: 'allowed' },
    })
    expect(approved).toMatchObject({
      status: 200,
      body: {
        requestId: 'req-material-1',
        data: {
          needsReview: true,
          facts: [{ id: 'fact-project-result-1', sourceRange: 'page:1' }],
          provenance: { sourceId: 'source-project-approved', extractorRevision: 'mock-fixture' },
        },
      },
    })

    for (const body of [
      { sourceId: 'source-project-pending', kind: 'project', contentStatus: 'allowed' },
      { sourceId: 'source-foreign', kind: 'project', contentStatus: 'allowed' },
      { sourceId: 'source-project-approved', kind: 'project', contentStatus: 'pending' },
      { sourceId: 'source-project-approved', kind: 'project', contentStatus: 'allowed', path: '/tmp/resume.pdf' },
    ]) {
      expect(await product.runtime.resolve({ method: 'POST', url: '/api/profile-materials/extract', body })).toMatchObject({
        status: expect.any(Number),
        body: { error: { code: expect.stringMatching(/PROFILE_SOURCE_NOT_ALLOWED|REQUEST_INVALID/) } },
      })
    }
  })

  it('returns identical DTOs through packaged and MSW adapters', async () => {
    const product = createProductScenario()
    const packaged = createPackagedFetch(product.runtime)
    server.use(...createProductHandlers(product))

    const [packagedResponse, mswResponse] = await Promise.all([
      packaged('https://app.local/api/models?kind=translation'),
      fetch('https://app.local/api/models?kind=translation'),
    ])
    expect(await json(mswResponse)).toEqual(await json(packagedResponse))
  })
})
