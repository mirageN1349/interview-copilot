import { createAuthClient as createBetterAuthClient } from 'better-auth/vue'
import { magicLinkClient } from 'better-auth/client/plugins'
import { nativeGateway } from '@/shared/api/native'

export type AuthTransport = typeof fetch

let authTransport: AuthTransport = (...args) => globalThis.fetch(...args)

export function setAuthTransport(transport: AuthTransport): void {
  authTransport = transport
}

export function createInterviewAuthClient(transport: AuthTransport = globalThis.fetch) {
  return createBetterAuthClient({
    baseURL: 'https://app.local',
    basePath: '/api/auth',
    fetchOptions: { customFetchImpl: transport, redirect: 'manual' },
    plugins: [magicLinkClient()],
  })
}

export const authClient = createInterviewAuthClient((...args) => authTransport(...args))

export async function requestMagicLink(email: string) {
  const result = await authClient.signIn.magicLink({ email: email.trim().toLowerCase(), callbackURL: '#/auth/verify' })
  if (result.error) throw new Error(result.error.message ?? 'Could not send the sign-in link.')
  return result.data
}

export async function verifyMagicLink(token: string, transport: AuthTransport = (...args) => authTransport(...args)) {
  const query = new URLSearchParams({ token, callbackURL: '#/auth/verify' })
  const response = await transport(`https://app.local/api/auth/magic-link/verify?${query}`, { redirect: 'manual' })
  if (response.status === 302) return
  const body = await response.json() as { code?: string; message?: string }
  throw Object.assign(new Error(body.message ?? 'Could not verify this sign-in link.'), { code: body.code })
}

export async function syncNativeSession(): Promise<void> {
  if (!('__TAURI_INTERNALS__' in window)) return
  const session = await authClient.getSession()
  if (!session.data) throw new Error('Authenticated session is unavailable.')
  await nativeGateway.invoke('auth_session_sync', {
    input: {
      userId: session.data.user.id,
      email: session.data.user.email,
      name: session.data.user.name,
    },
  })
}

export async function signOut() {
  const result = await authClient.signOut()
  if (result.error) throw new Error(result.error.message ?? 'Could not sign out.')
  if ('__TAURI_INTERNALS__' in window) await nativeGateway.invoke('auth_session_clear')
  return result.data
}
