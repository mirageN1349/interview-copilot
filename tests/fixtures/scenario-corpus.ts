import type { ScenarioDefinition } from '@/mocks/scenarios/runtime'

export const scenarioCorpus: ScenarioDefinition[] = [
  {
    method: 'GET',
    path: '/api/ping',
    resolve: ({ requestId }) => ({ status: 200, body: { data: { ok: true }, requestId } }),
  },
  {
    method: 'POST',
    path: '/api/bounded',
    resolve: ({ body, requestId }) => ({ status: 200, body: { data: body, requestId } }),
  },
]
