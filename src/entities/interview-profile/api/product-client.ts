import { createHttpGateway } from '@/shared/api/http'
import type {
  ModelCatalogEntry,
  ModelKind,
  VacancyExtraction,
} from '../model/types'

type ProductTransport = ReturnType<typeof createHttpGateway>
let transport: ProductTransport = createHttpGateway()

export function setProductTransport(fetcher: typeof fetch): void {
  transport = createHttpGateway({ fetch: fetcher })
}

export const productClient = {
  listModels(kind?: ModelKind): Promise<ModelCatalogEntry[]> {
    const suffix = kind ? `?kind=${encodeURIComponent(kind)}` : ''
    return transport.get(`/api/models${suffix}`)
  },
  parseVacancy(source: { kind: 'url'; url: string } | { kind: 'text'; text: string }): Promise<VacancyExtraction> {
    return transport.post('/api/vacancies/parse', { source })
  },
  extractMaterial(input: { sourceId: string; kind: 'resume' | 'project'; contentStatus: 'allowed' | 'redacted' }) {
    return transport.post<{ facts: Array<{ id: string; category: string; text: string; sourceRange: string }>; needsReview: true }>(
      '/api/profile-materials/extract',
      input,
    )
  },
}
