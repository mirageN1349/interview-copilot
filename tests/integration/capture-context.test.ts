import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { createCaptureActions } from '@/features/capture-context/model/capture-actions'
import { useCaptureUiStore } from '@/features/capture-context/model/capture-ui-store'

describe('capture context', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('keeps monitor, area and screenshot mode as local UI state', () => {
    const store = useCaptureUiStore()
    store.selectDisplay(42, 2)
    store.setArea({ x: -1200, y: 80, width: 900, height: 600 })
    store.autoScreenshotMode = 'area'

    expect(store.$state).toMatchObject({
      selectedDisplayId: 42,
      backingScale: 2,
      areaDraft: { x: -1200, y: 80, width: 900, height: 600 },
      autoScreenshotMode: 'area',
    })
  })

  it('attaches only vetted screenshot IDs and exposes redaction decisions', async () => {
    const capture = vi.fn().mockResolvedValue({
      id: 'artifact-redacted', contentStatus: 'redacted', redactionSummary: 'Sensitive text removed',
      displayId: 42, width: 1800, height: 1200, createdAtMs: 1,
    })
    const send = vi.fn().mockResolvedValue(undefined)
    const actions = createCaptureActions({ capture, send })

    const artifact = await actions.sendWithContext({
      meetingId: 'meeting-1', thread: 'side', content: 'What is wrong here?', contextGeneration: 0,
      displayId: 42, mode: 'area', area: { x: 10, y: 20, width: 900, height: 600 },
    })

    expect(send).toHaveBeenCalledWith(expect.objectContaining({ artifactIds: ['artifact-redacted'] }))
    expect(artifact.redactionSummary).toBe('Sensitive text removed')
  })

  it('never attaches pending or rejected captures', async () => {
    const actions = createCaptureActions({
      capture: vi.fn().mockResolvedValue({ id: 'pending', contentStatus: 'pending' }),
      send: vi.fn(),
    })
    await expect(actions.capture({ meetingId: 'meeting-1', displayId: 1, mode: 'display', thread: 'live' }))
      .rejects.toThrow('Screenshot is not attachable')
  })
})
