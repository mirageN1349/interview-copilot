import { queryOptions, type QueryClient } from '@tanstack/vue-query'

import { nativeGateway } from '@/shared/api/native'
import { queryKeys } from '@/shared/api/query-keys'
import type {
  ProfileDetails,
  ProfileSaveInput,
  ProfileSourceImportInput,
  ProfileSummary,
} from '../model/types'

type NativeInvoker = {
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>
}

export function createProfileGateway(invoker: NativeInvoker = nativeGateway) {
  return {
    list: () => invoker.invoke<ProfileSummary[]>('profile_list'),
    get: (profileId: string) => invoker.invoke<ProfileDetails>('profile_get', { profileId }),
    save: (input: ProfileSaveInput) => invoker.invoke<ProfileDetails>('profile_save', { input }),
    archive: (profileId: string, expectedRevision: number) => invoker.invoke<ProfileDetails>('profile_archive', { input: { profileId, expectedRevision } }),
    restore: (profileId: string, expectedRevision: number) => invoker.invoke<ProfileDetails>('profile_restore', { input: { profileId, expectedRevision } }),
    importSource: (input: ProfileSourceImportInput) => invoker.invoke<ProfileDetails>('profile_source_import', { input }),
  }
}

export const profileGateway = createProfileGateway()

export const profilesQuery = () => queryOptions({
  queryKey: queryKeys.profiles(),
  queryFn: profileGateway.list,
})

export const profileQuery = (profileId: string) => queryOptions({
  queryKey: queryKeys.profile(profileId),
  queryFn: () => profileGateway.get(profileId),
  enabled: Boolean(profileId),
})

export async function invalidateProfileQueries(queryClient: QueryClient, profileId?: string) {
  await queryClient.invalidateQueries({ queryKey: queryKeys.profiles() })
  if (profileId) await queryClient.invalidateQueries({ queryKey: queryKeys.profile(profileId) })
}
