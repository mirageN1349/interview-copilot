import type { ScenarioRuntime } from './scenarios/runtime'

export function mockTransportTarget(isTauri: boolean, isDev: boolean) {
  return isTauri ? 'packaged' : isDev ? 'browser' : null
}

export function createPackagedFetch(runtime: ScenarioRuntime): typeof fetch {
  return async (input, init) => {
    const request = input instanceof Request ? input : new Request(new URL(String(input), 'https://app.local'), init)
    let body: unknown
    if (request.method !== 'GET' && request.method !== 'HEAD') {
      const text = await request.text()
      body = text ? JSON.parse(text) as unknown : undefined
    }
    const result = await runtime.resolve({
      method: request.method,
      url: request.url,
      headers: Object.fromEntries(request.headers),
      body,
    })
    return new Response(JSON.stringify(result.body), {
      status: result.status,
      headers: { 'content-type': 'application/json', ...result.headers },
    })
  }
}
