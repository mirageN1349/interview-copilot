import { ws } from 'msw'

import { WS_ENDPOINT } from '@/shared/api/ws/protocol'
import { createMeetingScenario } from '../scenarios/ws'

export function createMeetingWebSocketHandler(scenario = createMeetingScenario()) {
  const meeting = ws.link(WS_ENDPOINT)
  return meeting.addEventListener('connection', ({ client }) => {
    client.addEventListener('message', (event) => {
      event.preventDefault()
      for (const response of scenario.receive(event.data)) client.send(response)
    })
  })
}

export const meetingWebSocketHandler = createMeetingWebSocketHandler()
