import { createScenarioRuntime, type ScenarioDefinition, type ScenarioResponse } from './runtime'

export type ModelKind = 'response' | 'transcription' | 'translation'
export type ProductModel = {
  id: string
  kind: ModelKind
  name: string
  description: string
  availability: 'available' | 'unavailable'
  languages: string[]
  capabilities: string[]
}

const MODEL_CATALOG: ProductModel[] = [
  {
    id: 'response/concise-v1',
    kind: 'response',
    name: 'Concise Assistant',
    description: 'Fast structured interview answers',
    availability: 'available',
    languages: ['en', 'ru'],
    capabilities: ['streaming', 'code', 'system-design'],
  },
  {
    id: 'response/deep-reasoning-preview',
    kind: 'response',
    name: 'Deep Reasoning Preview',
    description: 'Detailed reasoning preview',
    availability: 'unavailable',
    languages: ['en', 'ru'],
    capabilities: ['code', 'system-design'],
  },
  {
    id: 'openai/whisper-large-v3-turbo',
    kind: 'transcription',
    name: 'Whisper Large v3 Turbo',
    description: 'Multilingual transcription',
    availability: 'available',
    languages: ['auto', 'en', 'ru'],
    capabilities: ['streaming', 'timestamps'],
  },
  {
    id: 'nvidia/parakeet-tdt-0.6b-v3',
    kind: 'transcription',
    name: 'Parakeet TDT 0.6B v3',
    description: 'Multilingual transcription with punctuation',
    availability: 'available',
    languages: ['en', 'ru', 'uk'],
    capabilities: ['timestamps', 'punctuation'],
  },
  {
    id: 'mock/translation-v1',
    kind: 'translation',
    name: 'Interview Translation',
    description: 'Fixture translation for interview transcripts',
    availability: 'available',
    languages: ['en', 'ru'],
    capabilities: ['streaming'],
  },
]

const VACANCY_URL = 'https://jobs.example.test/senior-frontend-engineer'
const EXTRACTED_AT = '2026-08-04T00:00:00.000Z'
const MATERIALS = {
  'source-resume-approved': {
    kind: 'resume',
    contentStatus: 'allowed',
    facts: [{ id: 'fact-resume-role-1', category: 'experience', text: 'Led frontend delivery for a product team', sourceRange: 'page:1' }],
  },
  'source-project-approved': {
    kind: 'project',
    contentStatus: 'allowed',
    facts: [{ id: 'fact-project-result-1', category: 'project_result', text: 'Reduced page load time by 30%', sourceRange: 'page:1' }],
  },
  'source-project-redacted': {
    kind: 'project',
    contentStatus: 'redacted',
    facts: [{ id: 'fact-project-scope-1', category: 'project_scope', text: 'Improved a customer-facing application', sourceRange: 'section:summary' }],
  },
  'source-project-pending': { kind: 'project', contentStatus: 'pending', facts: [] },
} as const

function error(status: number, requestId: string, code: string, message: string, field?: string): ScenarioResponse {
  return { status, body: { error: { code, message, retryable: code === 'MODEL_UNAVAILABLE', ...(field ? { field } : {}) }, requestId } }
}

function exactObject(value: unknown, keys: string[]): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
    && Object.keys(value).length === keys.length
    && keys.every((key) => Object.hasOwn(value, key))
}

function vacancyData(sourceLabel: string, fixtureId: string) {
  return {
    title: 'Senior Frontend Engineer',
    company: 'Example Product',
    responsibilities: ['Build product interfaces'],
    requirements: ['Vue 3', 'TypeScript'],
    summary: 'Frontend role in a product team',
    confidence: 0.91,
    needsReview: true,
    sourceLabel,
    provenance: { fixtureId, extractionModelId: 'mock/vacancy-extractor-v1', extractedAt: EXTRACTED_AT },
  }
}

export function createProductScenario() {
  const definitions: ScenarioDefinition[] = [
    {
      method: 'GET',
      path: '/api/models',
      resolve(request) {
        const kind = new URL(request.url, 'https://app.local').searchParams.get('kind')
        if (kind && !['response', 'transcription', 'translation'].includes(kind)) {
          return error(400, 'req-models-1', 'MODEL_KIND_INVALID', 'Model kind is not supported.', 'kind')
        }
        const data = kind ? MODEL_CATALOG.filter((model) => model.kind === kind) : MODEL_CATALOG
        return { status: 200, body: { data: structuredClone(data), requestId: 'req-models-1' } }
      },
    },
    {
      method: 'POST',
      path: '/api/models/select',
      resolve(request) {
        if (!exactObject(request.body, ['id']) || typeof request.body.id !== 'string') {
          return error(400, 'req-model-select-1', 'REQUEST_INVALID', 'Expected exactly one model ID.', 'id')
        }
        const model = MODEL_CATALOG.find(({ id }) => id === request.body.id)
        if (!model) return error(404, 'req-model-select-1', 'MODEL_NOT_FOUND', 'Selected model does not exist.', 'id')
        if (model.availability !== 'available') {
          return error(409, 'req-model-select-1', 'MODEL_UNAVAILABLE', 'Selected model is currently unavailable.', 'id')
        }
        return { status: 200, body: { data: structuredClone(model), requestId: 'req-model-select-1' } }
      },
    },
    {
      method: 'POST',
      path: '/api/vacancies/parse',
      resolve(request) {
        if (!exactObject(request.body, ['source']) || !exactObject(request.body.source, ['kind', request.body.source && typeof request.body.source === 'object' && 'url' in request.body.source ? 'url' : 'text'])) {
          return error(400, 'req-vacancy-1', 'REQUEST_INVALID', 'Expected exactly one vacancy source.', 'source')
        }
        const source = request.body.source
        if (source.kind === 'url' && typeof source.url === 'string') {
          if (source.url !== VACANCY_URL) {
            return error(422, 'req-vacancy-1', 'VACANCY_SOURCE_UNAVAILABLE', 'This URL is unavailable. Paste the vacancy text instead.', 'source.url')
          }
          return { status: 200, body: { data: vacancyData('Fixture vacancy', 'vacancy-senior-frontend-1'), requestId: 'req-vacancy-1' } }
        }
        if (source.kind === 'text' && typeof source.text === 'string') {
          if (source.text.length > 10_000) return error(413, 'req-vacancy-1', 'VACANCY_TEXT_TOO_LARGE', 'Vacancy text is too large.', 'source.text')
          if (!source.text.trim()) return error(400, 'req-vacancy-1', 'REQUEST_INVALID', 'Vacancy text is required.', 'source.text')
          return { status: 200, body: { data: vacancyData('Pasted vacancy text', 'vacancy-pasted-text-1'), requestId: 'req-vacancy-1' } }
        }
        return error(400, 'req-vacancy-1', 'REQUEST_INVALID', 'Vacancy source is invalid.', 'source')
      },
    },
    {
      method: 'POST',
      path: '/api/profile-materials/extract',
      resolve(request) {
        if (!exactObject(request.body, ['sourceId', 'kind', 'contentStatus'])
          || typeof request.body.sourceId !== 'string'
          || !['resume', 'project'].includes(String(request.body.kind))
          || !['allowed', 'redacted'].includes(String(request.body.contentStatus))) {
          return error(400, 'req-material-1', 'REQUEST_INVALID', 'Material request is invalid.')
        }
        const material = MATERIALS[request.body.sourceId as keyof typeof MATERIALS]
        if (!material || material.kind !== request.body.kind || material.contentStatus !== request.body.contentStatus || material.contentStatus === 'pending') {
          return error(403, 'req-material-1', 'PROFILE_SOURCE_NOT_ALLOWED', 'Profile source is not approved.', 'sourceId')
        }
        return {
          status: 200,
          body: {
            data: {
              facts: structuredClone(material.facts),
              needsReview: true,
              provenance: { sourceId: request.body.sourceId, extractorRevision: 'mock-fixture' },
            },
            requestId: 'req-material-1',
          },
        }
      },
    },
  ]

  return { definitions, runtime: createScenarioRuntime(definitions), catalog: structuredClone(MODEL_CATALOG) }
}
