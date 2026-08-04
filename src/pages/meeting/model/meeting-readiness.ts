export type MeetingReadinessInput = {
  profileReady: boolean
  responseModelAvailable: boolean
  transcriptionModelAvailable: boolean
  translationSupported: boolean
  displaySelected: boolean
  permissionsGranted: boolean
  runGateAllowed: boolean
}

export type MeetingReadinessReason =
  | 'profile'
  | 'response_model'
  | 'transcription_model'
  | 'translation'
  | 'display'
  | 'permissions'
  | 'meeting_unavailable'

export function evaluateMeetingReadiness(input: MeetingReadinessInput) {
  const reasons: MeetingReadinessReason[] = []
  if (!input.profileReady) reasons.push('profile')
  if (!input.responseModelAvailable) reasons.push('response_model')
  if (!input.transcriptionModelAvailable) reasons.push('transcription_model')
  if (!input.translationSupported) reasons.push('translation')
  if (!input.displaySelected) reasons.push('display')
  if (!input.permissionsGranted) reasons.push('permissions')
  if (!input.runGateAllowed) reasons.push('meeting_unavailable')
  return { ready: reasons.length === 0, reasons }
}
