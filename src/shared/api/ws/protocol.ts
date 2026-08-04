export const WS_ENDPOINT = 'wss://assistant.mock.invalid/ws/v1'
export const MAX_FRAME_BYTES = 65_536

const COMMAND_TYPES = new Set([
  'policy.subscribe',
  'meeting.start',
  'meeting.stop',
  'meeting.resume',
  'audio.fragment.ready',
  'chat.send',
  'context.reset',
  'diagram.proposal.request',
  'heartbeat',
])

const EVENT_TYPES = new Set([
  'policy.snapshot',
  'kill_switch.activated',
  'meeting.accepted',
  'meeting.completed',
  'meeting.snapshot',
  'transcript.partial',
  'transcript.final',
  'question.detected',
  'answer.started',
  'answer.delta',
  'answer.completed',
  'diagram.patch.proposed',
  'context.reset.completed',
  'command.error',
  'heartbeat.ack',
])

const ISO_UTC = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{3})?Z$/

export type Envelope<TPayload = Record<string, unknown>> = {
  v: 1
  id: string
  type: string
  sentAt: string
  launchPolicyId?: string
  meetingId?: string
  correlationId?: string
  sequence?: number
  payload: TPayload
}

export type ChatThread = 'live' | 'side'
export type StreamRecovery = 'online' | 'synchronizing' | 'diverged'
export type StreamingAnswer = {
  messageId: string
  thread: ChatThread
  sourceIds: string[]
  content: string
  completed: boolean
}
export type MeetingStreamState = {
  meetingId: string
  launchPolicyId: string
  lastAppliedSequence: number
  seenIds: ReadonlySet<string>
  recovery: StreamRecovery
  automaticAnswersSuspended: boolean
  answers: Record<ChatThread, Record<string, StreamingAnswer>>
}
export type ReduceEffect = 'applied' | 'duplicate' | 'snapshot_required' | 'stream_diverged'

export class ProtocolError extends Error {
  constructor(public readonly code: string, message: string) {
    super(message)
    this.name = 'ProtocolError'
  }
}

function record(value: unknown, name: string): Record<string, unknown> {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new ProtocolError('ENVELOPE_INVALID', `${name} must be an object`)
  }
  return value as Record<string, unknown>
}

function boundedString(value: unknown, name: string, max = 4_096): string {
  if (typeof value !== 'string' || value.length === 0 || value.length > max) {
    throw new ProtocolError('ENVELOPE_INVALID', `${name} is invalid`)
  }
  return value
}

function exactKeys(payload: Record<string, unknown>, required: string[], optional: string[] = []) {
  const allowed = new Set([...required, ...optional])
  if (required.some((key) => !Object.hasOwn(payload, key)) || Object.keys(payload).some((key) => !allowed.has(key))) {
    throw new ProtocolError('PAYLOAD_INVALID', 'Payload fields are invalid')
  }
}

function safeInteger(value: unknown, name: string) {
  if (!Number.isSafeInteger(value) || Number(value) < 0) throw new ProtocolError('PAYLOAD_INVALID', `${name} is invalid`)
}

function stringArray(value: unknown, name: string, max = 100): string[] {
  if (!Array.isArray(value) || value.length > max || value.some((item) => typeof item !== 'string' || item.length === 0 || item.length > 128)) {
    throw new ProtocolError('PAYLOAD_INVALID', `${name} is invalid`)
  }
  return value
}

function validatePayload(type: string, payload: Record<string, unknown>) {
  if (type === 'policy.subscribe') {
    exactKeys(payload, ['deviceId'])
    boundedString(payload.deviceId, 'payload.deviceId', 128)
  }
  if (type === 'meeting.start') {
    exactKeys(payload, ['profileId', 'profileRevision', 'captureConfigurationId', 'mode'], ['matrixRowId'])
    boundedString(payload.profileId, 'payload.profileId', 128)
    boundedString(payload.captureConfigurationId, 'payload.captureConfigurationId', 128)
    safeInteger(payload.profileRevision, 'payload.profileRevision')
    if (payload.mode !== 'standard_lab' && payload.mode !== 'adversarial_lab') throw new ProtocolError('PAYLOAD_INVALID', 'payload.mode is invalid')
    if (payload.matrixRowId !== undefined) boundedString(payload.matrixRowId, 'payload.matrixRowId', 128)
  }
  if (type === 'meeting.stop') {
    exactKeys(payload, ['reason'])
    if (!['user', 'kill_switch', 'policy_lost', 'error'].includes(String(payload.reason))) throw new ProtocolError('PAYLOAD_INVALID', 'payload.reason is invalid')
  }
  if (type === 'meeting.resume') {
    exactKeys(payload, ['lastAppliedSequence', 'contextGeneration'])
    safeInteger(payload.lastAppliedSequence, 'payload.lastAppliedSequence')
    safeInteger(payload.contextGeneration, 'payload.contextGeneration')
  }
  if (type === 'audio.fragment.ready') {
    exactKeys(payload, ['artifactId', 'startedAtMs', 'endedAtMs', 'source', 'contentStatus'])
    boundedString(payload.artifactId, 'payload.artifactId', 128)
    safeInteger(payload.startedAtMs, 'payload.startedAtMs')
    safeInteger(payload.endedAtMs, 'payload.endedAtMs')
    if (Number(payload.endedAtMs) < Number(payload.startedAtMs)) throw new ProtocolError('PAYLOAD_INVALID', 'Audio times are invalid')
    if (payload.source !== 'system' && payload.source !== 'microphone') throw new ProtocolError('PAYLOAD_INVALID', 'payload.source is invalid')
    if (payload.contentStatus !== 'allowed' && payload.contentStatus !== 'redacted') throw new ProtocolError('PAYLOAD_INVALID', 'payload.contentStatus is invalid')
  }
  if (type === 'chat.send') {
    exactKeys(payload, ['thread', 'content', 'artifactIds', 'contextGeneration'], ['answerDepth'])
    if (payload.thread !== 'live' && payload.thread !== 'side') throw new ProtocolError('PAYLOAD_INVALID', 'payload.thread is invalid')
    boundedString(payload.content, 'payload.content', 16_000)
    stringArray(payload.artifactIds, 'payload.artifactIds', 20)
    safeInteger(payload.contextGeneration, 'payload.contextGeneration')
    if (payload.answerDepth !== undefined && !['brief', 'balanced', 'detailed'].includes(String(payload.answerDepth))) {
      throw new ProtocolError('PAYLOAD_INVALID', 'payload.answerDepth is invalid')
    }
  }
  if (type === 'context.reset') {
    exactKeys(payload, ['expectedGeneration'])
    safeInteger(payload.expectedGeneration, 'payload.expectedGeneration')
  }
  if (type === 'diagram.proposal.request') {
    exactKeys(payload, ['diagramRevision'], ['questionSegmentId'])
    safeInteger(payload.diagramRevision, 'payload.diagramRevision')
    if (payload.questionSegmentId !== undefined) boundedString(payload.questionSegmentId, 'payload.questionSegmentId', 128)
  }
  if (type === 'heartbeat') {
    exactKeys(payload, [], ['lastReceivedSequence'])
    if (payload.lastReceivedSequence !== undefined) safeInteger(payload.lastReceivedSequence, 'payload.lastReceivedSequence')
  }
  if (type === 'question.detected') {
    boundedString(payload.segmentId, 'payload.segmentId', 128)
    boundedString(payload.text, 'payload.text', 16_000)
    if (typeof payload.confidence !== 'number' || payload.confidence < 0 || payload.confidence > 1) {
      throw new ProtocolError('PAYLOAD_INVALID', 'payload.confidence is invalid')
    }
  }
  if (type === 'policy.snapshot') {
    exactKeys(payload, ['policyVersion', 'expiresAtMs', 'verified', 'killSwitch'])
    boundedString(payload.policyVersion, 'payload.policyVersion', 128)
    safeInteger(payload.expiresAtMs, 'payload.expiresAtMs')
    if (typeof payload.verified !== 'boolean' || !['clear', 'stop_new', 'stop_all'].includes(String(payload.killSwitch))) {
      throw new ProtocolError('PAYLOAD_INVALID', 'Policy snapshot is invalid')
    }
  }
  if (type === 'answer.started') {
    boundedString(payload.messageId, 'payload.messageId', 128)
    if (payload.thread !== 'live' && payload.thread !== 'side') throw new ProtocolError('PAYLOAD_INVALID', 'payload.thread is invalid')
    stringArray(payload.sourceIds, 'payload.sourceIds')
  }
  if (type === 'answer.delta') {
    boundedString(payload.messageId, 'payload.messageId', 128)
    boundedString(payload.delta, 'payload.delta', 16_000)
    if (payload.thread !== undefined && payload.thread !== 'live' && payload.thread !== 'side') {
      throw new ProtocolError('PAYLOAD_INVALID', 'payload.thread is invalid')
    }
  }
  if (type === 'answer.completed') {
    boundedString(payload.messageId, 'payload.messageId', 128)
    if (payload.thread !== undefined && payload.thread !== 'live' && payload.thread !== 'side') {
      throw new ProtocolError('PAYLOAD_INVALID', 'payload.thread is invalid')
    }
  }
}

export function decodeEnvelope(frame: unknown): Envelope {
  if (typeof frame !== 'string') throw new ProtocolError('BINARY_FRAME_UNSUPPORTED', 'Only text frames are supported')
  if (new TextEncoder().encode(frame).byteLength > MAX_FRAME_BYTES) throw new ProtocolError('FRAME_TOO_LARGE', 'Frame exceeds the protocol limit')

  let value: unknown
  try {
    value = JSON.parse(frame)
  } catch {
    throw new ProtocolError('ENVELOPE_INVALID', 'Frame is not valid JSON')
  }
  const input = record(value, 'envelope')
  if (input.v !== 1) throw new ProtocolError('PROTOCOL_VERSION_UNSUPPORTED', 'Protocol version is unsupported')
  const type = boundedString(input.type, 'type', 128)
  if (!COMMAND_TYPES.has(type) && !EVENT_TYPES.has(type)) throw new ProtocolError('MESSAGE_TYPE_UNKNOWN', 'Message type is not supported')
  const sentAt = boundedString(input.sentAt, 'sentAt', 40)
  if (!ISO_UTC.test(sentAt) || Number.isNaN(Date.parse(sentAt))) throw new ProtocolError('ENVELOPE_INVALID', 'sentAt must be UTC ISO-8601')
  const payload = record(input.payload, 'payload')
  validatePayload(type, payload)

  const envelope: Envelope = {
    v: 1,
    id: boundedString(input.id, 'id', 128),
    type,
    sentAt,
    payload,
  }
  for (const key of ['launchPolicyId', 'meetingId', 'correlationId'] as const) {
    if (input[key] !== undefined) envelope[key] = boundedString(input[key], key, 128)
  }
  if (input.sequence !== undefined) {
    if (!Number.isSafeInteger(input.sequence) || Number(input.sequence) < 0) throw new ProtocolError('ENVELOPE_INVALID', 'sequence is invalid')
    envelope.sequence = Number(input.sequence)
  }
  if (envelope.sequence !== undefined && (!envelope.meetingId || !envelope.launchPolicyId)) {
    throw new ProtocolError('ENVELOPE_INVALID', 'Stream events require meeting and policy IDs')
  }
  return envelope
}

export function encodeEnvelope(envelope: Envelope): string {
  const encoded = JSON.stringify(envelope)
  if (new TextEncoder().encode(encoded).byteLength > MAX_FRAME_BYTES) throw new ProtocolError('FRAME_TOO_LARGE', 'Frame exceeds the protocol limit')
  return encoded
}

export function createMeetingStreamState(meetingId: string, launchPolicyId: string): MeetingStreamState {
  return {
    meetingId,
    launchPolicyId,
    lastAppliedSequence: 0,
    seenIds: new Set(),
    recovery: 'online',
    automaticAnswersSuspended: false,
    answers: { live: {}, side: {} },
  }
}

function diverged(state: MeetingStreamState): MeetingStreamState {
  return { ...state, recovery: 'diverged', automaticAnswersSuspended: true }
}

export function reduceMeetingEvent(state: MeetingStreamState, envelope: Envelope): { state: MeetingStreamState; effect: ReduceEffect } {
  if (envelope.meetingId !== state.meetingId || envelope.launchPolicyId !== state.launchPolicyId || envelope.sequence === undefined) {
    return { state: diverged(state), effect: 'stream_diverged' }
  }
  if (state.seenIds.has(envelope.id)) return { state, effect: 'duplicate' }
  if (envelope.sequence <= state.lastAppliedSequence) return { state: diverged(state), effect: 'stream_diverged' }
  if (envelope.sequence !== state.lastAppliedSequence + 1) {
    return {
      state: { ...state, recovery: 'synchronizing', automaticAnswersSuspended: true },
      effect: 'snapshot_required',
    }
  }

  const seenIds = new Set(state.seenIds).add(envelope.id)
  const next: MeetingStreamState = {
    ...state,
    lastAppliedSequence: envelope.sequence,
    seenIds,
    recovery: 'online',
    answers: { live: { ...state.answers.live }, side: { ...state.answers.side } },
  }
  const payload = envelope.payload

  if (envelope.type === 'answer.started') {
    const thread = payload.thread as ChatThread
    const messageId = String(payload.messageId)
    if (next.answers[thread][messageId]) return { state: diverged(state), effect: 'stream_diverged' }
    next.answers[thread][messageId] = {
      messageId,
      thread,
      sourceIds: [...payload.sourceIds as string[]],
      content: '',
      completed: false,
    }
  }

  if (envelope.type === 'answer.delta' || envelope.type === 'answer.completed') {
    const messageId = String(payload.messageId)
    const found = next.answers.live[messageId] ?? next.answers.side[messageId]
    if (!found || found.completed || (payload.thread !== undefined && payload.thread !== found.thread)) {
      return { state: diverged(state), effect: 'stream_diverged' }
    }
    const updated = { ...found }
    if (envelope.type === 'answer.delta') updated.content += String(payload.delta)
    else updated.completed = true
    next.answers[found.thread][messageId] = updated
  }

  return { state: next, effect: 'applied' }
}
