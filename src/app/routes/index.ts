import { createRouter, createWebHashHistory } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'

import { authClient } from '@/shared/api/auth/client'
import { syncNativeSession } from '@/shared/api/auth/client'
import type { PermissionSnapshot } from '@/pages/permissions/model/use-permissions'
import { nextMissingPermission } from '@/pages/permissions/model/use-permissions'
import { restrictedDiagnosticRoute } from './diagnostics'

const signIn = () => import('@/pages/sign-in/ui/SignInPage.vue')
const checkEmail = () => import('@/pages/sign-in/ui/CheckEmailPage.vue')
const verify = () => import('@/pages/sign-in/ui/AuthVerifyPage.vue')
const permissions = () => import('@/pages/permissions/ui/PermissionsPage.vue')
const profiles = () => import('@/pages/profiles/ui/ProfilesPage.vue')
const profile = () => import('@/pages/profiles/ui/ProfilePage.vue')
const newMeeting = () => import('@/pages/meeting/ui/NewMeetingPage.vue')
const meetingPage = () => import('@/pages/meeting/ui/MeetingPage.vue')
const overlay = () => import('@/features/overlay-chat/ui/OverlayShell.vue')
const history = () => import('@/pages/history/ui/HistoryPage.vue')
const historyDetail = () => import('@/pages/history/ui/MeetingDetailPage.vue')
const account = () => import('@/pages/account/ui/AccountPage.vue')

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', redirect: '/profiles' },
    { path: '/sign-in', component: signIn, meta: { title: 'Sign in' } },
    { path: '/sign-in/check-email', component: checkEmail, meta: { title: 'Check your email' } },
    { path: '/auth/verify', component: verify, meta: { title: 'Verify sign in' } },
    { path: '/permissions', component: permissions, meta: { title: 'Permissions' } },
    { path: '/profiles', component: profiles, meta: { title: 'Profiles' } },
    { path: '/profiles/:profileId', component: profile, meta: { title: 'Profile' } },
    { path: '/meetings/new', component: newMeeting, meta: { title: 'New meeting' } },
    { path: '/meetings/:meetingId', component: meetingPage, meta: { title: 'Meeting' } },
    { path: '/history', component: history, meta: { title: 'History' } },
    { path: '/history/:meetingId', component: historyDetail, meta: { title: 'Meeting history' } },
    { path: '/account', component: account, meta: { title: 'Account' } },
    { path: '/overlay/:meetingId', component: overlay, meta: { title: 'Overlay' } },
    restrictedDiagnosticRoute,
  ],
})

const publicRoutes = new Set(['/sign-in', '/sign-in/check-email', '/auth/verify'])

router.beforeEach(async (to) => {
  if (to.path.startsWith('/overlay/') && '__TAURI_INTERNALS__' in window) {
    const { getCurrentWindow } = await import('@tauri-apps/api/window')
    if (getCurrentWindow().label === 'overlay') return true
    return { path: '/profiles' }
  }
  const session = await authClient.getSession().catch(() => ({ data: null }))
  const hasSession = Boolean(session.data)

  if (!hasSession) return publicRoutes.has(to.path) ? true : { path: '/sign-in' }
  if ('__TAURI_INTERNALS__' in window) await syncNativeSession().catch(() => undefined)
  if (publicRoutes.has(to.path)) return { path: '/permissions' }
  if (to.path === '/permissions' || !('__TAURI_INTERNALS__' in window)) return true

  const snapshot = await invoke<PermissionSnapshot>('permissions_status').catch(() => null)
  if (!snapshot || nextMissingPermission(snapshot)) return { path: '/permissions' }
  return true
})
