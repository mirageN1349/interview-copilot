import type { MeetingActionId } from './keyboard-store'

type ActionContext = { meetingId: string }
type MeetingAction = (context: ActionContext) => Promise<unknown> | unknown

export function createMeetingActionRegistry(dependencies: {
  overlayToggle: MeetingAction
  overlayInteractive: MeetingAction
  focusLive: MeetingAction
  focusSide: MeetingAction
  requestContextReset: MeetingAction
  captureFull: MeetingAction
  captureArea: MeetingAction
  stop: MeetingAction
  emergencyStop: MeetingAction
}) {
  const actions: Record<MeetingActionId, MeetingAction> = {
    'overlay.toggle': dependencies.overlayToggle,
    'overlay.interactive': dependencies.overlayInteractive,
    'chat.live.focus': dependencies.focusLive,
    'chat.side.focus': dependencies.focusSide,
    'context.reset': dependencies.requestContextReset,
    'capture.full': dependencies.captureFull,
    'capture.area': dependencies.captureArea,
    'meeting.stop': dependencies.stop,
    'meeting.emergency-stop': dependencies.emergencyStop,
  }
  return {
    dispatch(action: MeetingActionId, context: ActionContext) {
      return actions[action](context)
    },
    ids: Object.keys(actions) as MeetingActionId[],
  }
}
