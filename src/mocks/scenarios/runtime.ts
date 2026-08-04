export type ScenarioRequest = {
  method: string
  url: string
  headers?: Record<string, string>
  body?: unknown
}

export type ScenarioContext = ScenarioRequest & { path: string; requestId: string }
export type ScenarioResponse = { status: number; body: unknown; headers?: Record<string, string> }
export type ScenarioDefinition = {
  method: string
  path: string
  resolve: (request: ScenarioContext) => ScenarioResponse | Promise<ScenarioResponse>
}

export type ScenarioRuntime = ReturnType<typeof createScenarioRuntime>

export function createScenarioRuntime(definitions: ScenarioDefinition[]) {
  const routes = new Map(definitions.map((definition) => [
    `${definition.method.toUpperCase()} ${definition.path}`,
    definition.resolve,
  ]))

  return {
    async resolve(request: ScenarioRequest): Promise<ScenarioResponse> {
      const method = request.method.toUpperCase()
      const path = new URL(request.url, 'https://app.local').pathname
      const resolver = routes.get(`${method} ${path}`)
      if (!resolver) throw new Error(`Unhandled mock request: ${method} ${path}`)
      return resolver({ ...request, method, path, requestId: `req-${method.toLowerCase()}-${path.replace(/\W+/g, '-').replace(/^-|-$/g, '')}` })
    },
  }
}
