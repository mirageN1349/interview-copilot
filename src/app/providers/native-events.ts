import { listen, type UnlistenFn } from '@tauri-apps/api/event'

const listeners = new Map<string, UnlistenFn>()

export async function listenOnce<T>(event: string, handler: (payload: T) => void): Promise<void> {
  if (!('__TAURI_INTERNALS__' in window) || listeners.has(event)) return
  listeners.set(
    event,
    await listen<T>(event, ({ payload }) => handler(payload)),
  )
}

export function disposeNativeEvents(): void {
  for (const unlisten of listeners.values()) unlisten()
  listeners.clear()
}
