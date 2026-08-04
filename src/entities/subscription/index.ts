import { queryOptions, type QueryClient } from '@tanstack/vue-query'

import { createHttpGateway } from '@/shared/api/http'
import { queryKeys } from '@/shared/api/query-keys'

export type DemoSubscription = {
  plan: 'demo'
  status: 'inactive' | 'active'
  features: ['Live assistant', 'Profiles', 'Meeting history']
  expiresAt: null
}

let transport = createHttpGateway()

export function setSubscriptionTransport(fetcher: typeof fetch) {
  transport = createHttpGateway({ fetch: fetcher })
}

export const subscriptionGateway = {
  get: () => transport.get<DemoSubscription>('/api/subscription'),
  activate: () => transport.post<DemoSubscription>('/api/subscription/activate', { plan: 'demo' }),
}

export const subscriptionQuery = () => queryOptions({ queryKey: queryKeys.subscription(), queryFn: subscriptionGateway.get })

export async function updateSubscriptionCache(queryClient: QueryClient, subscription: DemoSubscription) {
  queryClient.setQueryData(queryKeys.subscription(), subscription)
}
