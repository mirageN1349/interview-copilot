import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createMemoryHistory, createRouter } from 'vue-router'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { routeForAccess } from '@/app/routes/guards'
import PermissionsPage from '@/pages/permissions/ui/PermissionsPage.vue'
import {
  nextMissingPermission,
  type PermissionSnapshot,
} from '@/pages/permissions/model/use-permissions'

const granted: PermissionSnapshot = {
  screenRecording: 'granted',
  microphone: 'granted',
  accessibility: 'granted',
  observedAt: '2026-08-04T00:00:00.000Z',
  restartMayBeRequired: false,
}

const invokeMock = vi.hoisted(() => vi.fn())
vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }))

describe('permission restart routing', () => {
  beforeEach(() => invokeMock.mockReset().mockResolvedValue(granted))

  it('renders and refreshes permission state while open', async () => {
    vi.useFakeTimers()
    invokeMock
      .mockResolvedValueOnce({ ...granted, screenRecording: 'not_determined', restartMayBeRequired: true })
      .mockResolvedValueOnce({ ...granted, screenRecording: 'not_determined', restartMayBeRequired: true })
      .mockResolvedValueOnce(undefined)
      .mockResolvedValue(granted)
    const router = createRouter({ history: createMemoryHistory(), routes: [{ path: '/permissions', component: PermissionsPage }] })
    const i18n = createI18n({ legacy: false, locale: 'en', messages: { en: { common: { allow: 'Allow', allowed: 'Allowed', continue: 'Continue', settings: 'Settings' }, permissions: { eyebrow: 'Setup', title: 'Allow access', intro: 'Intro', screen: { title: 'Screen', description: 'Screen help' }, microphone: { title: 'Microphone', description: 'Microphone help' }, accessibility: { title: 'Accessibility', description: 'Accessibility help' } } } } })
    await router.push('/permissions')
    await router.isReady()

    const wrapper = mount(PermissionsPage, { global: { plugins: [router, i18n] } })
    await flushPromises()

    expect(wrapper.text()).toContain('Allow access')
    await wrapper.findAll('button').find((button) => button.text() === 'Allow')!.trigger('click')
    await flushPromises()
    expect(invokeMock).toHaveBeenCalledWith('permissions_open_settings', { kind: 'screen' })
    expect(wrapper.text()).toContain('Allowed')
    expect(wrapper.find('button[aria-label="Settings: Screen"]').exists()).toBe(false)
    wrapper.unmount()
    vi.useRealTimers()
  })

  it('selects the first permission missing from the live snapshot', () => {
    expect(nextMissingPermission({ ...granted, screenRecording: 'denied' })).toBe('screen')
    expect(nextMissingPermission({ ...granted, microphone: 'not_determined' })).toBe('microphone')
    expect(nextMissingPermission({ ...granted, accessibility: 'restricted' })).toBe('accessibility')
    expect(nextMissingPermission(granted)).toBeNull()
  })

  it('routes in session, live-permission, readiness order', () => {
    expect(routeForAccess({ hasSession: false, permissionsComplete: false, meetingAvailable: false })).toEqual({ path: '/sign-in' })
    expect(routeForAccess({ hasSession: true, permissionsComplete: false, meetingAvailable: false })).toEqual({ path: '/permissions' })
    expect(routeForAccess({ hasSession: true, permissionsComplete: true, meetingAvailable: false })).toEqual({ path: '/meetings/new', query: { state: 'unavailable' } })
    expect(routeForAccess({ hasSession: true, permissionsComplete: true, meetingAvailable: true })).toBeNull()
  })
})
