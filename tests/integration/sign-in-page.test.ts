import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createMemoryHistory, createRouter } from 'vue-router'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import SignInPage from '@/pages/sign-in/ui/SignInPage.vue'
import { en } from '@/shared/config/i18n/en'

const requestMagicLink = vi.hoisted(() => vi.fn().mockResolvedValue(undefined))
vi.mock('@/shared/api/auth/client', () => ({ requestMagicLink }))

describe('sign-in page', () => {
  beforeEach(() => requestMagicLink.mockClear())

  it('navigates onboarding with arrows and progress, preserves email, then signs in', async () => {
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [
        { path: '/sign-in', component: SignInPage },
        { path: '/sign-in/check-email', component: { template: '<main>Check email</main>' } },
      ],
    })
    await router.push('/sign-in')
    await router.isReady()

    const wrapper = mount(SignInPage, {
      global: { plugins: [router, createI18n({ legacy: false, locale: 'en', messages: { en } })] },
    })

    expect(wrapper.get('h1').text()).toBe(en.auth.signIn.onboarding.steps.context.title)
    expect(wrapper.findAll('.onboarding-dot')).toHaveLength(4)
    expect(wrapper.find('.onboarding-dot--active').attributes('aria-label')).toBe('Go to step 1')

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight' }))
    await flushPromises()
    expect(wrapper.get('h1').text()).toBe(en.auth.signIn.onboarding.steps.live.title)

    await wrapper.findAll('.onboarding-dot')[3]!.trigger('click')
    await wrapper.get('input[type="email"]').setValue('USER@example.test')
    await wrapper.findAll('.onboarding-dot')[0]!.trigger('click')
    await wrapper.findAll('.onboarding-dot')[3]!.trigger('click')
    expect((wrapper.get('input[type="email"]').element as HTMLInputElement).value).toBe('USER@example.test')

    wrapper.get('input').element.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft', bubbles: true }))
    await flushPromises()
    expect(wrapper.get('h1').text()).toBe(en.auth.signIn.title)

    await wrapper.get('form').trigger('submit')
    await flushPromises()

    expect(requestMagicLink).toHaveBeenCalledWith('USER@example.test')
    expect(router.currentRoute.value.fullPath).toBe('/sign-in/check-email?email=user@example.test')
    wrapper.unmount()
  })
})
