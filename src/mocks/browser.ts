import { setupWorker } from 'msw/browser'
import type { RequestHandler } from 'msw'
import { createMswHandlers } from './handlers'
import type { ScenarioRuntime } from './scenarios/runtime'

export function createBrowserMock(runtime: ScenarioRuntime, extraHandlers: RequestHandler[] = []) {
  const worker = setupWorker(...createMswHandlers(runtime), ...extraHandlers)
  return { start: () => worker.start({ onUnhandledRequest: 'error' }), stop: () => worker.stop() }
}
