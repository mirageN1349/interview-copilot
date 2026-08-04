import type { RouteLocationRaw } from 'vue-router'

export type RouteAccess = {
  hasSession: boolean
  permissionsComplete: boolean
  meetingAvailable: boolean
}

export function routeForAccess(access: RouteAccess): RouteLocationRaw | null {
  if (!access.hasSession) return { path: '/sign-in' }
  if (!access.permissionsComplete) return { path: '/permissions' }
  if (!access.meetingAvailable) return { path: '/meetings/new', query: { state: 'unavailable' } }
  return null
}
