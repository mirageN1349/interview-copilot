import { createScenarioRuntime, type ScenarioDefinition, type ScenarioResponse } from './runtime'

export type DemoSubscription = {
  plan: 'demo'
  status: 'inactive' | 'active'
  features: ['Live assistant', 'Profiles', 'Meeting history']
  expiresAt: null
}

function invalidPlan(): ScenarioResponse {
  return {
    status: 400,
    body: { error: { code: 'PLAN_INVALID', message: 'Only the Demo plan is available.', retryable: false, field: 'plan' }, requestId: 'req-subscription-activate-1' },
  }
}

export function createSubscriptionScenario() {
  const entitlement: DemoSubscription = { plan: 'demo', status: 'inactive', features: ['Live assistant', 'Profiles', 'Meeting history'], expiresAt: null }
  const definitions: ScenarioDefinition[] = [
    {
      method: 'GET',
      path: '/api/subscription',
      resolve: () => ({ status: 200, body: { data: structuredClone(entitlement), requestId: 'req-subscription-1' } }),
    },
    {
      method: 'POST',
      path: '/api/subscription/activate',
      resolve(request) {
        if (typeof request.body !== 'object' || request.body === null || Array.isArray(request.body)
          || Object.keys(request.body).length !== 1 || !Object.hasOwn(request.body, 'plan')
          || request.body.plan !== 'demo') return invalidPlan()
        entitlement.status = 'active'
        return { status: 200, body: { data: structuredClone(entitlement), requestId: 'req-subscription-activate-1' } }
      },
    },
  ]
  return { definitions, runtime: createScenarioRuntime(definitions) }
}
