export type ChatThread = 'live' | 'side'
export type AssistantMessage = {
  id: string
  thread: ChatThread
  content: string
  status: 'streaming' | 'complete' | 'error' | 'cancelled'
  profileSourceIds: string[]
  contextGeneration: number
}

export type RecognizedQuestion = {
  segmentId: string
  text: string
  confidence: number
  requiresConfirmation: boolean
}

export type MeetingViewState = {
  meetingId: string
  confidenceThreshold: number
  lastAppliedSequence: number
  appliedEventIds: string[]
  contextGeneration: number
  pendingQuestion: RecognizedQuestion | null
  messages: Record<ChatThread, AssistantMessage[]>
  outbound: Array<{ kind: 'answer.request'; segmentId: string; contextGeneration: number }>
  synchronization: 'ready' | 'required'
}

type MeetingEvent = {
  id: string
  sequence: number
  type: 'question.detected' | 'answer.started' | 'answer.delta' | 'answer.completed' | 'context.reset.completed'
  payload: Record<string, unknown>
}

export function createMeetingViewState(meetingId: string, confidenceThreshold: number): MeetingViewState {
  return {
    meetingId,
    confidenceThreshold,
    lastAppliedSequence: 0,
    appliedEventIds: [],
    contextGeneration: 0,
    pendingQuestion: null,
    messages: { live: [], side: [] },
    outbound: [],
    synchronization: 'ready',
  }
}

export function applyMeetingEvent(current: MeetingViewState, event: MeetingEvent): MeetingViewState {
  if (current.appliedEventIds.includes(event.id)) return current
  if (event.sequence !== current.lastAppliedSequence + 1) {
    return { ...current, synchronization: 'required' }
  }

  const state: MeetingViewState = {
    ...current,
    appliedEventIds: [...current.appliedEventIds.slice(-499), event.id],
    lastAppliedSequence: event.sequence,
    messages: {
      live: current.messages.live.map((message) => ({ ...message, profileSourceIds: [...message.profileSourceIds] })),
      side: current.messages.side.map((message) => ({ ...message, profileSourceIds: [...message.profileSourceIds] })),
    },
    outbound: [...current.outbound],
  }

  if (event.type === 'question.detected') {
    const segmentId = readString(event.payload.segmentId)
    const text = readString(event.payload.text)
    const confidence = readConfidence(event.payload.confidence)
    const requiresConfirmation = confidence < state.confidenceThreshold
    state.pendingQuestion = { segmentId, text, confidence, requiresConfirmation }
    if (!requiresConfirmation) {
      state.outbound.push({ kind: 'answer.request', segmentId, contextGeneration: state.contextGeneration })
    }
  }

  if (event.type === 'answer.started') {
    const thread = readThread(event.payload.thread)
    state.messages[thread].push({
      id: readString(event.payload.messageId),
      thread,
      content: '',
      status: 'streaming',
      profileSourceIds: readStringArray(event.payload.profileSourceIds ?? event.payload.sourceIds),
      contextGeneration: state.contextGeneration,
    })
  }

  if (event.type === 'answer.delta') {
    const message = findMessage(state, readString(event.payload.messageId))
    if (!message || message.status !== 'streaming') return { ...state, synchronization: 'required' }
    message.content += readString(event.payload.delta)
  }

  if (event.type === 'answer.completed') {
    const message = findMessage(state, readString(event.payload.messageId))
    if (!message || message.status !== 'streaming') return { ...state, synchronization: 'required' }
    message.status = 'complete'
  }

  if (event.type === 'context.reset.completed') {
    const generation = event.payload.generation ?? event.payload.contextGeneration
    if (!Number.isInteger(generation) || Number(generation) !== state.contextGeneration + 1) {
      return { ...state, synchronization: 'required' }
    }
    state.contextGeneration = Number(generation)
    state.pendingQuestion = null
    state.outbound = []
  }

  return state
}

function findMessage(state: MeetingViewState, id: string): AssistantMessage | undefined {
  return [...state.messages.live, ...state.messages.side].find((message) => message.id === id)
}

function readString(value: unknown): string {
  if (typeof value !== 'string' || value.length === 0 || value.length > 20_000) throw new TypeError('Invalid meeting event text')
  return value
}

function readStringArray(value: unknown): string[] {
  if (!Array.isArray(value) || value.length > 100 || value.some((item) => typeof item !== 'string')) throw new TypeError('Invalid source IDs')
  return [...value]
}

function readConfidence(value: unknown): number {
  if (typeof value !== 'number' || !Number.isFinite(value) || value < 0 || value > 1) throw new TypeError('Invalid confidence')
  return value
}

function readThread(value: unknown): ChatThread {
  if (value !== 'live' && value !== 'side') throw new TypeError('Invalid chat thread')
  return value
}
