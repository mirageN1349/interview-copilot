import { ProtocolError, WS_ENDPOINT, decodeEnvelope, encodeEnvelope, type Envelope } from './protocol'

export type ConnectionState = 'idle' | 'connecting' | 'online' | 'reconnecting' | 'offline' | 'stopped' | 'fatal'
export interface SocketLike extends EventTarget {
  readonly CONNECTING: number
  readonly OPEN: number
  readonly CLOSING: number
  readonly CLOSED: number
  readonly readyState: number
  send(data: string): void
  close(code?: number, reason?: string): void
}

type ClientOptions = {
  createSocket?: (url: string) => SocketLike
  now?: () => number
  random?: () => number
  createId?: () => string
  onEnvelope?: (envelope: Envelope) => void
  onState?: (state: ConnectionState) => void
  onError?: (error: ProtocolError) => void
}

const BACKOFF = [250, 500, 1_000, 2_000, 4_000, 5_000]

export function reconnectDelay(attempt: number, random = Math.random): number {
  const base = BACKOFF[Math.min(Math.max(0, attempt), BACKOFF.length - 1)]!
  return Math.min(5_000, Math.round(base * (0.9 + random() * 0.2)))
}

export function createWebSocketClient(options: ClientOptions = {}) {
  const createSocket = options.createSocket ?? ((url) => new WebSocket(url))
  const now = options.now ?? Date.now
  const random = options.random ?? Math.random
  const createId = options.createId ?? (() => crypto.randomUUID())
  let connectionState: ConnectionState = 'idle'
  let socket: SocketLike | undefined
  let reconnectAttempt = 0
  let reconnectStartedAt: number | undefined
  let reconnectTimer: ReturnType<typeof setTimeout> | undefined
  let heartbeatTimer: ReturnType<typeof setInterval> | undefined
  let ackTimer: ReturnType<typeof setTimeout> | undefined
  let missedAcks = 0
  let stopped = false
  const queue: string[] = []

  function setState(state: ConnectionState) {
    connectionState = state
    options.onState?.(state)
  }

  function clearTimers() {
    if (reconnectTimer) clearTimeout(reconnectTimer)
    if (heartbeatTimer) clearInterval(heartbeatTimer)
    if (ackTimer) clearTimeout(ackTimer)
    reconnectTimer = heartbeatTimer = ackTimer = undefined
  }

  function sendHeartbeat() {
    if (!socket || socket.readyState !== socket.OPEN) return
    socket.send(encodeEnvelope({
      v: 1,
      id: createId(),
      type: 'heartbeat',
      sentAt: new Date().toISOString(),
      payload: {},
    }))
    if (ackTimer) clearTimeout(ackTimer)
    ackTimer = setTimeout(() => {
      missedAcks += 1
      if (missedAcks >= 2) socket?.close(4000, 'heartbeat timeout')
    }, 5_000)
  }

  function scheduleReconnect() {
    clearTimers()
    if (stopped) return
    reconnectStartedAt ??= now()
    if (now() - reconnectStartedAt > 60_000) {
      setState('offline')
      return
    }
    setState('reconnecting')
    reconnectTimer = setTimeout(open, reconnectDelay(reconnectAttempt++, random))
  }

  function open() {
    if (stopped) return
    setState(reconnectStartedAt !== undefined ? 'reconnecting' : 'connecting')
    socket = createSocket(WS_ENDPOINT)
    socket.addEventListener('open', () => {
      reconnectAttempt = 0
      reconnectStartedAt = undefined
      missedAcks = 0
      setState('online')
      while (queue.length) socket?.send(queue.shift()!)
      heartbeatTimer = setInterval(sendHeartbeat, 15_000)
    })
    socket.addEventListener('message', (event) => {
      try {
        const envelope = decodeEnvelope((event as MessageEvent).data)
        if (envelope.type === 'heartbeat.ack') {
          missedAcks = 0
          if (ackTimer) clearTimeout(ackTimer)
          ackTimer = undefined
        }
        options.onEnvelope?.(envelope)
      } catch (error) {
        const protocolError = error instanceof ProtocolError ? error : new ProtocolError('ENVELOPE_INVALID', 'Invalid server frame')
        options.onError?.(protocolError)
        if (protocolError.code === 'PROTOCOL_VERSION_UNSUPPORTED') {
          stopped = true
          clearTimers()
          socket?.close(4001, 'unsupported protocol')
          setState('fatal')
        }
      }
    })
    socket.addEventListener('close', scheduleReconnect, { once: true })
    socket.addEventListener('error', () => socket?.close(1011, 'transport error'), { once: true })
  }

  return {
    connect: open,
    send(envelope: Envelope) {
      const encoded = encodeEnvelope(envelope)
      if (socket?.readyState === socket?.OPEN) {
        socket.send(encoded)
        return
      }
      if (envelope.type === 'audio.fragment.ready') throw new ProtocolError('OFFLINE_MEDIA_NOT_QUEUED', 'Media commands are not queued')
      if (queue.length >= 50) throw new ProtocolError('OUTBOUND_QUEUE_FULL', 'Outbound control queue is full')
      queue.push(encoded)
    },
    recover() {
      if (connectionState !== 'offline') return
      reconnectStartedAt = undefined
      reconnectAttempt = 0
      open()
    },
    stop() {
      stopped = true
      clearTimers()
      socket?.close(1000, 'client stopped')
      setState('stopped')
    },
    state: () => connectionState,
    queued: () => queue.length,
  }
}
