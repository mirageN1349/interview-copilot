import { createScenarioRuntime, type ScenarioDefinition, type ScenarioResponse } from './runtime'

type TokenRecord = { email: string; expiresAt: number; used: boolean }
type Session = {
  user: { id: string; email: string; name: string }
  session: { id: string; expiresAt: string }
}

export type AuthScenarioOptions = {
  allowedEmails?: string[]
  now?: () => number
  token?: () => string
  storage?: Pick<Storage, 'getItem' | 'setItem' | 'removeItem'>
}

const SESSION_KEY = 'interview-copilot.mock-session'

const error = (status: number, code: string, message: string): ScenarioResponse => ({
  status,
  body: { code, message },
})

const validCallback = (value: unknown): value is string =>
  typeof value === 'string' && /^#\/[a-z0-9/_-]*$/i.test(value)

async function sha256(value: string) {
  const bytes = new TextEncoder().encode(value)
  const digest = await crypto.subtle.digest('SHA-256', bytes)
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, '0')).join('')
}

export function createAuthScenario(options: AuthScenarioOptions = {}) {
  const allowedEmails = new Set((options.allowedEmails ?? ['user@example.test']).map((email) => email.toLowerCase()))
  const now = options.now ?? Date.now
  const token = options.token ?? (() => crypto.randomUUID().replaceAll('-', ''))
  const tokens = new Map<string, TokenRecord>()
  const inbox: Array<{ email: string; token: string; url: string }> = []
  const storage = options.storage

  function clearSession() {
    session = null
    try { storage?.removeItem(SESSION_KEY) } catch { /* local mock remains usable without persistence */ }
  }

  function persistSession() {
    if (!session) return
    try {
      storage?.setItem(SESSION_KEY, JSON.stringify({ email: session.user.email, expiresAt: session.session.expiresAt }))
    } catch { /* local mock remains usable without persistence */ }
  }

  function restoreSession(): Session | null {
    try {
      const raw = storage?.getItem(SESSION_KEY)
      if (!raw) return null
      const value = JSON.parse(raw) as { email?: unknown; expiresAt?: unknown }
      const email = typeof value.email === 'string' ? value.email.toLowerCase() : ''
      const expiresAt = typeof value.expiresAt === 'string' ? value.expiresAt : ''
      if (!allowedEmails.has(email) || !Number.isFinite(Date.parse(expiresAt)) || Date.parse(expiresAt) <= now()) {
        storage?.removeItem(SESSION_KEY)
        return null
      }
      return { user: { id: '019-user', email, name: 'Alex Morgan' }, session: { id: '019-session', expiresAt } }
    } catch {
      try { storage?.removeItem(SESSION_KEY) } catch { /* ignore unavailable storage */ }
      return null
    }
  }

  let session: Session | null = restoreSession()

  const definitions: ScenarioDefinition[] = [
    {
      method: 'POST',
      path: '/api/auth/sign-in/magic-link',
      async resolve(request) {
        const body = request.body as { email?: unknown; callbackURL?: unknown } | undefined
        const email = typeof body?.email === 'string' ? body.email.trim().toLowerCase() : ''
        if (!allowedEmails.has(email)) return error(403, 'AUTH_EMAIL_NOT_ALLOWED', 'Use an approved demo email.')
        if (!validCallback(body?.callbackURL)) return error(400, 'AUTH_CALLBACK_INVALID', 'Return route is not allowed.')

        const rawToken = token()
        const tokenHash = await sha256(rawToken)
        tokens.set(tokenHash, { email, expiresAt: now() + 10 * 60_000, used: false })
        const query = new URLSearchParams({ token: rawToken, callbackURL: body.callbackURL })
        inbox.splice(0, inbox.length, { email, token: rawToken, url: `#/auth/verify?${query}` })
        return { status: 200, body: { status: true } }
      },
    },
    {
      method: 'GET',
      path: '/api/auth/magic-link/verify',
      async resolve(request) {
        const url = new URL(request.url, 'https://app.local')
        const rawToken = url.searchParams.get('token')
        const callbackURL = url.searchParams.get('callbackURL') ?? '#/auth/verify'
        if (!rawToken) return error(400, 'AUTH_TOKEN_MISSING', 'This sign-in link is incomplete.')
        if (!validCallback(callbackURL)) return error(400, 'AUTH_CALLBACK_INVALID', 'Return route is not allowed.')

        const record = tokens.get(await sha256(rawToken))
        if (!record) return error(401, 'AUTH_TOKEN_INVALID', 'This sign-in link is invalid.')
        if (record.used) return error(409, 'AUTH_TOKEN_USED', 'This sign-in link was already used.')
        if (record.expiresAt < now()) return error(401, 'AUTH_TOKEN_EXPIRED', 'This sign-in link has expired.')

        record.used = true
        session = {
          user: { id: '019-user', email: record.email, name: 'Alex Morgan' },
          session: { id: '019-session', expiresAt: new Date(now() + 6 * 60 * 60_000).toISOString() },
        }
        persistSession()
        return { status: 302, body: null, headers: { location: callbackURL } }
      },
    },
    {
      method: 'GET',
      path: '/api/auth/get-session',
      resolve() {
        if (session && Date.parse(session.session.expiresAt) <= now()) clearSession()
        return { status: 200, body: session }
      },
    },
    {
      method: 'POST',
      path: '/api/auth/sign-out',
      resolve() {
        clearSession()
        return { status: 200, body: { success: true } }
      },
    },
  ]

  return {
    definitions,
    runtime: createScenarioRuntime(definitions),
    devInbox: () => inbox.map((message) => ({ ...message })),
    inspect: () => ({ tokenHashes: [...tokens.keys()], hasSession: session !== null }),
    reset() {
      tokens.clear()
      inbox.length = 0
      clearSession()
    },
  }
}

export const authScenario = createAuthScenario({ storage: typeof localStorage === 'undefined' ? undefined : localStorage })
