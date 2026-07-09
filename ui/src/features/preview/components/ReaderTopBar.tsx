import { useEffect, useRef, useState } from 'react'
import { Menu, AArrowUp, Monitor, Maximize2, X } from 'lucide-react'
import type { ReaderSettings } from '../utils/injectReaderStyles'
import type { ThemeName } from '../utils/themes'
import { FontSettingsPanel } from './FontSettingsPanel'

interface Props {
  title: string
  chapterTitle?: string
  onToggleSidebar: () => void
  onClose: () => void
  settings: ReaderSettings
  onFontChange: (font: string) => void
  onSizeChange: (size: number) => void
  onLineHeightChange: (lh: number) => void
  onThemeChange: (theme: ThemeName) => void
  onCycleTheme: () => void
  onToggleFullscreen: () => void
  autoHide: boolean
}

export function ReaderTopBar({
  title, chapterTitle, onToggleSidebar, onClose,
  settings, onFontChange, onSizeChange, onLineHeightChange, onThemeChange,
  onCycleTheme, onToggleFullscreen, autoHide,
}: Props) {
  const [showFontPanel, setShowFontPanel] = useState(false)
  const [visible, setVisible] = useState(true)
  const hideTimer = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    if (!autoHide) { setVisible(true); return }
    const reset = () => {
      setVisible(true)
      clearTimeout(hideTimer.current)
      hideTimer.current = setTimeout(() => setVisible(false), 3000)
    }
    window.addEventListener('mousemove', reset)
    return () => {
      window.removeEventListener('mousemove', reset)
      clearTimeout(hideTimer.current)
    }
  }, [autoHide])

  return (
    <header className={`h-12 flex items-center justify-between px-3 border-b border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 shrink-0 z-10 transition-opacity duration-300 ${
      visible ? 'opacity-100' : 'opacity-0 pointer-events-none'
    }`}>
      <button type="button" onClick={onToggleSidebar} className="p-2 rounded-lg hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-600 dark:text-slate-400">
        <Menu className="w-5 h-5" />
      </button>
      <div className="flex-1 text-center min-w-0 mx-2">
        <span className="text-sm font-medium text-slate-800 dark:text-slate-200 truncate block">
          {chapterTitle || title}
        </span>
      </div>
      <div className="flex items-center gap-1 relative">
        <button type="button" onClick={() => setShowFontPanel(v => !v)} className="p-2 rounded-lg hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-600 dark:text-slate-400">
          <AArrowUp className="w-5 h-5" />
        </button>
        <button type="button" onClick={onCycleTheme} className="p-2 rounded-lg hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-600 dark:text-slate-400">
          <Monitor className="w-5 h-5" />
        </button>
        <button type="button" onClick={onToggleFullscreen} className="p-2 rounded-lg hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-600 dark:text-slate-400">
          <Maximize2 className="w-5 h-5" />
        </button>
        <button type="button" onClick={onClose} className="p-2 rounded-lg hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-600 dark:text-slate-400">
          <X className="w-5 h-5" />
        </button>
        {showFontPanel && (
          <FontSettingsPanel
            settings={settings}
            onFontChange={onFontChange}
            onSizeChange={onSizeChange}
            onLineHeightChange={onLineHeightChange}
            onThemeChange={onThemeChange}
          />
        )}
      </div>
    </header>
  )
}
