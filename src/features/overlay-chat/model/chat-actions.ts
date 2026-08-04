type StopReason = 'user' | 'kill_switch' | 'policy_lost' | 'error'
import type { ChatThread } from './meeting-reducer'
import type { Envelope } from '@/shared/api/ws/protocol'

export function createDevAudioLoop(emit: () => void, intervalMs = 7_000) {
  let active = false
  let timer: ReturnType<typeof setTimeout> | undefined

  function schedule(delay: number) {
    if (!active || timer) return
    timer = setTimeout(() => {
      timer = undefined
      if (!active) return
      emit()
      schedule(intervalMs)
    }, delay)
  }

  return {
    start(initialDelayMs = 1_500) {
      if (active) return
      active = true
      schedule(initialDelayMs)
    },
    stop() {
      active = false
      if (timer) clearTimeout(timer)
      timer = undefined
    },
  }
}

export function createChatActions<T>(dependencies: {
  nativeStop: (meetingId: string, reason: StopReason) => Promise<T>
  socketSend: (message: unknown) => Promise<unknown>
}) {
  return {
    async stop(meetingId: string, reason: StopReason): Promise<T> {
      const result = await dependencies.nativeStop(meetingId, reason)
      await dependencies.socketSend({ type: 'meeting.stop', meetingId, payload: { reason } }).catch(() => undefined)
      return result
    },
  }
}

export function createMeetingChatActions(dependencies: {
  send: (envelope: Envelope) => void
  resetNative: (meetingId: string) => Promise<number>
  createId?: () => string
  now?: () => Date
}) {
  const createId = dependencies.createId ?? (() => crypto.randomUUID())
  const now = dependencies.now ?? (() => new Date())

  function envelope(meetingId: string, launchPolicyId: string, type: string, payload: Record<string, unknown>): Envelope {
    return { v: 1, id: createId(), type, sentAt: now().toISOString(), meetingId, launchPolicyId, payload }
  }

  return {
    sendMessage(input: { meetingId: string; launchPolicyId: string; thread: ChatThread; content: string; artifactIds?: string[]; contextGeneration: number; answerDepth?: 'brief' | 'balanced' | 'detailed' }) {
      const content = input.content.trim()
      if (!content || content.length > 16_000) throw new TypeError('Chat content is invalid')
      const artifactIds = input.artifactIds ?? []
      if (artifactIds.length > 20 || artifactIds.some((id) => !/^[a-zA-Z0-9._-]{1,128}$/.test(id))) throw new TypeError('Chat artifacts are invalid')
      dependencies.send(envelope(input.meetingId, input.launchPolicyId, 'chat.send', {
        thread: input.thread,
        content,
        artifactIds,
        contextGeneration: input.contextGeneration,
        ...(input.answerDepth ? { answerDepth: input.answerDepth } : {}),
      }))
    },
    async resetContext(input: { meetingId: string; launchPolicyId: string; currentGeneration: number }) {
      const generation = await dependencies.resetNative(input.meetingId)
      if (generation !== input.currentGeneration + 1) throw new Error('Context generation diverged')
      dependencies.send(envelope(input.meetingId, input.launchPolicyId, 'context.reset', { expectedGeneration: input.currentGeneration }))
      return generation
    },
  }
}
