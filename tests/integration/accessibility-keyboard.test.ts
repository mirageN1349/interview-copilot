import { mount } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { defineComponent, nextTick, ref } from 'vue'
import { readFileSync } from 'node:fs'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import AppDialog from '@/shared/ui/dialog/Dialog.vue'
import DiagramEditor from '@/features/edit-diagram/ui/DiagramEditor.vue'
import { createDiagram } from '@/features/edit-diagram/model/diagram'
const materialCss = readFileSync('src/app/styles/material.css', 'utf8')
const tokenCss = readFileSync('src/app/styles/tokens.css', 'utf8')

describe('keyboard and accessibility regressions', () => {
  beforeEach(() => {
    Object.defineProperty(HTMLDialogElement.prototype, 'showModal', {
      configurable: true,
      value: vi.fn(function (this: HTMLDialogElement) { this.open = true }),
    })
    Object.defineProperty(HTMLDialogElement.prototype, 'close', {
      configurable: true,
      value: vi.fn(function (this: HTMLDialogElement) {
        this.open = false
        this.dispatchEvent(new Event('close'))
      }),
    })
  })
  afterEach(() => vi.restoreAllMocks())

  it('restores focus to the invoking control when a dialog closes', async () => {
    const host = defineComponent({
      components: { AppDialog },
      setup() { return { open: ref(false) } },
      template: '<button id="trigger" @click="open = true">Open</button><AppDialog v-model:open="open" title="Confirm"><button id="inside">Confirm</button></AppDialog>',
    })
    const wrapper = mount(host, { attachTo: document.body })
    const trigger = wrapper.get<HTMLButtonElement>('#trigger')
    trigger.element.focus()
    await trigger.trigger('click')
    await nextTick()
    expect(wrapper.get('dialog').attributes('aria-labelledby')).toBeTruthy()
    ;(wrapper.vm as unknown as { open: boolean }).open = false
    await nextTick()
    expect(document.activeElement).toBe(trigger.element)
    wrapper.unmount()
  })

  it('closes an open modal before its route is unmounted', async () => {
    const wrapper = mount(AppDialog, {
      props: { open: true, title: 'Confirm' },
      attachTo: document.body,
    })
    await nextTick()
    const element = wrapper.get<HTMLDialogElement>('dialog').element
    expect(element.open).toBe(true)

    wrapper.unmount()

    expect(element.open).toBe(false)
    expect(HTMLDialogElement.prototype.close).toHaveBeenCalledTimes(1)
  })

  it('exposes diagram relationships and keyboard commands without noisy live regions', async () => {
    const i18n = createI18n({ legacy: false, locale: 'en', messages: { en: { diagram: {
      title: 'Diagram', canvas: 'Canvas', relationships: '{label}: {relationships}', connectedTo: 'to {label}', connectedFrom: 'from {label}', noRelationships: 'none',
      newNode: 'New', undo: 'Undo', rename: 'Rename', proposal: 'Proposal', accept: 'Accept', reject: 'Reject',
      status: { connect: 'Connect', connected: 'Connected', accepted: 'Accepted', rejected: 'Rejected', undone: 'Undone', stale: 'Stale' },
    } } } })
    const wrapper = mount(DiagramEditor, {
      props: { initialDiagram: createDiagram({ revision: 0, nodes: [{ id: 'api', label: 'API', x: 0, y: 0 }], edges: [] }) },
      global: { plugins: [i18n] }, attachTo: document.body,
    })
    const node = wrapper.get('[data-node-id="api"]')
    expect(node.attributes('aria-describedby')).toBeTruthy()
    expect(node.attributes('aria-keyshortcuts')).toContain('Shift+ArrowRight')
    expect(wrapper.findAll('[aria-live]')).toHaveLength(1)
    await node.trigger('focus')
    await wrapper.get('[data-testid="diagram-editor"]').trigger('keydown', { key: 'ArrowRight', shiftKey: true })
    expect(wrapper.emitted('update:diagram')).toHaveLength(1)
    wrapper.unmount()
  })

  it('provides system accessibility fallbacks and scalable layout tokens', () => {
    expect(materialCss).toContain('@media (prefers-reduced-motion: reduce)')
    expect(materialCss).toContain('@media (prefers-reduced-transparency: reduce)')
    expect(tokenCss).toContain('@media (prefers-contrast: more)')
    expect(tokenCss).toContain('--motion-fast: 140ms')
    expect(`${materialCss}\n${tokenCss}`).not.toMatch(/font-size:\s*\d+px/)
  })
})
