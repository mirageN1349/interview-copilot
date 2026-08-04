import { createHash } from 'node:crypto'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { createAuthHandlers } from '@/mocks/handlers/auth'
import { createPackagedFetch } from '@/mocks/packaged'
import { createAuthScenario } from '@/mocks/scenarios/auth'
import { createInterviewAuthClient, setAuthTransport, verifyMagicLink } from '@/shared/api/auth/client'
import { server } from '../setup'

const allowedEmail = 'user@example.test'

async function json(response: Response) {
  return response.json() as Promise<Record<string, unknown>>
}

describe('Better Auth-shaped mock contract', () => {
  afterEach(() => {
    setAuthTransport(globalThis.fetch)
    vi.restoreAllMocks()
  })

  it('verifies links through the configured transport', async () => {
    const auth = createAuthScenario({ allowedEmails: [allowedEmail], token: () => 'configured-token' })
    await auth.runtime.resolve({
      method: 'POST',
      url: '/api/auth/sign-in/magic-link',
      body: { email: allowedEmail, callbackURL: '#/auth/verify' },
    })
    const transport = vi.fn(createPackagedFetch(auth.runtime))
    setAuthTransport(transport)

    await verifyMagicLink('configured-token')

    expect(transport).toHaveBeenCalledOnce()
  })

  it('restores only the mock session across app reloads', async () => {
    const values = new Map<string, string>()
    const storage = {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => { values.set(key, value) },
      removeItem: (key: string) => { values.delete(key) },
    }
    const options = { allowedEmails: [allowedEmail], token: () => 'reload-token', storage }
    const first = createAuthScenario(options)
    await first.runtime.resolve({ method: 'POST', url: '/api/auth/sign-in/magic-link', body: { email: allowedEmail, callbackURL: '#/auth/verify' } })
    await first.runtime.resolve({ method: 'GET', url: '/api/auth/magic-link/verify?token=reload-token&callbackURL=%23%2Fauth%2Fverify' })

    const restored = createAuthScenario(options)
    expect(await restored.runtime.resolve({ method: 'GET', url: '/api/auth/get-session' })).toMatchObject({ body: { user: { email: allowedEmail } } })
    expect([...values.values()].join()).not.toContain('reload-token')

    await restored.runtime.resolve({ method: 'POST', url: '/api/auth/sign-out' })
    expect((await createAuthScenario(options).runtime.resolve({ method: 'GET', url: '/api/auth/get-session' })).body).toBeNull()
  })

  it('normalizes allowlisted email and exposes the raw token only through the dev inbox', async () => {
    const auth = createAuthScenario({ allowedEmails: [allowedEmail], token: () => 'single-use-secret' })
    const response = await auth.runtime.resolve({
      method: 'POST',
      url: '/api/auth/sign-in/magic-link',
      body: { email: ' USER@EXAMPLE.TEST ', callbackURL: '#/auth/verify' },
    })

    expect(response).toMatchObject({ status: 200, body: { status: true } })
    expect(auth.devInbox()).toEqual([{ email: allowedEmail, token: 'single-use-secret', url: expect.stringContaining('token=single-use-secret') }])
    expect(auth.inspect().tokenHashes).toEqual([createHash('sha256').update('single-use-secret').digest('hex')])
    expect(JSON.stringify(response)).not.toContain('single-use-secret')
  })

  it('rejects identities outside the synthetic allowlist', async () => {
    const auth = createAuthScenario({ allowedEmails: [allowedEmail] })
    const response = await auth.runtime.resolve({
      method: 'POST',
      url: '/api/auth/sign-in/magic-link',
      body: { email: 'real@example.com', callbackURL: '#/auth/verify' },
    })
    expect(response).toMatchObject({ status: 403, body: { code: 'AUTH_EMAIL_NOT_ALLOWED' } })
  })

  it('rejects missing, malformed, expired and reused tokens without echoing them', async () => {
    let now = Date.parse('2026-08-04T12:00:00.000Z')
    const auth = createAuthScenario({ allowedEmails: [allowedEmail], now: () => now, token: () => 'secret-token' })
    const missing = await auth.runtime.resolve({ method: 'GET', url: '/api/auth/magic-link/verify?callbackURL=%23%2Fauth%2Fverify' })
    const malformed = await auth.runtime.resolve({ method: 'GET', url: '/api/auth/magic-link/verify?token=wrong&callbackURL=%23%2Fauth%2Fverify' })
    expect(missing).toMatchObject({ status: 400, body: { code: 'AUTH_TOKEN_MISSING' } })
    expect(malformed).toMatchObject({ status: 401, body: { code: 'AUTH_TOKEN_INVALID' } })
    expect(JSON.stringify(malformed)).not.toContain('wrong')

    await auth.runtime.resolve({ method: 'POST', url: '/api/auth/sign-in/magic-link', body: { email: allowedEmail, callbackURL: '#/auth/verify' } })
    now += 10 * 60_000 + 1
    const expired = await auth.runtime.resolve({ method: 'GET', url: '/api/auth/magic-link/verify?token=secret-token&callbackURL=%23%2Fauth%2Fverify' })
    expect(expired).toMatchObject({ status: 401, body: { code: 'AUTH_TOKEN_EXPIRED' } })

    now = Date.parse('2026-08-04T12:00:00.000Z')
    auth.reset()
    await auth.runtime.resolve({ method: 'POST', url: '/api/auth/sign-in/magic-link', body: { email: allowedEmail, callbackURL: '#/auth/verify' } })
    expect(await auth.runtime.resolve({ method: 'GET', url: '/api/auth/magic-link/verify?token=secret-token&callbackURL=%23%2Fauth%2Fverify' })).toMatchObject({ status: 302 })
    expect(await auth.runtime.resolve({ method: 'GET', url: '/api/auth/magic-link/verify?token=secret-token&callbackURL=%23%2Fauth%2Fverify' })).toMatchObject({ status: 409, body: { code: 'AUTH_TOKEN_USED' } })
  })

  it.each(['https://evil.example', 'javascript:alert(1)', 'file:///tmp/x', '/profiles'])(
    'rejects unsafe callback %s',
    async (callbackURL) => {
      const auth = createAuthScenario({ allowedEmails: [allowedEmail] })
      const response = await auth.runtime.resolve({
        method: 'POST',
        url: '/api/auth/sign-in/magic-link',
        body: { email: allowedEmail, callbackURL },
      })
      expect(response).toMatchObject({ status: 400, body: { code: 'AUTH_CALLBACK_INVALID' } })
    },
  )

  it('refreshes and expires the session, then signs out idempotently', async () => {
    let now = Date.parse('2026-08-04T12:00:00.000Z')
    const auth = createAuthScenario({ allowedEmails: [allowedEmail], now: () => now, token: () => 'ok-token' })
    await auth.runtime.resolve({ method: 'POST', url: '/api/auth/sign-in/magic-link', body: { email: allowedEmail, callbackURL: '#/auth/verify' } })
    await auth.runtime.resolve({ method: 'GET', url: '/api/auth/magic-link/verify?token=ok-token&callbackURL=%23%2Fauth%2Fverify' })
    expect(await auth.runtime.resolve({ method: 'GET', url: '/api/auth/get-session' })).toMatchObject({ status: 200, body: { user: { email: allowedEmail } } })
    now += 6 * 60 * 60_000 + 1
    expect(await auth.runtime.resolve({ method: 'GET', url: '/api/auth/get-session' })).toEqual({ status: 200, body: null })
    expect(await auth.runtime.resolve({ method: 'POST', url: '/api/auth/sign-out' })).toMatchObject({ status: 200, body: { success: true } })
  })

  it('returns identical DTOs through packaged and MSW adapters', async () => {
    const auth = createAuthScenario({ allowedEmails: [allowedEmail], token: () => 'adapter-token' })
    const packaged = createPackagedFetch(auth.runtime)
    const direct = await auth.runtime.resolve({ method: 'GET', url: '/api/auth/get-session' })
    server.use(...createAuthHandlers(auth))
    const [packagedResponse, mswResponse] = await Promise.all([
      packaged('https://app.local/api/auth/get-session'),
      fetch('https://app.local/api/auth/get-session'),
    ])
    expect(await json(packagedResponse)).toEqual(direct.body)
    expect(await json(mswResponse)).toEqual(direct.body)
  })

  it('drives the contract through the Better Auth Vue magic-link client', async () => {
    const auth = createAuthScenario({ allowedEmails: [allowedEmail], token: () => 'client-token' })
    const client = createInterviewAuthClient(createPackagedFetch(auth.runtime))
    const result = await client.signIn.magicLink({ email: allowedEmail, callbackURL: '#/auth/verify' })
    expect(result).toMatchObject({ data: { status: true }, error: null })
  })
})
