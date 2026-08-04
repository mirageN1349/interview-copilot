import { describe, expect, it, vi } from 'vitest'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { createPinia, setActivePinia } from 'pinia'

import { createChatActions, createDevAudioLoop } from '@/features/overlay-chat/model/chat-actions'
import {
  applyMeetingEvent,
  createMeetingViewState,
} from '@/features/overlay-chat/model/meeting-reducer'
import { evaluateMeetingReadiness } from '@/pages/meeting/model/meeting-readiness'
import { useOverlayUiStore } from '@/features/overlay-chat/model/overlay-ui-store'
import { createInMemoryMeetingSocket, createMeetingScenario } from '@/mocks/scenarios/ws'
import { decodeEnvelope, encodeEnvelope, type Envelope } from '@/shared/api/ws/protocol'

describe('meeting overlay behavior', () => {
  it('runs one dev audio loop at a time and stops without a trailing tick', async () => {
    vi.useFakeTimers()
    const emit = vi.fn()
    const loop = createDevAudioLoop(emit, 1_000)

    loop.start(100)
    loop.start(100)
    await vi.advanceTimersByTimeAsync(2_100)
    expect(emit).toHaveBeenCalledTimes(3)

    loop.stop()
    await vi.advanceTimersByTimeAsync(5_000)
    expect(emit).toHaveBeenCalledTimes(3)
    vi.useRealTimers()
  })

  it('opens as an interactive, resizable panel with usable dimensions', () => {
    setActivePinia(createPinia())
    expect(useOverlayUiStore().visibility).toBe('visible_interactive')

    const config = JSON.parse(readFileSync(resolve('src-tauri/tauri.conf.json'), 'utf8'))
    const overlay = config.app.windows.find((window: { label: string }) => window.label === 'overlay')
    expect(overlay).toMatchObject({
      width: 840,
      height: 420,
      minWidth: 560,
      minHeight: 280,
      visible: false,
      resizable: true,
      transparent: true,
      backgroundColor: [0, 0, 0, 0],
      shadow: false,
    })

    const shell = readFileSync(resolve('src/features/overlay-chat/ui/OverlayShell.vue'), 'utf8')
    expect(shell).toContain("import { TabsIndicator, TabsList, TabsRoot, TabsTrigger } from 'reka-ui'")
    expect(shell).toContain('<TabsRoot')
    expect(shell).toContain('v-model="activeSection"')
    expect(shell).toContain('<TabsIndicator')
    expect(shell.match(/class="overlay-segment-indicator/g)).toHaveLength(1)
    expect(shell).toContain('width: var(--reka-tabs-indicator-size)')
    expect(shell).toContain('translate3d(var(--reka-tabs-indicator-position), 0, 0)')
    expect(shell).toContain('transition: transform 320ms cubic-bezier(0.22, 1, 0.36, 1), width 320ms')
    expect(shell).not.toContain('.overlay-segments[data-active=')
    expect(shell).toContain('name="overlay-content"')
    expect(shell).toContain('mode="out-in"')
    expect(shell).toContain('key="live"')
    expect(shell).toContain('key="status"')
    expect(shell).toContain('transition: opacity 210ms cubic-bezier(0.22, 1, 0.36, 1), transform 210ms')
    expect(shell).toContain('.overlay-content-enter-from')
    expect(shell).toContain('@media (prefers-reduced-motion: reduce)')
    expect(shell).toContain('class="overlay-chrome absolute inset-x-0 top-0')
    expect(shell).toContain('grid-rows-[1fr_auto]')
    expect(shell).toContain('backdrop-filter: blur(18px) saturate(135%)')
    expect(shell).not.toContain('.overlay-status,\n.overlay-segments')
    expect(shell).toContain('.overlay-segments {')
    expect(shell).toContain('background: color-mix(in oklch, var(--surface-raised) 32%, transparent)')
    expect(shell).toContain('.overlay-chrome::before {')
    expect(shell).toContain('mask-image: linear-gradient(to bottom, black 0 58%, transparent 100%)')
    expect(shell).toContain('.overlay-footer::before {')
    expect(shell).toContain('mask-image: linear-gradient(to bottom, transparent 0%, black 42% 100%)')
    expect(shell).not.toContain('overlay-divider')
    expect(shell).toContain('size-4 cursor-nwse-resize opacity-0')
    expect(shell).not.toContain('<Scaling')
    expect(shell).not.toContain('resize-dots')
    expect(shell).not.toContain('border-b-2 border-r-2 border-current')
    expect(shell).not.toContain('await getCurrentWindow().hide()')
    expect(shell).toContain('stopping.value = false\n    void getCurrentWindow().hide()')
    expect(shell).toContain('if (stopping.value) return')
    expect(shell.indexOf("meetingGateway.stop(meetingId.value, 'user')")).toBeLessThan(shell.indexOf('store.hide()'))
    expect(shell.indexOf('store.hide()')).toBeLessThan(shell.indexOf('void getCurrentWindow().hide()'))
    expect(shell).toContain('getCurrentWindow().startDragging()')
    expect(shell).toContain("nativeGateway.invoke('overlay_ready'")
    const mounted = shell.slice(shell.indexOf('onMounted(async () =>'))
    expect(mounted.indexOf('subscribeMeetingEnvelopes(applyEnvelope)')).toBeLessThan(mounted.indexOf('resumeMeeting()'))

    const capability = JSON.parse(readFileSync(resolve('src-tauri/capabilities/overlay.json'), 'utf8'))
    expect(capability.permissions).toContain('core:window:allow-start-dragging')

    const html = readFileSync(resolve('index.html'), 'utf8')
    expect(html).toContain("location.hash.startsWith('#/overlay')")
    expect(html).toContain('html.overlay-window body')

    const glass = readFileSync(resolve('src-tauri/src/macos/glass.rs'), 'utf8')
    expect(glass).toContain('NSGlassEffectViewStyle::Regular')
    expect(glass).toContain('.tint_color((58, 64, 74, 72))')
    expect(glass).toContain('NSVisualEffectMaterial::HudWindow')
    const windowing = readFileSync(resolve('src-tauri/src/macos/windowing.rs'), 'utf8')
    expect(windowing).toContain('native.setOpaque(false)')
    expect(windowing).toContain('native.setBackgroundColor(Some(&NSColor::clearColor()))')
    expect(windowing).toContain('layer.setCornerRadius(22.0)')
    expect(windowing).toContain('layer.setMasksToBounds(true)')
  })

  it('keeps low-confidence questions as drafts and starts high-confidence answers', () => {
    const initial = createMeetingViewState('meeting-1', 0.72)
    const low = applyMeetingEvent(initial, {
      id: 'question-low', sequence: 1, type: 'question.detected',
      payload: { segmentId: 'segment-1', text: 'Could you repeat?', confidence: 0.61 },
    })
    expect(low.pendingQuestion).toMatchObject({ text: 'Could you repeat?', requiresConfirmation: true })
    expect(low.outbound).toEqual([])

    const high = applyMeetingEvent(low, {
      id: 'question-high', sequence: 2, type: 'question.detected',
      payload: { segmentId: 'segment-2', text: 'Tell me about your project', confidence: 0.94 },
    })
    expect(high.pendingQuestion).toMatchObject({ requiresConfirmation: false })
    expect(high.outbound).toEqual([{ kind: 'answer.request', segmentId: 'segment-2', contextGeneration: 0 }])
  })

  it('isolates live and side chat and preserves history across a context reset', () => {
    let state = createMeetingViewState('meeting-1', 0.72)
    state = applyMeetingEvent(state, {
      id: 'live-start', sequence: 1, type: 'answer.started',
      payload: { messageId: 'live-1', thread: 'live', profileSourceIds: ['source-active'] },
    })
    state = applyMeetingEvent(state, {
      id: 'live-delta', sequence: 2, type: 'answer.delta',
      payload: { messageId: 'live-1', delta: 'Live answer' },
    })
    state = applyMeetingEvent(state, {
      id: 'side-start', sequence: 3, type: 'answer.started',
      payload: { messageId: 'side-1', thread: 'side', profileSourceIds: [] },
    })
    state = applyMeetingEvent(state, {
      id: 'side-delta', sequence: 4, type: 'answer.delta',
      payload: { messageId: 'side-1', delta: 'Side answer' },
    })
    state = applyMeetingEvent(state, {
      id: 'reset', sequence: 5, type: 'context.reset.completed',
      payload: { generation: 1 },
    })

    expect(state.messages.live.map((message) => message.content)).toEqual(['Live answer'])
    expect(state.messages.side.map((message) => message.content)).toEqual(['Side answer'])
    expect(state.contextGeneration).toBe(1)
  })

  it('restores a fresh main-window socket before streaming side-chat events to the overlay window', async () => {
    vi.useFakeTimers()
    const bus = new EventTarget()
    const socket = createInMemoryMeetingSocket(createMeetingScenario())
    let view = createMeetingViewState('meeting-restored', 0.72)

    bus.addEventListener('assistant-command', (event) => {
      socket.send(encodeEnvelope((event as CustomEvent<Envelope>).detail))
    })
    socket.addEventListener('message', (event) => {
      bus.dispatchEvent(new CustomEvent('assistant-envelope', {
        detail: decodeEnvelope((event as MessageEvent).data),
      }))
    })
    bus.addEventListener('assistant-envelope', (event) => {
      const envelope = (event as CustomEvent<Envelope>).detail
      if (envelope.meetingId !== view.meetingId || envelope.sequence === undefined) return
      if (!['question.detected', 'answer.started', 'answer.delta', 'answer.completed', 'context.reset.completed'].includes(envelope.type)) {
        if (envelope.sequence === view.lastAppliedSequence + 1) {
          view = { ...view, lastAppliedSequence: envelope.sequence, appliedEventIds: [...view.appliedEventIds, envelope.id] }
        }
        return
      }
      view = applyMeetingEvent(view, {
        id: envelope.id,
        sequence: envelope.sequence,
        type: envelope.type as Parameters<typeof applyMeetingEvent>[1]['type'],
        payload: envelope.payload,
      })
    })

    await new Promise<void>((resolve) => socket.addEventListener('open', () => resolve(), { once: true }))
    const sendFromOverlay = (envelope: Envelope) => bus.dispatchEvent(new CustomEvent('assistant-command', { detail: envelope }))
    sendFromOverlay({
      v: 1, id: 'resume-restored', type: 'meeting.resume', sentAt: '2026-08-04T10:00:00.000Z',
      meetingId: view.meetingId, launchPolicyId: 'policy-1', payload: { lastAppliedSequence: 0, contextGeneration: 0 },
    })
    await vi.runAllTimersAsync()
    expect(view.lastAppliedSequence).toBe(1)

    sendFromOverlay({
      v: 1, id: 'chat-restored', type: 'chat.send', sentAt: '2026-08-04T10:00:00.000Z',
      meetingId: view.meetingId, launchPolicyId: 'policy-1',
      payload: { thread: 'side', content: 'Сформулируй краткий ответ', artifactIds: [], contextGeneration: 0 },
    })
    await vi.runAllTimersAsync()

    expect(view.synchronization).toBe('ready')
    expect(view.messages.side).toEqual([
      expect.objectContaining({ status: 'complete', content: expect.stringMatching(/\S/) }),
    ])
    socket.close()
    vi.useRealTimers()
  })

  it('stops locally even when the socket is offline', async () => {
    const nativeStop = vi.fn().mockResolvedValue({ status: 'completed' })
    const socketSend = vi.fn().mockRejectedValue(new Error('offline'))
    const actions = createChatActions({ nativeStop, socketSend })

    await expect(actions.stop('meeting-1', 'user')).resolves.toEqual({ status: 'completed' })
    expect(nativeStop).toHaveBeenCalledBefore(socketSend)
  })

  it('reports each missing readiness input without starting a meeting', () => {
    expect(evaluateMeetingReadiness({
      profileReady: false,
      responseModelAvailable: true,
      transcriptionModelAvailable: false,
      translationSupported: true,
      displaySelected: false,
      permissionsGranted: true,
      runGateAllowed: false,
    })).toEqual({
      ready: false,
      reasons: ['profile', 'transcription_model', 'display', 'meeting_unavailable'],
    })
  })
})
