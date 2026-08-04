import { flushPromises } from '@vue/test-utils'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { createHistoryGateway, normalizeHistoryFilters } from '@/entities/meeting'
import { createAppearanceController } from '@/app/providers/appearance'
import { i18n } from '@/app/providers/i18n'
import HistoryFilters from '@/pages/history/ui/HistoryFilters.vue'
import { setAuthTransport, signOut } from '@/shared/api/auth/client'
import DatePicker from '@/shared/ui/date-picker/DatePicker.vue'
import { render } from '../helpers/render'

describe('history and account', () => {
  afterEach(() => setAuthTransport(globalThis.fetch))

  it('normalizes bounded history filters and sends only the native search contract', async () => {
    const invoke = vi.fn(async <T>() => ({ items: [], nextCursor: null }) as T)
    const gateway = createHistoryGateway({ invoke })
    const filters = normalizeHistoryFilters({ query: '  Vue   interview  ', profileQuery: 'Frontend', fromMs: 20, toMs: 10 })

    expect(filters).toEqual({ query: 'Vue interview', field: 'any', profileQuery: 'Frontend', fromMs: 10, toMs: 20, limit: 30 })
    await gateway.search(filters)
    expect(invoke).toHaveBeenCalledWith('meeting_search', { input: filters })
  })

  it('signs out through the auth transport', async () => {
    const fetcher = vi.fn(async () => Response.json({ data: { success: true } }))
    setAuthTransport(fetcher as typeof fetch)

    await signOut()

    expect(String(fetcher.mock.calls[0]?.[0])).toContain('/api/auth/sign-out')
    expect(fetcher.mock.calls[0]?.[1]).toMatchObject({ method: 'POST' })
  })

  it('persists light/dark/auto and follows the OS only in auto mode', () => {
    const dark = { matches: true, addEventListener: vi.fn(), removeEventListener: vi.fn() }
    const reduceMotion = { matches: false, addEventListener: vi.fn(), removeEventListener: vi.fn() }
    const reduceTransparency = { matches: true, addEventListener: vi.fn(), removeEventListener: vi.fn() }
    const contrast = { matches: false, addEventListener: vi.fn(), removeEventListener: vi.fn() }
    const matchMedia = vi.fn((query: string) => ({
      '(prefers-color-scheme: dark)': dark,
      '(prefers-reduced-motion: reduce)': reduceMotion,
      '(prefers-reduced-transparency: reduce)': reduceTransparency,
      '(prefers-contrast: more)': contrast,
    })[query]!)
    const storage = new Map<string, string>()
    const controller = createAppearanceController({
      root: document.documentElement,
      matchMedia: matchMedia as unknown as typeof window.matchMedia,
      storage: { getItem: (key) => storage.get(key) ?? null, setItem: (key, value) => storage.set(key, value) },
      refreshMaterial: vi.fn(),
    })

    controller.setTheme('light')
    expect(document.documentElement.dataset.theme).toBe('light')
    expect(storage.get('appearance.theme')).toBe('light')
    controller.setTheme('auto')
    expect(document.documentElement.dataset.theme).toBe('dark')
    expect(document.documentElement.dataset.reduceTransparency).toBe('true')
    controller.dispose()
  })

  it('keeps the search icon clear of text and exposes keyboard date dialogs', async () => {
    const filters = render(HistoryFilters, {
      props: { modelValue: { field: 'any', limit: 30 } },
      global: { plugins: [i18n] },
    })
    const search = filters.get('input[type="search"]')
    expect(search.attributes('style')).toContain('padding-left: 2.5rem')
    expect(filters.findAll('[aria-haspopup="dialog"]')).toHaveLength(2)
    expect(filters.get('legend').text()).toBeTruthy()

    const picker = render(DatePicker, {
      props: { placeholder: 'Choose date', ariaLabel: 'Start date' },
      global: { plugins: [i18n] },
    })
    await picker.get('[aria-haspopup="dialog"]').trigger('click')
    await flushPromises()
    const dialog = document.body.querySelector<HTMLElement>('[data-slot="popover-content"]')
    expect(dialog?.getAttribute('aria-label')).toBe('Start date')
    expect(dialog?.querySelector('select')).toBeNull()
    expect(dialog?.querySelector('[data-slot="calendar-heading"]')?.textContent?.trim()).toBeTruthy()
    dialog?.querySelector<HTMLButtonElement>('[data-slot="calendar-cell-trigger"]:not([data-disabled])')?.click()
    await flushPromises()
    expect(picker.emitted('update:modelValue')?.[0]?.[0]).toEqual(expect.any(Number))
    await picker.get('[aria-haspopup="dialog"]').trigger('click')
    await flushPromises()
    document.body.querySelector<HTMLElement>('[data-slot="popover-content"]')?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
    await flushPromises()
    expect(document.body.querySelector('[data-slot="popover-content"]')).toBeNull()
    picker.unmount()
  })

  it('removes an open calendar portal when navigation unmounts the page', async () => {
    const picker = render(DatePicker, {
      props: { placeholder: 'Choose date', ariaLabel: 'Start date' },
      global: { plugins: [i18n] },
    })
    await picker.get('[aria-haspopup="dialog"]').trigger('click')
    await flushPromises()
    expect(document.body.querySelector('[data-slot="popover-content"]')).not.toBeNull()

    picker.unmount()
    await flushPromises()
    expect(document.body.querySelector('[data-slot="popover-content"]')).toBeNull()
  })
})
