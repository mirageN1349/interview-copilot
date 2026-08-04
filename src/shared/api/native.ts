import { invoke as tauriInvoke } from '@tauri-apps/api/core'
import { parseCommandError } from './contracts/common'

type Invoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>

export function createNativeGateway(invoke: Invoke = tauriInvoke) {
  return {
    async invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
      if (!/^[a-z][a-z0-9_]{0,63}$/.test(command)) throw new TypeError('Invalid native command')
      try {
        return await invoke<T>(command, args)
      } catch (error) {
        throw parseCommandError(error)
      }
    },
  }
}

export const nativeGateway = createNativeGateway()
