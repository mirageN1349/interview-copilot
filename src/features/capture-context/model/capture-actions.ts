import type { ChatThread } from '@/features/overlay-chat/model/meeting-reducer'
import type { AnswerDepth } from '@/entities/interview-profile'
import type { CaptureArea } from './capture-ui-store'

export type ScreenshotArtifact = {
  id: string
  contentStatus: 'allowed' | 'redacted'
  redactionSummary?: string | null
  displayId?: number
  width?: number
  height?: number
  createdAtMs?: number
}

type CaptureInput = {
  meetingId: string
  displayId: number
  mode: 'display' | 'area'
  area?: CaptureArea
  thread: ChatThread
}

type SendInput = {
  meetingId: string
  thread: ChatThread
  content: string
  artifactIds: string[]
  contextGeneration: number
  answerDepth?: AnswerDepth
}

export function createCaptureActions(dependencies: {
  capture: (input: CaptureInput) => Promise<Record<string, unknown>>
  send: (input: SendInput) => Promise<unknown>
}) {
  async function capture(input: CaptureInput): Promise<ScreenshotArtifact> {
    if (!Number.isSafeInteger(input.displayId) || input.displayId <= 0) throw new TypeError('Invalid display')
    if (input.mode === 'area' && !input.area) throw new TypeError('Capture area is required')
    const artifact = await dependencies.capture(input)
    if (artifact.contentStatus !== 'allowed' && artifact.contentStatus !== 'redacted') throw new Error('Screenshot is not attachable')
    if (typeof artifact.id !== 'string' || !/^[a-zA-Z0-9._-]{1,128}$/.test(artifact.id)) throw new Error('Screenshot ID is invalid')
    return artifact as ScreenshotArtifact
  }

  return {
    capture,
    async sendWithContext(input: Omit<SendInput, 'artifactIds'> & Omit<CaptureInput, 'thread' | 'meetingId'>): Promise<ScreenshotArtifact> {
      const artifact = await capture({
        meetingId: input.meetingId,
        displayId: input.displayId,
        mode: input.mode,
        ...(input.area ? { area: input.area } : {}),
        thread: input.thread,
      })
      await dependencies.send({
        meetingId: input.meetingId,
        thread: input.thread,
        content: input.content,
        artifactIds: [artifact.id],
        contextGeneration: input.contextGeneration,
        ...(input.answerDepth ? { answerDepth: input.answerDepth } : {}),
      })
      return artifact
    },
  }
}
