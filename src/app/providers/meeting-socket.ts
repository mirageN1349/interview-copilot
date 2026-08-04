import { emit, listen, type UnlistenFn } from '@tauri-apps/api/event'

import { createWebSocketClient, type SocketLike } from '@/shared/api/ws/client'
import { createPolicyController, type PolicySnapshot } from '@/shared/api/ws/policy'
import type { Envelope } from '@/shared/api/ws/protocol'
import { nativeGateway } from '@/shared/api/native'

type MeetingSocket = ReturnType<typeof createWebSocketClient>
let socket: MeetingSocket | null = null
const envelopeHistory: Envelope[] = []
let commandBridgeReady = false
let policyTimer: ReturnType<typeof setInterval> | undefined

function createRuntimePolicy() {
  return createPolicyController({
    stopAll: () => nativeGateway.invoke('emergency_stop_all_fail_closed'),
    cancelPending: () => window.dispatchEvent(new CustomEvent('assistant-cancel-pending')),
    emitAudit: () => undefined,
  })
}
let policy = createRuntimePolicy()

function policySnapshot(payload: Record<string, unknown>): PolicySnapshot | null {
  if (typeof payload.policyVersion !== 'string' || !Number.isSafeInteger(payload.expiresAtMs)
    || typeof payload.verified !== 'boolean'
    || !['clear', 'stop_new', 'stop_all'].includes(String(payload.killSwitch))) return null
  return {
    policyVersion: payload.policyVersion,
    expiresAtMs: Number(payload.expiresAtMs),
    verified: payload.verified,
    killSwitch: payload.killSwitch as PolicySnapshot['killSwitch'],
  }
}

export function initializeMeetingSocket(createSocket?: (url: string) => SocketLike): MeetingSocket {
  if (socket) return socket
  socket = createWebSocketClient({
    createSocket,
    onEnvelope(envelope) {
      if (envelope.type === 'policy.snapshot') {
        const snapshot = policySnapshot(envelope.payload)
        if (snapshot) void policy.apply(snapshot)
      }
      if (envelope.type === 'kill_switch.activated') {
        void policy.apply({ policyVersion: 'remote', expiresAtMs: Date.now() + 1, verified: true, killSwitch: 'stop_all' })
      }
      envelopeHistory.push(envelope)
      if (envelopeHistory.length > 100) envelopeHistory.shift()
      if ('__TAURI_INTERNALS__' in window) void emit('assistant://envelope', envelope)
      else window.dispatchEvent(new CustomEvent('assistant-envelope', { detail: envelope }))
    },
    onState(state) {
      if (state === 'online') {
        socket?.send({
          v: 1, id: crypto.randomUUID(), type: 'policy.subscribe', sentAt: new Date().toISOString(),
          payload: { deviceId: 'local-desktop' },
        })
      }
      if (state === 'offline' || state === 'fatal') void policy.transportLost()
    },
  })
  socket.connect()
  policyTimer ??= setInterval(() => { void policy.tick() }, 1_000)
  if ('__TAURI_INTERNALS__' in window && !commandBridgeReady) {
    commandBridgeReady = true
    void listen<Envelope>('assistant://command', (event) => socket?.send(event.payload))
  }
  return socket
}

export function sendMeetingEnvelope(envelope: Envelope): void {
  if (!socket && '__TAURI_INTERNALS__' in window) {
    void emit('assistant://command', envelope)
    return
  }
  if (!socket) throw new Error('Meeting connection is not initialized')
  socket.send(envelope)
}

export async function subscribeMeetingEnvelopes(handler: (envelope: Envelope) => void): Promise<UnlistenFn> {
  for (const envelope of envelopeHistory) handler(envelope)
  if ('__TAURI_INTERNALS__' in window) {
    return listen<Envelope>('assistant://envelope', (event) => handler(event.payload))
  }
  const listener = (event: Event) => handler((event as CustomEvent<Envelope>).detail)
  window.addEventListener('assistant-envelope', listener)
  return () => window.removeEventListener('assistant-envelope', listener)
}

export function stopMeetingSocket(): void {
  socket?.stop()
  socket = null
  if (policyTimer) clearInterval(policyTimer)
  policyTimer = undefined
  policy = createRuntimePolicy()
}
