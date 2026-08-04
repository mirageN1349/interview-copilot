export type ApiError = {
  code: string
  message: string
  retryable: boolean
  field?: string
}

export type ApiEnvelope<T> = { data: T; requestId: string }
export type ApiErrorEnvelope = { error: ApiError; requestId: string }

export type CommandError = {
  code: string
  message: string
  retryable: boolean
  recovery?: 'open_settings' | 'reselect_display' | 'rebind_hotkey' | 'restart_app'
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

export function readBoundedString(value: unknown, field: string, maxLength: number): string {
  if (typeof value !== 'string' || value.length === 0 || value.length > maxLength) {
    throw new TypeError(`${field} must be a non-empty string of at most ${maxLength} characters`)
  }
  return value
}

export function parseApiEnvelope<T>(value: unknown): ApiEnvelope<T> {
  if (!isRecord(value) || !('data' in value)) throw new TypeError('Invalid API success envelope')
  return {
    data: value.data as T,
    requestId: readBoundedString(value.requestId, 'requestId', 128),
  }
}

export function parseApiErrorEnvelope(value: unknown): ApiErrorEnvelope {
  if (!isRecord(value) || !isRecord(value.error)) throw new TypeError('Invalid API error envelope')
  const error = value.error
  if (typeof error.retryable !== 'boolean') throw new TypeError('error.retryable must be boolean')
  return {
    error: {
      code: readBoundedString(error.code, 'error.code', 64),
      message: readBoundedString(error.message, 'error.message', 512),
      retryable: error.retryable,
      ...(typeof error.field === 'string' ? { field: readBoundedString(error.field, 'error.field', 64) } : {}),
    },
    requestId: readBoundedString(value.requestId, 'requestId', 128),
  }
}

export function parseCommandError(value: unknown): CommandError {
  if (!isRecord(value)) return { code: 'NATIVE_COMMAND_FAILED', message: 'Native command failed', retryable: false }
  const recoveries = new Set<CommandError['recovery']>([
    'open_settings', 'reselect_display', 'rebind_hotkey', 'restart_app',
  ])
  const recovery = recoveries.has(value.recovery as CommandError['recovery'])
    ? value.recovery as CommandError['recovery']
    : undefined
  return {
    code: typeof value.code === 'string' ? value.code.slice(0, 64) : 'NATIVE_COMMAND_FAILED',
    message: typeof value.message === 'string' ? value.message.slice(0, 512) : 'Native command failed',
    retryable: value.retryable === true,
    ...(recovery ? { recovery } : {}),
  }
}
