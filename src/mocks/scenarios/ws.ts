import { ProtocolError, decodeEnvelope, encodeEnvelope, type Envelope } from '@/shared/api/ws/protocol'
import type { SocketLike } from '@/shared/api/ws/client'

const SENT_AT = '2026-08-04T10:00:00.000Z'
const POLICY_ID = 'policy-1'
const MEETING_ID = 'meeting-fixture-1'
const ALLOWED_ARTIFACTS = new Set(['artifact-audio-1', 'artifact-screen-redacted-1'])
const MOCK_QUESTIONS = [
  'Расскажите о сложном проекте, которым вы особенно гордитесь.',
  'Как вы принимаете технические решения при неполных требованиях?',
  'Приведите пример производственной ошибки и того, как вы её исправили.',
]
const MOCK_ANSWERS = [
  ['Я бы начал с контекста и результата. ', 'В одном из проектов я отвечал за критичный пользовательский сценарий, ', 'разбил рискованную поставку на проверяемые этапы ', 'и довёл её до стабильного релиза с измеримым эффектом.'],
  ['Сначала я фиксирую ограничения и критерий успеха. ', 'Затем выбираю минимальное обратимое решение, ', 'проверяю его на реальных данных ', 'и только после этого расширяю реализацию.'],
  ['Я быстро локализовал источник сбоя по логам и метрикам, ', 'остановил дальнейшее влияние на пользователей, ', 'добавил регрессионную проверку ', 'и задокументировал сигнал, который позволит заметить проблему раньше.'],
]

function allowedArtifact(id: unknown): id is string {
  return typeof id === 'string' && (ALLOWED_ARTIFACTS.has(id) || /^audio-\d+-\d+$/.test(id))
}

function fixtureIndex(seed: string): number {
  const audioSequence = /^audio-\d+-(\d+)$/.exec(seed)?.[1]
  if (audioSequence) return (Number(audioSequence) - 1) % MOCK_QUESTIONS.length
  return [...seed].reduce((total, character) => total + character.charCodeAt(0), 0) % MOCK_QUESTIONS.length
}

type MeetingScenario = ReturnType<typeof createMeetingScenario>

function commandError(command: Envelope, code: string, retryable: boolean): Envelope {
  return {
    v: 1,
    id: `error-${command.id}`,
    type: 'command.error',
    sentAt: SENT_AT,
    meetingId: command.meetingId,
    launchPolicyId: command.launchPolicyId,
    correlationId: command.id,
    payload: { code, retryable },
  }
}

export function createMeetingScenario() {
  let activeMeetingId = MEETING_ID
  let running = false
  let completed: Envelope | undefined
  let contextGeneration = 0
  let sequence = 0
  const responses = new Map<string, string[]>()
  const replay: string[] = []

  function serverEvent(command: Envelope, type: string, payload: Record<string, unknown>): Envelope {
    sequence += 1
    return {
      v: 1,
      id: `server-${sequence}-${type}`,
      type,
      sentAt: SENT_AT,
      meetingId: activeMeetingId,
      launchPolicyId: command.launchPolicyId ?? POLICY_ID,
      correlationId: command.id,
      sequence,
      payload,
    }
  }

  function stream(command: Envelope, ...events: Envelope[]) {
    const encoded = events.map(encodeEnvelope)
    replay.push(...encoded)
    if (replay.length > 50) replay.splice(0, replay.length - 50)
    responses.set(command.id, encoded)
    return encoded
  }

  function fail(command: Envelope, code: string, retryable = false) {
    const result = [encodeEnvelope(commandError(command, code, retryable))]
    responses.set(command.id, result)
    return result
  }

  function receive(frame: unknown): string[] {
    let command: Envelope
    try {
      command = decodeEnvelope(frame)
    } catch (error) {
      const code = error instanceof ProtocolError ? error.code : 'ENVELOPE_INVALID'
      return [encodeEnvelope({
        v: 1,
        id: 'error-invalid-frame',
        type: 'command.error',
        sentAt: SENT_AT,
        payload: { code, retryable: false },
      })]
    }
    const cached = responses.get(command.id)
    if (cached) return [...cached]

    if (command.type === 'heartbeat') {
      const result = [encodeEnvelope({
        v: 1,
        id: `ack-${command.id}`,
        type: 'heartbeat.ack',
        sentAt: SENT_AT,
        correlationId: command.id,
        payload: { lastAcceptedSequence: sequence },
      })]
      responses.set(command.id, result)
      return result
    }
    if (command.type === 'policy.subscribe') {
      const result = [encodeEnvelope({
        v: 1,
        id: `policy-${command.id}`,
        type: 'policy.snapshot',
        sentAt: SENT_AT,
        correlationId: command.id,
        payload: {
          policyVersion: POLICY_ID,
          expiresAtMs: Date.parse('2099-12-31T23:59:59.000Z'),
          verified: true,
          killSwitch: 'clear',
        },
      })]
      responses.set(command.id, result)
      return result
    }
    if (command.type === 'meeting.start') {
      if (![POLICY_ID, 'default-meeting-policy'].includes(String(command.launchPolicyId))) return fail(command, 'POLICY_DENIED')
      activeMeetingId = command.meetingId ?? MEETING_ID
      running = true
      return stream(command, serverEvent(command, 'meeting.accepted', { meetingId: activeMeetingId, startedAt: SENT_AT }))
    }
    if (command.type === 'meeting.stop') {
      if (completed) {
        const result = [encodeEnvelope({ ...completed, correlationId: command.id })]
        responses.set(command.id, result)
        return result
      }
      running = false
      completed = serverEvent(command, 'meeting.completed', {
        meetingId: MEETING_ID,
        stoppedAt: SENT_AT,
        artifacts: { recording: 'recording-fixture-1' },
      })
      return stream(command, completed)
    }
    if (command.type === 'meeting.resume') {
      if (!running && !completed && command.meetingId) {
        activeMeetingId = command.meetingId
        running = true
        contextGeneration = Number(command.payload.contextGeneration ?? 0)
        return stream(command, serverEvent(command, 'meeting.snapshot', {
          meetingId: activeMeetingId,
          contextGeneration,
          lastSequence: sequence + 1,
        }))
      }
      const lastSequence = Number(command.payload.lastAppliedSequence ?? 0)
      const available = replay.filter((item) => (decodeEnvelope(item).sequence ?? 0) > lastSequence)
      if (available.length <= 20) {
        responses.set(command.id, available)
        return [...available]
      }
      return stream(command, serverEvent(command, 'meeting.snapshot', {
        meetingId: MEETING_ID,
        contextGeneration,
        lastSequence: sequence,
      }))
    }
    if (!running || command.meetingId !== activeMeetingId) return fail(command, 'MEETING_NOT_RUNNING')

    if (command.type === 'context.reset') {
      if (command.payload.expectedGeneration !== contextGeneration) return fail(command, 'CONTEXT_GENERATION_STALE', true)
      contextGeneration += 1
      return stream(command, serverEvent(command, 'context.reset.completed', { contextGeneration }))
    }
    if (command.type === 'chat.send') {
      if (command.payload.contextGeneration !== contextGeneration) return fail(command, 'CONTEXT_GENERATION_STALE', true)
      const artifactIds = Array.isArray(command.payload.artifactIds) ? command.payload.artifactIds : []
      if (artifactIds.some((id) => !allowedArtifact(id))) return fail(command, 'ARTIFACT_NOT_ALLOWED')
      const thread = command.payload.thread === 'side' ? 'side' : 'live'
      const messageId = `answer-${command.id}`
      const answer = MOCK_ANSWERS[fixtureIndex(String(command.payload.content))]!
      const chunks = command.payload.answerDepth === 'brief'
        ? answer.slice(0, 2)
        : command.payload.answerDepth === 'detailed'
          ? [...answer, ' В завершение я бы связал этот пример с требованиями роли и уточнил ожидаемые метрики успеха.']
          : answer
      return stream(
        command,
        serverEvent(command, 'answer.started', { messageId, thread, sourceIds: ['profile-source-1'] }),
        ...chunks.map((delta) => serverEvent(command, 'answer.delta', { messageId, thread, delta })),
        serverEvent(command, 'answer.completed', { messageId, thread, sourceIds: ['profile-source-1'] }),
      )
    }
    if (command.type === 'audio.fragment.ready') {
      if (!allowedArtifact(command.payload.artifactId)) return fail(command, 'ARTIFACT_NOT_ALLOWED')
      const segmentId = `segment-${command.id}`
      const question = MOCK_QUESTIONS[fixtureIndex(String(command.payload.artifactId))]!
      return stream(
        command,
        serverEvent(command, 'transcript.final', { segmentId, text: question, confidence: 0.96 }),
        serverEvent(command, 'question.detected', { segmentId, text: question, confidence: 0.96 }),
      )
    }
    if (command.type === 'diagram.proposal.request') {
      return stream(command, serverEvent(command, 'diagram.patch.proposed', {
        id: `proposal-${command.id}`,
        baseRevision: command.payload.diagramRevision,
        operations: [{ type: 'node.add', node: { id: 'api', label: 'API', x: 32, y: 32 } }],
      }))
    }
    return fail(command, 'MESSAGE_TYPE_UNKNOWN')
  }

  return { receive, meetingId: MEETING_ID, launchPolicyId: POLICY_ID }
}

class InMemoryMeetingSocket extends EventTarget implements SocketLike {
  readonly CONNECTING = 0
  readonly OPEN = 1
  readonly CLOSING = 2
  readonly CLOSED = 3
  readyState = this.CONNECTING
  private readonly pendingFrames: string[] = []
  private drainTimer: ReturnType<typeof setTimeout> | undefined
  private draining = false

  constructor(private readonly scenario: MeetingScenario) {
    super()
    queueMicrotask(() => {
      if (this.readyState !== this.CONNECTING) return
      this.readyState = this.OPEN
      this.dispatchEvent(new Event('open'))
    })
  }

  send(data: string) {
    if (this.readyState !== this.OPEN) throw new Error('Socket is not open')
    this.pendingFrames.push(...this.scenario.receive(data))
    if (!this.draining) {
      this.draining = true
      queueMicrotask(() => this.drain())
    }
  }

  private drain() {
    if (this.readyState !== this.OPEN) return
    const frame = this.pendingFrames.shift()
    if (!frame) {
      this.drainTimer = undefined
      this.draining = false
      return
    }
    this.dispatchEvent(new MessageEvent('message', { data: frame }))
    this.drainTimer = setTimeout(() => this.drain(), 90)
  }

  close() {
    if (this.readyState === this.CLOSED) return
    if (this.drainTimer) clearTimeout(this.drainTimer)
    this.draining = false
    this.readyState = this.CLOSED
    this.dispatchEvent(new CloseEvent('close'))
  }
}

export function createInMemoryMeetingSocket(scenario = createMeetingScenario()): SocketLike {
  return new InMemoryMeetingSocket(scenario)
}
