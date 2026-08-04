import { flushPromises, mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { describe, expect, it } from 'vitest'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

import SideChat from '@/features/overlay-chat/ui/SideChat.vue'
import { en } from '@/shared/config/i18n/en'

describe('overlay composer', () => {
  it('uses one composer focus glow instead of outlines on nested controls', () => {
    const source = readFileSync(resolve('src/features/overlay-chat/ui/SideChat.vue'), 'utf8')
    expect(source).toContain('.composer:focus-within')
    expect(source).toContain('.composer :deep(:is(textarea, button):focus-visible)')
    expect(source).toContain('0 0 0 3px color-mix(in oklch, var(--accent) 13%, transparent)')
    expect(source).toContain('outline: none')
    expect(source).not.toContain('border-color: color-mix(in oklch, var(--accent) 38%, transparent)')
  })

  it('submits the selected answer depth from Enter and exposes the screen-context action', async () => {
    const wrapper = mount(SideChat, {
      props: { messages: [], timeline: [] },
      global: { plugins: [createI18n({ legacy: false, locale: 'en', messages: { en } })] },
      attachTo: document.body,
    })

    await wrapper.get(`button[aria-label="${en.profiles.models.depth}"]`).trigger('click')
    await flushPromises()
    const listbox = document.body.querySelector<HTMLElement>('[role="listbox"]')!
    const options = [...listbox.querySelectorAll<HTMLButtonElement>('[role="option"]')]
    expect(options).toHaveLength(3)
    expect(options[1]?.getAttribute('aria-selected')).toBe('true')
    options[1]?.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }))
    expect(document.activeElement).toBe(options[2])
    options[2]?.click()
    await flushPromises()

    await wrapper.get('textarea').setValue('Explain this code')
    await wrapper.get('textarea').trigger('keydown', { key: 'Enter' })
    await wrapper.get(`[aria-label="${en.overlay.side.attach}"]`).trigger('click')

    expect(wrapper.emitted('send')).toEqual([[{ content: 'Explain this code', depth: 'detailed' }]])
    expect(wrapper.emitted('attach')).toHaveLength(1)
    wrapper.unmount()
  })

  it('closes the answer-depth popover with Escape and outside press', async () => {
    const wrapper = mount(SideChat, {
      props: { messages: [], timeline: [] },
      global: { plugins: [createI18n({ legacy: false, locale: 'en', messages: { en } })] },
      attachTo: document.body,
    })
    const trigger = wrapper.get(`button[aria-label="${en.profiles.models.depth}"]`)

    await trigger.trigger('click')
    await flushPromises()
    document.body.querySelector<HTMLElement>('[data-slot="popover-content"]')
      ?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
    await flushPromises()
    expect(document.body.querySelector('[role="listbox"]')).toBeNull()

    await trigger.trigger('click')
    await flushPromises()
    document.body.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }))
    await flushPromises()
    expect(document.body.querySelector('[role="listbox"]')).toBeNull()
    wrapper.unmount()
  })
})
