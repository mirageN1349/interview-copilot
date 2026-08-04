export type MeetingStatus = 'prepared' | 'gating' | 'running' | 'stopping' | 'completed' | 'failed' | 'expired'
export type MeetingMode = 'standard_lab' | 'adversarial_lab'
export type CapturePhase = 'idle' | 'starting' | 'listening' | 'recording' | 'paused' | 'stopping' | 'stopped' | 'failed'

export type RunGateResult = {
  allowed: boolean
  reasonCodes: string[]
  policyVersion?: string
  policyExpiresAt?: string
  matrixRowId?: string
}

export type MeetingRuntimeSummary = {
  id: string
  launchPolicyId: string
  profileId: string
  profileRevision: number
  title: string
  status: MeetingStatus
  mode: MeetingMode
  contextGeneration: number
  capturePhase: CapturePhase
  displayId: number
  soundThreshold: number
  createdAtMs: number
  startedAtMs: number | null
  endedAtMs: number | null
  failureCode: string | null
}

export type MeetingStartInput = {
  launchPolicyId: string
  profileId: string
  profileRevision: number
  captureConfigurationId: string
  mode: MeetingMode
  title: string
}

export type DisplayDescriptor = {
  displayId: number
  label: string
  width: number
  height: number
  backingScale: number
  isPrimary: boolean
}
