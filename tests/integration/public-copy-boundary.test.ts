import { describe, expect, it } from 'vitest'
import { createI18n } from 'vue-i18n'

import { en } from '@/shared/config/i18n/en'
import { ru } from '@/shared/config/i18n/ru'
import { assertPublicCopy, publicMeetingUnavailable } from '@/shared/lib/public-copy'
import { restrictedDiagnosticRoute } from '@/app/routes/diagnostics'
import { createPolicyController } from '@/shared/api/ws/policy'

function strings(value: unknown): string[] {
  if (typeof value === 'string') return [value]
  if (!value || typeof value !== 'object') return []
  return Object.values(value).flatMap(strings)
}

describe('public copy boundary', () => {
  it('compiles literal email addresses in both locales', () => {
    const messages = { en, ru }
    for (const locale of ['en', 'ru'] as const) {
      const translator = createI18n({ legacy: false, locale, messages })
      expect(translator.global.t('auth.signIn.emailPlaceholder')).toBe('you@example.com')
    }
  })

  it('keeps ordinary localized copy free of restricted operational terms', () => {
    for (const copy of [...strings(en), ...strings(ru)]) {
      expect(() => assertPublicCopy(copy)).not.toThrow()
    }
  })

  it('maps detailed denial codes to one neutral public state', () => {
    expect(publicMeetingUnavailable('ADVERSARIAL_MATRIX_UNSUPPORTED')).toEqual({
      titleKey: 'meeting.unavailable.title',
      detailKey: 'meeting.unavailable.detail',
      actionKey: 'meeting.unavailable.action',
    })
    expect(() => assertPublicCopy('Security policy and audit matrix failed')).toThrow()
  })

  it('keeps diagnostics outside ordinary navigation', () => {
    expect(restrictedDiagnosticRoute.path).toBe('/_diagnostics')
    expect(restrictedDiagnosticRoute.meta).toMatchObject({ restricted: true, navigation: false })
  })

  it('cancels and stops locally once when policy is lost', async () => {
    const calls: string[] = []
    const controller = createPolicyController({
      now: () => 100,
      cancelPending: () => calls.push('cancel'),
      stopAll: () => calls.push('stop'),
      emitAudit: () => calls.push('audit'),
    })

    await controller.transportLost()
    await controller.transportLost()
    expect(calls).toEqual(['cancel', 'stop', 'audit'])
    expect(controller.blocked()).toBe(true)
  })

  it('emits the local audit decision even when stop finalization fails', async () => {
    const calls: string[] = []
    const controller = createPolicyController({
      cancelPending: () => calls.push('cancel'),
      stopAll: async () => {
        calls.push('stop')
        throw new Error('finalization failed')
      },
      emitAudit: () => calls.push('audit'),
    })

    await expect(controller.transportLost()).rejects.toThrow('finalization failed')
    expect(calls).toEqual(['cancel', 'stop', 'audit'])
    await expect(controller.transportLost()).resolves.toBe(false)
  })
})
