import { useState, useCallback } from 'react'
import type { ReaderSettings } from '../utils/injectReaderStyles'
import type { ThemeName } from '../utils/themes'
import { cycleTheme as cycleThemeName } from '../utils/themes'

const LS_KEY = 'reader_settings'

function loadDefaults(): ReaderSettings {
  try {
    const raw = localStorage.getItem(LS_KEY)
    if (raw) return JSON.parse(raw) as ReaderSettings
  } catch {}
  return { font: 'Noto Serif', fontSize: 18, lineHeight: 1.7, theme: 'light' }
}

function persist(s: ReaderSettings) {
  localStorage.setItem(LS_KEY, JSON.stringify(s))
}

export function useBookSettings() {
  const [settings, setSettings] = useState<ReaderSettings>(loadDefaults)

  const save = useCallback((next: ReaderSettings) => {
    setSettings(next)
    persist(next)
  }, [])

  const setFont = useCallback((font: string) => save({ ...settings, font }), [settings, save])
  const setSize = useCallback((fontSize: number) => save({ ...settings, fontSize }), [settings, save])
  const setLineHeight = useCallback((lineHeight: number) => save({ ...settings, lineHeight }), [settings, save])
  const setTheme = useCallback((theme: ThemeName) => save({ ...settings, theme }), [settings, save])
  const cycleTheme = useCallback(() => save({ ...settings, theme: cycleThemeName(settings.theme) }), [settings, save])

  return { settings, setFont, setSize, setLineHeight, setTheme, cycleTheme }
}
