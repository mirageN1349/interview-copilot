import { infiniteQueryOptions, queryOptions, type QueryClient } from '@tanstack/vue-query'

import { nativeGateway } from '@/shared/api/native'
import { queryKeys } from '@/shared/api/query-keys'
import { normalizeHistoryFilters, type HistoryFilters, type HistoryMeetingDetail, type HistoryPage } from '../model/history-filters'

type NativeInvoker = { invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> }

export function createHistoryGateway(invoker: NativeInvoker = nativeGateway) {
  return {
    search: (filters: HistoryFilters) => invoker.invoke<HistoryPage>('meeting_search', { input: filters }),
    get: (meetingId: string) => invoker.invoke<HistoryMeetingDetail>('meeting_history_get', { meetingId }),
    deleteContent: (meetingId: string) => invoker.invoke<{ meetingId: string; filesPending: number }>('meeting_delete_content', { input: { meetingId } }),
  }
}

export const historyGateway = createHistoryGateway()

export const historyQuery = (filters: Partial<HistoryFilters>) => {
  const normalized = normalizeHistoryFilters(filters)
  return infiniteQueryOptions({
    queryKey: queryKeys.meetings(normalized),
    initialPageParam: undefined as string | undefined,
    queryFn: ({ pageParam }) => historyGateway.search({ ...normalized, ...(pageParam ? { cursor: pageParam } : {}) }),
    getNextPageParam: (page) => page.nextCursor ?? undefined,
  })
}

export const historyMeetingQuery = (meetingId: string) => queryOptions({
  queryKey: [...queryKeys.meeting(meetingId), 'history'] as const,
  queryFn: () => historyGateway.get(meetingId),
  enabled: Boolean(meetingId),
})

export async function invalidateHistory(queryClient: QueryClient, meetingId?: string) {
  await queryClient.invalidateQueries({ queryKey: queryKeys.meetings() })
  if (meetingId) await queryClient.invalidateQueries({ queryKey: queryKeys.meeting(meetingId) })
}
