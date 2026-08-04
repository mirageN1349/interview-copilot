import { describe, expect, it, vi } from 'vitest'

import {
  ProtocolError,
  createMeetingStreamState,
  decodeEnvelope,
  encodeEnvelope,
  reduceMeetingEvent,
  type Envelope,
} from '@/shared/api/ws/protocol'
import { createWebSocketClient, reconnectDelay, type SocketLike } from '@/shared/api/ws/client'
import { createInMemoryMeetingSocket, createMeetingScenario } from '@/mocks/scenarios/ws'
import { applyMeetingEvent, createMeetingViewState } from '@/features/overlay-chat/model/meeting-reducer'

const now = '2026-08-04T10:00:00.000Z'

function event(type: string, sequence: number, payload: Record<string, unknown>, id = `event-${sequence}`): Envelope {
  return {
    v: 1,
    id,
    type,
    sentAt: now,
    meetingId: 'meeting-1',
    launchPolicyId: 'policy-1',
    sequence,
    payload,
  }
}

class FakeSocket extends EventTarget implements SocketLike {
  readonly CONNECTING = 0
  readonly OPEN = 1
  readonly CLOSING = 2
  readonly CLOSED = 3
  readyState = this.CONNECTING
  sent: string[] = []

  send(data: string) {
    this.sent.push(data)
  }

  close() {
    this.readyState = this.CLOSED
    this.dispatchEvent(new CloseEvent('close'))
  }

  open() {
    this.readyState = this.OPEN
    this.dispatchEvent(new Event('open'))
  }

  receive(data: string) {
    this.dispatchEvent(new MessageEvent('message', { data }))
  }
}

describe('WebSocket protocol', () => {
  it('accepts only bounded, versioned, known text envelopes', () => {
    const valid = event('question.detected', 1, { segmentId: 'segment-1', text: 'Why?', confidence: 0.9 })
    expect(decodeEnvelope(encodeEnvelope(valid))).toEqual(valid)
    expect(() => decodeEnvelope(encodeEnvelope({ ...valid, v: 2 } as never))).toThrowError(
      expect.objectContaining({ code: 'PROTOCOL_VERSION_UNSUPPORTED' }),
    )
    expect(() => decodeEnvelope(encodeEnvelope({ ...valid, type: 'unknown.event' }))).toThrow(ProtocolError)
    expect(() => decodeEnvelope(encodeEnvelope({
      ...valid,
      type: 'chat.send',
      payload: { thread: 'live', content: 'ok', artifactIds: [], contextGeneration: 0, rawBytes: 'forbidden' },
    }))).toThrowError(expect.objectContaining({ code: 'PAYLOAD_INVALID' }))
    expect(() => decodeEnvelope(`"${'x'.repeat(65_537)}"`)).toThrowError(
      expect.objectContaining({ code: 'FRAME_TOO_LARGE' }),
    )
    expect(() => decodeEnvelope(new Uint8Array([1, 2, 3]))).toThrowError(
      expect.objectContaining({ code: 'BINARY_FRAME_UNSUPPORTED' }),
    )
  })

  it('applies ordered answer events once and isolates live from side', () => {
    let state = createMeetingStreamState('meeting-1', 'policy-1')
    state = reduceMeetingEvent(state, event('answer.started', 1, {
      messageId: 'live-1', thread: 'live', sourceIds: ['profile-source-1'],
    })).state
    state = reduceMeetingEvent(state, event('answer.delta', 2, {
      messageId: 'live-1', delta: 'Live answer', thread: 'live',
    })).state
    const duplicate = reduceMeetingEvent(state, event('answer.delta', 2, {
      messageId: 'live-1', delta: 'Live answer', thread: 'live',
    }))
    expect(duplicate.effect).toBe('duplicate')
    state = duplicate.state
    state = reduceMeetingEvent(state, event('answer.started', 3, {
      messageId: 'side-1', thread: 'side', sourceIds: [],
    })).state
    state = reduceMeetingEvent(state, event('answer.delta', 4, {
      messageId: 'side-1', delta: 'Side answer', thread: 'side',
    })).state

    expect(state.answers.live['live-1']?.content).toBe('Live answer')
    expect(state.answers.side['side-1']?.content).toBe('Side answer')
    expect(state.answers.side['live-1']).toBeUndefined()
  })

  it('suspends on gaps, unknown old events and invalid delta order', () => {
    const initial = createMeetingStreamState('meeting-1', 'policy-1')
    const gap = reduceMeetingEvent(initial, event('question.detected', 2, {
      segmentId: 'segment-2', text: 'Gap?', confidence: 0.8,
    }))
    expect(gap.effect).toBe('snapshot_required')
    expect(gap.state.recovery).toBe('synchronizing')

    const started = reduceMeetingEvent(initial, event('answer.started', 1, {
      messageId: 'answer-1', thread: 'live', sourceIds: [],
    })).state
    const invalid = reduceMeetingEvent(started, event('answer.delta', 2, {
      messageId: 'missing', delta: 'must not render', thread: 'live',
    }))
    expect(invalid.effect).toBe('stream_diverged')
    expect(invalid.state.answers.live.missing).toBeUndefined()

    const unknownOld = reduceMeetingEvent(started, event('question.detected', 1, {
      segmentId: 'other', text: 'Old?', confidence: 0.8,
    }, 'unknown-old'))
    expect(unknownOld.effect).toBe('stream_diverged')
  })
})

describe('meeting scenario parity', () => {
  it('is deterministic, idempotent and rejects stale or foreign context', () => {
    const scenario = createMeetingScenario()
    const start = {
      v: 1 as const,
      id: 'command-start',
      type: 'meeting.start',
      sentAt: now,
      launchPolicyId: 'policy-1',
      payload: {
        profileId: 'profile-1', profileRevision: 1, captureConfigurationId: 'capture-1', mode: 'standard_lab',
      },
    }
    const accepted = scenario.receive(encodeEnvelope(start))
    expect(accepted).toEqual(scenario.receive(encodeEnvelope(start)))
    const meetingId = decodeEnvelope(accepted[0]!).meetingId

    const stale = scenario.receive(encodeEnvelope({
      ...start,
      id: 'command-chat-stale',
      type: 'chat.send',
      meetingId,
      payload: { thread: 'live', content: 'Help', artifactIds: [], contextGeneration: 8 },
    }))
    expect(decodeEnvelope(stale[0]!).payload).toEqual(expect.objectContaining({ code: 'CONTEXT_GENERATION_STALE' }))

    const foreign = scenario.receive(encodeEnvelope({
      ...start,
      id: 'command-chat-artifact',
      type: 'chat.send',
      meetingId,
      payload: { thread: 'live', content: 'Help', artifactIds: ['artifact-foreign'], contextGeneration: 0 },
    }))
    expect(decodeEnvelope(foreign[0]!).payload).toEqual(expect.objectContaining({ code: 'ARTIFACT_NOT_ALLOWED' }))
  })

  it('uses the same reducer through the socket-compatible packaged adapter', async () => {
    const scenario = createMeetingScenario()
    const socket = createInMemoryMeetingSocket(scenario)
    await new Promise<void>((resolve) => socket.addEventListener('open', () => resolve(), { once: true }))
    const received = new Promise<Envelope>((resolve) => {
      socket.addEventListener('message', (message) => resolve(decodeEnvelope((message as MessageEvent).data)), { once: true })
    })
    socket.send(encodeEnvelope({
      v: 1,
      id: 'heartbeat-packaged',
      type: 'heartbeat',
      sentAt: now,
      payload: {},
    }))
    await expect(received).resolves.toEqual(expect.objectContaining({ type: 'heartbeat.ack', correlationId: 'heartbeat-packaged' }))
    socket.close()
  })

  it('turns vetted audio into a question and streams chat answers in chunks', () => {
    const scenario = createMeetingScenario()
    const start = {
      v: 1 as const,
      id: 'command-start-stream',
      type: 'meeting.start',
      sentAt: now,
      launchPolicyId: 'policy-1',
      meetingId: 'meeting-stream',
      payload: { profileId: 'profile-1', profileRevision: 1, captureConfigurationId: 'capture-1', mode: 'standard_lab' },
    }
    scenario.receive(encodeEnvelope(start))
    const audio = scenario.receive(encodeEnvelope({
      ...start,
      id: 'command-audio-stream',
      type: 'audio.fragment.ready',
      payload: { artifactId: 'audio-1785880000000-1', startedAtMs: 1, endedAtMs: 2, source: 'system', contentStatus: 'allowed' },
    })).map(decodeEnvelope)
    expect(audio.map((event) => event.type)).toEqual(['transcript.final', 'question.detected'])

    const answer = scenario.receive(encodeEnvelope({
      ...start,
      id: 'command-chat-stream',
      type: 'chat.send',
      payload: { thread: 'live', content: String(audio[1]!.payload.text), artifactIds: [], contextGeneration: 0 },
    })).map(decodeEnvelope)
    expect(answer.filter((event) => event.type === 'answer.delta')).toHaveLength(4)
    expect(answer.at(0)?.type).toBe('answer.started')
    expect(answer.at(-1)?.type).toBe('answer.completed')
  })

  it('cycles through different live interview questions', () => {
    const scenario = createMeetingScenario()
    const start = {
      v: 1 as const, id: 'start-variety', type: 'meeting.start', sentAt: now,
      launchPolicyId: 'policy-1', meetingId: 'meeting-variety',
      payload: { profileId: 'profile-1', profileRevision: 1, captureConfigurationId: 'capture-1', mode: 'standard_lab' },
    }
    scenario.receive(encodeEnvelope(start))
    const questions = [1, 2, 3].map((index) => {
      const events = scenario.receive(encodeEnvelope({
        ...start,
        id: `audio-variety-${index}`,
        type: 'audio.fragment.ready',
        payload: { artifactId: `audio-1785880000000-${index}`, startedAtMs: 1, endedAtMs: 2, source: 'system', contentStatus: 'allowed' },
      })).map(decodeEnvelope)
      return events.find((event) => event.type === 'question.detected')?.payload.text
    })
    expect(new Set(questions).size).toBe(3)
  })

  it('applies the selected mock answer depth', () => {
    const scenario = createMeetingScenario()
    const start = {
      v: 1 as const, id: 'start-depth', type: 'meeting.start', sentAt: now,
      launchPolicyId: 'policy-1', meetingId: 'meeting-depth',
      payload: { profileId: 'profile-1', profileRevision: 1, captureConfigurationId: 'capture-1', mode: 'standard_lab' },
    }
    scenario.receive(encodeEnvelope(start))
    const answer = scenario.receive(encodeEnvelope({
      ...start,
      id: 'chat-depth',
      type: 'chat.send',
      payload: { thread: 'side', content: 'Кратко', artifactIds: [], contextGeneration: 0, answerDepth: 'brief' },
    })).map(decodeEnvelope)

    expect(answer.filter((event) => event.type === 'answer.delta')).toHaveLength(2)
  })

  it('keeps policy control events outside the side-chat meeting sequence', () => {
    const scenario = createMeetingScenario()
    scenario.receive(encodeEnvelope({
      v: 1, id: 'policy-side', type: 'policy.subscribe', sentAt: now, payload: { deviceId: 'mac-1' },
    }))
    const start = {
      v: 1 as const,
      id: 'start-side',
      type: 'meeting.start',
      sentAt: now,
      launchPolicyId: 'policy-1',
      meetingId: 'meeting-side',
      payload: { profileId: 'profile-1', profileRevision: 1, captureConfigurationId: 'capture-1', mode: 'standard_lab' },
    }
    const accepted = decodeEnvelope(scenario.receive(encodeEnvelope(start))[0]!)
    expect(accepted.sequence).toBe(1)

    let view = createMeetingViewState('meeting-side', 0.72)
    view = { ...view, lastAppliedSequence: accepted.sequence! }
    const answer = scenario.receive(encodeEnvelope({
      ...start,
      id: 'chat-side',
      type: 'chat.send',
      payload: { thread: 'side', content: 'Помоги сформулировать ответ', artifactIds: [], contextGeneration: 0 },
    })).map(decodeEnvelope)
    for (const event of answer) {
      view = applyMeetingEvent(view, {
        id: event.id,
        sequence: event.sequence!,
        type: event.type as Parameters<typeof applyMeetingEvent>[1]['type'],
        payload: event.payload,
      })
    }

    expect(view.synchronization).toBe('ready')
    expect(view.messages.side).toEqual([
      expect.objectContaining({ status: 'complete', content: expect.stringMatching(/\S/) }),
    ])
  })
})

describe('WebSocket client recovery', () => {
  it('bounds backoff, heartbeats, reconnect duration and queued controls', () => {
    vi.useFakeTimers()
    const sockets: FakeSocket[] = []
    let elapsed = 0
    const states: string[] = []
    const client = createWebSocketClient({
      createSocket: () => {
        const socket = new FakeSocket()
        sockets.push(socket)
        return socket
      },
      now: () => elapsed,
      random: () => 0.5,
      onState: (state) => states.push(state),
      createId: () => `client-${sockets.length}-${states.length}`,
    })

    expect(reconnectDelay(0, () => 0.5)).toBe(250)
    expect(reconnectDelay(99, () => 0.5)).toBe(5_000)
    client.connect()
    expect(() => {
      for (let index = 0; index < 51; index += 1) {
        client.send({ v: 1, id: `queued-${index}`, type: 'policy.subscribe', sentAt: now, payload: { deviceId: 'mac-1' } })
      }
    }).toThrowError(expect.objectContaining({ code: 'OUTBOUND_QUEUE_FULL' }))

    sockets[0]!.open()
    expect(sockets[0]!.sent).toHaveLength(50)
    vi.advanceTimersByTime(15_000)
    expect(decodeEnvelope(sockets[0]!.sent.at(-1)!).type).toBe('heartbeat')

    sockets[0]!.close()
    vi.advanceTimersByTime(250)
    expect(sockets).toHaveLength(2)
    elapsed = 60_001
    sockets[1]!.close()
    expect(states.at(-1)).toBe('offline')
    expect(client.state()).toBe('offline')

    client.recover()
    expect(sockets).toHaveLength(3)
    client.stop()
    vi.useRealTimers()
  })
})
