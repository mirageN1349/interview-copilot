import { parseApiEnvelope, parseApiErrorEnvelope, type ApiError } from './contracts/common'

export class ApiRequestError extends Error {
  constructor(public readonly detail: ApiError, public readonly requestId: string) {
    super(detail.message)
  }
}

type GatewayOptions = { fetch?: typeof fetch; maxResponseBytes?: number }

export function createHttpGateway(options: GatewayOptions = {}) {
  const request = options.fetch ?? globalThis.fetch
  const maxResponseBytes = options.maxResponseBytes ?? 1_000_000

  async function send<T>(method: string, path: string, body?: unknown): Promise<T> {
    if (!path.startsWith('/') || path.startsWith('//')) throw new TypeError('HTTP gateway accepts same-origin paths only')
    const response = await request(path, {
      method,
      headers: body === undefined ? undefined : { 'content-type': 'application/json' },
      body: body === undefined ? undefined : JSON.stringify(body),
    })
    const text = await response.text()
    if (text.length > maxResponseBytes) throw new TypeError('HTTP response exceeds size limit')
    const payload: unknown = JSON.parse(text)
    if (!response.ok) {
      const failure = parseApiErrorEnvelope(payload)
      throw new ApiRequestError(failure.error, failure.requestId)
    }
    return parseApiEnvelope<T>(payload).data
  }

  return {
    get: <T>(path: string) => send<T>('GET', path),
    post: <T>(path: string, body: unknown) => send<T>('POST', path, body),
  }
}
