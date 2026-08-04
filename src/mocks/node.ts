import { setupServer } from 'msw/node'
import type { RequestHandler } from 'msw'
import { createMswHandlers } from './handlers'
import type { ScenarioRuntime } from './scenarios/runtime'

export function createNodeMock(runtime: ScenarioRuntime, extraHandlers: RequestHandler[] = []) {
  return setupServer(...createMswHandlers(runtime), ...extraHandlers)
}
