import { defineStore } from 'pinia'

export type OverlaySection = 'live' | 'side' | 'design' | 'status'
export type OverlayVisibility = 'hidden' | 'visible_passive' | 'visible_interactive'

export const useOverlayUiStore = defineStore('overlay-ui', {
  state: () => ({
    visibility: 'visible_interactive' as OverlayVisibility,
    activeSection: 'live' as OverlaySection,
    compact: false,
    position: { x: 24, y: 24 },
    lastInteractiveControl: 'overlay-live-input',
  }),
  actions: {
    show(interactive = false) {
      this.visibility = interactive ? 'visible_interactive' : 'visible_passive'
    },
    hide() {
      this.visibility = 'hidden'
    },
    setSection(section: OverlaySection) {
      this.activeSection = section
    },
    move(dx: number, dy: number) {
      this.position = { x: this.position.x + dx, y: this.position.y + dy }
    },
  },
})
