export const APPEARANCE_STORAGE_KEY = 'appearance.theme'
export type AppearanceTheme = 'light' | 'dark' | 'auto'
export type ResolvedTheme = Exclude<AppearanceTheme, 'auto'>

export function parseAppearanceTheme(value: string | null): AppearanceTheme {
  return value === 'light' || value === 'dark' || value === 'auto' ? value : 'auto'
}
