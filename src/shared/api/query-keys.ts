export const queryKeys = {
  profiles: () => ['profiles'] as const,
  profile: (id: string) => ['profiles', id] as const,
  meetings: (filters: Readonly<Record<string, unknown>> = {}) => ['meetings', filters] as const,
  meeting: (id: string) => ['meetings', id] as const,
  models: (kind?: string) => ['models', kind ?? 'all'] as const,
  subscription: () => ['subscription'] as const,
}
