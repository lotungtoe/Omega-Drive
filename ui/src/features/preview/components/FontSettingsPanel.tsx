import type { ReaderSettings } from '../utils/injectReaderStyles'
import type { ThemeName } from '../utils/themes'
import { themes } from '../utils/themes'

interface Props {
  settings: ReaderSettings
  onFontChange: (font: string) => void
  onSizeChange: (size: number) => void
  onLineHeightChange: (lh: number) => void
  onThemeChange: (theme: ThemeName) => void
}

const FONTS = ['Noto Serif', 'Literata', 'Georgia', 'Inter', 'OpenDyslexic']
const THEME_NAMES: { name: ThemeName; label: string }[] = [
  { name: 'light', label: 'Sáng' },
  { name: 'sepia', label: 'Sepia' },
  { name: 'dark', label: 'Tối' },
  { name: 'black', label: 'Đen' },
]

export function FontSettingsPanel({ settings, onFontChange, onSizeChange, onLineHeightChange, onThemeChange }: Props) {
  return (
    <div className="absolute top-full right-0 mt-1 w-64 bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-700 rounded-xl shadow-2xl p-4 z-50">
      <div className="space-y-4">
        <div>
          <label className="text-xs font-semibold text-slate-500 dark:text-slate-400 uppercase tracking-wider">Font</label>
          <select value={settings.font} onChange={e => onFontChange(e.target.value)}
            className="mt-1 w-full px-3 py-2 rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 text-sm">
            {FONTS.map(f => <option key={f} value={f}>{f}</option>)}
          </select>
        </div>
        <div>
          <label className="text-xs font-semibold text-slate-500 dark:text-slate-400 uppercase tracking-wider">Cỡ chữ</label>
          <div className="mt-1 flex items-center gap-3">
            <button type="button" onClick={() => onSizeChange(Math.max(12, settings.fontSize - 1))}
              disabled={settings.fontSize <= 12}
              className="w-8 h-8 rounded-lg border border-slate-200 dark:border-slate-700 text-sm disabled:opacity-40">−</button>
            <span className="text-sm font-medium w-8 text-center">{settings.fontSize}</span>
            <button type="button" onClick={() => onSizeChange(Math.min(32, settings.fontSize + 1))}
              disabled={settings.fontSize >= 32}
              className="w-8 h-8 rounded-lg border border-slate-200 dark:border-slate-700 text-sm disabled:opacity-40">+</button>
          </div>
        </div>
        <div>
          <label className="text-xs font-semibold text-slate-500 dark:text-slate-400 uppercase tracking-wider">Dãn dòng</label>
          <div className="mt-1 flex items-center gap-3">
            <button type="button" onClick={() => onLineHeightChange(Math.round((settings.lineHeight - 0.1) * 10) / 10)}
              disabled={settings.lineHeight <= 1.0}
              className="w-8 h-8 rounded-lg border border-slate-200 dark:border-slate-700 text-sm disabled:opacity-40">−</button>
            <span className="text-sm font-medium w-10 text-center">{settings.lineHeight.toFixed(1)}</span>
            <button type="button" onClick={() => onLineHeightChange(Math.round((settings.lineHeight + 0.1) * 10) / 10)}
              disabled={settings.lineHeight >= 3.0}
              className="w-8 h-8 rounded-lg border border-slate-200 dark:border-slate-700 text-sm disabled:opacity-40">+</button>
          </div>
        </div>
        <div>
          <label className="text-xs font-semibold text-slate-500 dark:text-slate-400 uppercase tracking-wider">Nền</label>
          <div className="mt-1 flex gap-2">
            {THEME_NAMES.map(t => (
              <button key={t.name} type="button" onClick={() => onThemeChange(t.name)}
                className={`flex-1 h-10 rounded-lg border-2 transition-all flex items-center justify-center text-xs font-medium ${
                  settings.theme === t.name ? 'border-amber-500 ring-1 ring-amber-500' : 'border-slate-200 dark:border-slate-700'
                }`}
                style={{ background: themes[t.name].background, color: themes[t.name].text }}>
                {t.label}
              </button>
            ))}
          </div>
        </div>
      </div>
    </div>
  )
}
