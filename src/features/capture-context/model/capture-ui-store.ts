import { defineStore } from 'pinia'

export type CaptureArea = { x: number; y: number; width: number; height: number }
export type ScreenshotMode = 'off' | 'display' | 'area'

export const useCaptureUiStore = defineStore('capture-ui', {
  state: () => ({
    selectedDisplayId: null as number | null,
    backingScale: 1,
    areaDraft: null as CaptureArea | null,
    autoScreenshotMode: 'off' as ScreenshotMode,
    soundThreshold: 0.18,
    lastRedactionSummary: null as string | null,
  }),
  actions: {
    selectDisplay(displayId: number, backingScale: number) {
      if (!Number.isSafeInteger(displayId) || displayId <= 0 || !Number.isFinite(backingScale) || backingScale <= 0) throw new TypeError('Invalid display')
      this.selectedDisplayId = displayId
      this.backingScale = backingScale
      this.areaDraft = null
    },
    setArea(area: CaptureArea) {
      if (![area.x, area.y, area.width, area.height].every(Number.isFinite) || area.width <= 0 || area.height <= 0) throw new TypeError('Invalid capture area')
      this.areaDraft = { ...area }
    },
  },
})
