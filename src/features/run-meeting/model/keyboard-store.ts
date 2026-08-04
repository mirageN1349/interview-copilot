import { defineStore } from 'pinia'

export type MeetingActionId =
  | 'overlay.toggle'
  | 'overlay.interactive'
  | 'chat.live.focus'
  | 'chat.side.focus'
  | 'context.reset'
  | 'capture.full'
  | 'capture.area'
  | 'meeting.stop'
  | 'meeting.emergency-stop'

export type BindingState = {
  accelerator: string
  status: 'pending' | 'active' | 'conflict' | 'unbound'
  conflictWith?: string
}

const defaults: Record<MeetingActionId, string> = {
  'overlay.toggle': 'CommandOrControl+Shift+Space',
  'overlay.interactive': 'CommandOrControl+Shift+O',
  'chat.live.focus': 'CommandOrControl+Shift+L',
  'chat.side.focus': 'CommandOrControl+Shift+J',
  'context.reset': 'CommandOrControl+Shift+R',
  'capture.full': 'CommandOrControl+Shift+S',
  'capture.area': 'CommandOrControl+Shift+A',
  'meeting.stop': 'CommandOrControl+Shift+Escape',
  'meeting.emergency-stop': 'CommandOrControl+Shift+Escape',
}

export function defaultMeetingBindings(): Record<MeetingActionId, string> {
  return { ...defaults }
}

export const useKeyboardStore = defineStore('meeting-keyboard', {
  state: () => ({
    bindings: Object.fromEntries(Object.entries(defaults).map(([action, accelerator]) => [action, { accelerator, status: 'pending' }])) as Record<MeetingActionId, BindingState>,
    commandPaletteOpen: false,
    focusReturnTarget: null as string | null,
  }),
  actions: {
    registrationResult(action: MeetingActionId, result: { registered: boolean; conflictWith?: string }) {
      this.bindings[action] = result.registered
        ? { ...this.bindings[action], status: 'active', conflictWith: undefined }
        : { ...this.bindings[action], status: 'conflict', conflictWith: result.conflictWith }
    },
    remap(action: MeetingActionId, accelerator: string) {
      this.bindings[action] = { accelerator, status: 'pending' }
    },
  },
})
