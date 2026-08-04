import { http, HttpResponse, type RequestHandler } from 'msw'
import type { ScenarioRuntime } from '../scenarios/runtime'

export function createMswHandlers(runtime: ScenarioRuntime): RequestHandler[] {
  return [http.all('*', async ({ request }) => {
    let body: unknown
    if (request.method !== 'GET' && request.method !== 'HEAD') {
      const text = await request.text()
      body = text ? JSON.parse(text) as unknown : undefined
    }
    try {
      const result = await runtime.resolve({
        method: request.method,
        url: request.url,
        headers: Object.fromEntries(request.headers),
        body,
      })
      return HttpResponse.json(result.body, { status: result.status, headers: result.headers })
    } catch (error) {
      return HttpResponse.json({
        error: { code: 'MOCK_ROUTE_UNHANDLED', message: (error as Error).message, retryable: false },
        requestId: 'req-unhandled',
      }, { status: 501 })
    }
  })]
}
