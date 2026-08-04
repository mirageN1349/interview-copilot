import { queryOptions, type QueryClient } from '@tanstack/vue-query'

import { nativeGateway } from '@/shared/api/native'
import { queryKeys } from '@/shared/api/query-keys'
import type { DisplayDescriptor, MeetingMode, MeetingRuntimeSummary, MeetingStartInput, RunGateResult } from '../model/types'

type NativeInvoker = { invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> }

export function createMeetingGateway(invoker: NativeInvoker = nativeGateway) {
  return {
    get: (meetingId: string) => invoker.invoke<MeetingRuntimeSummary>('meeting_get', { meetingId }),
    listDisplays: () => invoker.invoke<DisplayDescriptor[]>('displays_list'),
    evaluateGate: (input: { launchPolicyId: string; requestedMode: MeetingMode; captureConfigurationId?: string }) => invoker.invoke<RunGateResult>('run_gate_evaluate', { input }),
    start: (input: MeetingStartInput) => invoker.invoke<MeetingRuntimeSummary>('meeting_start', { input }),
    stop: (meetingId: string, reason: 'user' | 'kill_switch' | 'policy_lost' | 'error') => invoker.invoke<MeetingRuntimeSummary>('meeting_stop', { input: { meetingId, reason } }),
    emergencyStop: () => invoker.invoke<MeetingRuntimeSummary | null>('emergency_stop_all'),
    resetContext: (meetingId: string) => invoker.invoke<number>('meeting_context_reset', { meetingId }),
  }
}

export const meetingGateway = createMeetingGateway()

export const meetingQuery = (meetingId: string) => queryOptions({
  queryKey: queryKeys.meeting(meetingId),
  queryFn: () => meetingGateway.get(meetingId),
  enabled: Boolean(meetingId),
})

export async function updateMeetingCache(queryClient: QueryClient, meeting: MeetingRuntimeSummary) {
  queryClient.setQueryData(queryKeys.meeting(meeting.id), meeting)
  await queryClient.invalidateQueries({ queryKey: queryKeys.meetings() })
}
