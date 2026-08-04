import { http, HttpResponse, type RequestHandler } from 'msw'

import { createSubscriptionScenario } from '../scenarios/subscription'

export const subscriptionScenario = createSubscriptionScenario()

export function createSubscriptionHandlers(scenario = subscriptionScenario): RequestHandler[] {
  return scenario.definitions.map((definition) => http.all(`*${definition.path}`, async ({ request }) => {
    const text = request.method === 'GET' || request.method === 'HEAD' ? '' : await request.text()
    const result = await scenario.runtime.resolve({
      method: request.method,
      url: request.url,
      headers: Object.fromEntries(request.headers),
      body: text ? JSON.parse(text) as unknown : undefined,
    })
    return HttpResponse.json(result.body, { status: result.status, headers: result.headers })
  }))
}

export const subscriptionHandlers = createSubscriptionHandlers()
