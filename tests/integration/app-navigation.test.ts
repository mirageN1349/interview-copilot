import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { createMemoryHistory, createRouter } from 'vue-router'
import { describe, expect, it } from 'vitest'

import App from '@/App.vue'
import { en } from '@/shared/config/i18n/en'

describe('app navigation', () => {
  it('renders fixed branded navigation and offsets routed content', async () => {
    const router = createRouter({
      history: createMemoryHistory(),
      routes: [
        { path: '/profiles', component: { template: '<main>Profiles</main>' } },
        { path: '/meetings/new', component: { template: '<main>Meeting</main>' } },
      ],
    })
    await router.push('/meetings/new')
    await router.isReady()

    const wrapper = mount(App, {
      global: { plugins: [router, createI18n({ legacy: false, locale: 'en', messages: { en } })] },
    })

    expect(wrapper.get('nav').classes()).toContain('fixed')
    expect(wrapper.get('.app-brand').text()).toBe('Interview Copilot')
    expect(wrapper.get('.app-route-frame--nav').exists()).toBe(true)
    expect(wrapper.get('a[href="/meetings/new"]').classes()).toContain('app-navigation-link--active')

    await router.push('/profiles')
    await wrapper.vm.$nextTick()
    expect(wrapper.get('main').text()).toBe('Profiles')
    expect(wrapper.findAll('main')).toHaveLength(1)
    expect(wrapper.get('main').classes()).toContain('route-page-content')
    wrapper.unmount()
  })
})
