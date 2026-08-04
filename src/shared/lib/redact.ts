const SENSITIVE_KEYS = /^(authorization|cookie|email|password|path|token|secret)$/i
const EMAIL = /\b[^\s@]+@[^\s@]+\.[^\s@]+\b/g
const LOCAL_PATH = /(?:\/Users|\/home|[A-Za-z]:\\)[^\s]*/g

export function redact(value: unknown): unknown {
  if (typeof value === 'string') return value.replace(EMAIL, '[REDACTED]').replace(LOCAL_PATH, '[REDACTED]')
  if (Array.isArray(value)) return value.map(redact)
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [
      key,
      SENSITIVE_KEYS.test(key) ? '[REDACTED]' : redact(item),
    ]))
  }
  return value
}
