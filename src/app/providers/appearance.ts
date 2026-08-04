import { reactive, readonly } from 'vue'

import { APPEARANCE_STORAGE_KEY, parseAppearanceTheme, type AppearanceTheme } from '@/shared/config/appearance'
import { nativeGateway } from '@/shared/api/native'

type MediaQuery = Pick<MediaQueryList, 'matches' | 'addEventListener' | 'removeEventListener'>
type AppearanceOptions = {
  root?: HTMLElement
  matchMedia?: (query: string) => MediaQuery
  storage?: Pick<Storage, 'getItem' | 'setItem'>
  refreshMaterial?: (state: AppearanceState) => void | Promise<void>
}
type AppearanceState = {
  theme: AppearanceTheme
  resolvedTheme: 'light' | 'dark'
  reduceMotion: boolean
  reduceTransparency: boolean
  increasedContrast: boolean
}

const state = reactive<AppearanceState>({ theme: 'auto', resolvedTheme: 'light', reduceMotion: false, reduceTransparency: false, increasedContrast: false })
export const appearanceState = readonly(state)

export function createAppearanceController(options: AppearanceOptions = {}) {
  const root = options.root ?? document.documentElement
  const media = options.matchMedia ?? window.matchMedia.bind(window)
  const storage = options.storage ?? localStorage
  const refreshMaterial = options.refreshMaterial ?? ((snapshot) => {
    if (!('__TAURI_INTERNALS__' in window)) return
    return nativeGateway.invoke('overlay_apply_material', { appearance: snapshot }).catch(() => undefined)
  })
  const queries = {
    dark: media('(prefers-color-scheme: dark)'),
    reduceMotion: media('(prefers-reduced-motion: reduce)'),
    reduceTransparency: media('(prefers-reduced-transparency: reduce)'),
    increasedContrast: media('(prefers-contrast: more)'),
  }

  function apply() {
    state.resolvedTheme = state.theme === 'auto' ? (queries.dark.matches ? 'dark' : 'light') : state.theme
    state.reduceMotion = queries.reduceMotion.matches
    state.reduceTransparency = queries.reduceTransparency.matches
    state.increasedContrast = queries.increasedContrast.matches
    root.dataset.theme = state.resolvedTheme
    root.dataset.reduceMotion = String(state.reduceMotion)
    root.dataset.reduceTransparency = String(state.reduceTransparency)
    root.dataset.increasedContrast = String(state.increasedContrast)
    void refreshMaterial({ ...state })
  }
  function setTheme(theme: AppearanceTheme) {
    state.theme = theme
    storage.setItem(APPEARANCE_STORAGE_KEY, theme)
    apply()
    if ('__TAURI_INTERNALS__' in window) {
      void nativeGateway.invoke('appearance_save', {
        input: { theme, reduceTransparency: state.reduceTransparency },
      }).catch(() => undefined)
    }
  }
  const onChange = () => apply()
  Object.values(queries).forEach((query) => query.addEventListener('change', onChange))
  state.theme = parseAppearanceTheme(storage.getItem(APPEARANCE_STORAGE_KEY))
  apply()

  return {
    state: appearanceState,
    setTheme,
    dispose: () => Object.values(queries).forEach((query) => query.removeEventListener('change', onChange)),
  }
}

let applicationController: ReturnType<typeof createAppearanceController> | undefined
export function initializeAppearanceProvider() {
  return applicationController ??= createAppearanceController()
}
