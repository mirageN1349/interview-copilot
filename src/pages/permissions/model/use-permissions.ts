import { invoke } from '@tauri-apps/api/core'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

export type PermissionKind = 'screen' | 'microphone' | 'accessibility'
export type PermissionState = 'not_determined' | 'granted' | 'denied' | 'restricted'

export type PermissionSnapshot = {
  screenRecording: PermissionState
  microphone: PermissionState
  accessibility: PermissionState
  observedAt: string
  restartMayBeRequired: boolean
}

type PermissionApi = {
  status(): Promise<PermissionSnapshot>
  request(kind: PermissionKind): Promise<PermissionSnapshot>
  openSettings(kind: PermissionKind): Promise<void>
}

const nativeApi: PermissionApi = {
  status: () => invoke('permissions_status'),
  request: (kind) => invoke('permissions_request', { kind }),
  openSettings: (kind) => invoke('permissions_open_settings', { kind }),
}

const order: PermissionKind[] = ['screen', 'microphone', 'accessibility']

export function nextMissingPermission(snapshot: PermissionSnapshot): PermissionKind | null {
  return order.find((kind) => stateFor(snapshot, kind) !== 'granted') ?? null
}

export function stateFor(snapshot: PermissionSnapshot, kind: PermissionKind): PermissionState {
  if (kind === 'screen') return snapshot.screenRecording
  return snapshot[kind]
}

export function usePermissions(api: PermissionApi = nativeApi) {
  const snapshot = ref<PermissionSnapshot | null>(null)
  const phase = ref<'checking' | 'incomplete' | 'requesting' | 'complete' | 'error'>('checking')
  const error = ref('')
  const nextMissing = computed(() => (snapshot.value ? nextMissingPermission(snapshot.value) : null))
  let refreshTimer: ReturnType<typeof setInterval> | undefined

  async function refresh(silent = false): Promise<void> {
    if (!silent) {
      phase.value = 'checking'
      error.value = ''
    }
    try {
      snapshot.value = await api.status()
      phase.value = nextMissingPermission(snapshot.value) ? 'incomplete' : 'complete'
    } catch {
      if (silent) return
      phase.value = 'error'
      error.value = 'permissions.statusError'
    }
  }

  async function request(kind: PermissionKind): Promise<void> {
    phase.value = 'requesting'
    error.value = ''
    try {
      snapshot.value = await api.request(kind)
      if (kind !== 'microphone' && stateFor(snapshot.value, kind) !== 'granted') {
        await api.openSettings(kind)
      }
      await refresh()
    } catch {
      phase.value = 'error'
      error.value = 'permissions.requestError'
    }
  }

  async function openSettings(kind: PermissionKind): Promise<void> {
    error.value = ''
    try {
      await api.openSettings(kind)
    } catch {
      error.value = 'permissions.requestError'
    }
  }

  function onActivation(): void {
    if (document.visibilityState === 'visible') void refresh()
  }

  onMounted(() => {
    void refresh()
    refreshTimer = window.setInterval(() => {
      if (phase.value !== 'requesting') void refresh(true)
    }, 1_000)
    window.addEventListener('focus', onActivation)
    document.addEventListener('visibilitychange', onActivation)
  })
  onBeforeUnmount(() => {
    if (refreshTimer) window.clearInterval(refreshTimer)
    window.removeEventListener('focus', onActivation)
    document.removeEventListener('visibilitychange', onActivation)
  })

  return { snapshot, phase, error, nextMissing, refresh, request, openSettings }
}
