import { describe, expect, it, vi } from 'vitest'

import { createSubscriptionScenario } from '@/mocks/scenarios/subscription'

describe('demo subscription contract', () => {
  it('activates once and returns the same entitlement on retries', async () => {
    const outbound = vi.spyOn(globalThis, 'fetch')
    const scenario = createSubscriptionScenario()

    const first = await scenario.runtime.resolve({ method: 'POST', url: '/api/subscription/activate', body: { plan: 'demo' } })
    const second = await scenario.runtime.resolve({ method: 'POST', url: '/api/subscription/activate', body: { plan: 'demo' } })

    expect(first).toEqual(second)
    expect(first).toMatchObject({
      status: 200,
      body: { data: { plan: 'demo', status: 'active', expiresAt: null }, requestId: 'req-subscription-activate-1' },
    })
    expect(JSON.stringify(first)).not.toMatch(/checkout|payment|card|processor/i)
    expect(outbound).not.toHaveBeenCalled()
  })

  it('rejects every activation payload except the Demo plan', async () => {
    const scenario = createSubscriptionScenario()
    const response = await scenario.runtime.resolve({ method: 'POST', url: '/api/subscription/activate', body: { plan: 'pro' } })

    expect(response).toMatchObject({ status: 400, body: { error: { code: 'PLAN_INVALID' } } })
  })
})
