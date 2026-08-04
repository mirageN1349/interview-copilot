import type { QueryClient } from '@tanstack/vue-query'

import { queryKeys } from '@/shared/api/query-keys'
import type { MeetingRuntimeSummary } from './types'

export function applyMeetingRuntimeEvent(
  queryClient: QueryClient,
  meeting: MeetingRuntimeSummary,
): void {
  queryClient.setQueryData(queryKeys.meeting(meeting.id), meeting)
  queryClient.invalidateQueries({ queryKey: queryKeys.meetings() }).catch(() => undefined)
}
