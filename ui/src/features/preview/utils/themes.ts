export type ThemeName = 'light' | 'sepia' | 'dark' | 'black'

export interface ThemeColors {
  background: string
  text: string
  link: string
  highlight: string
  selection: string
}

export const themes: Record<ThemeName, ThemeColors> = {
  light:    { background: '#faf7f2', text: '#1e1e1e', link: '#2563eb', highlight: '#fef08a', selection: '#bfdbfe' },
  sepia:    { background: '#f5e6c8', text: '#3b2f1e', link: '#8b5cf6', highlight: '#fde68a', selection: '#ddd6fe' },
  dark:     { background: '#1a1a1a', text: '#d4d4d4', link: '#60a5fa', highlight: '#78350f', selection: '#1e3a5f' },
  black:    { background: '#000000', text: '#aaaaaa', link: '#60a5fa', highlight: '#78350f', selection: '#1e3a5f' },
}

export function cycleTheme(current: ThemeName): ThemeName {
  const order: ThemeName[] = ['light', 'sepia', 'dark', 'black']
  return order[(order.indexOf(current) + 1) % order.length]
}
