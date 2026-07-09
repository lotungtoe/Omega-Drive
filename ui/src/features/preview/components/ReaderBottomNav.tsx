import { ChevronLeft, ChevronRight } from 'lucide-react'
import { useTranslation } from 'react-i18next'

interface Props {
  currentChapter: number
  totalChapters: number
  onPrev: () => void
  onNext: () => void
  progressPercent: number
  autoHide: boolean
  visible: boolean
}

export function ReaderBottomNav({ currentChapter, totalChapters, onPrev, onNext, progressPercent, autoHide, visible }: Props) {
  const { t } = useTranslation()

  return (
    <div className={`border-t border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 transition-opacity duration-300 ${
      autoHide ? (visible ? 'opacity-100' : 'opacity-0 pointer-events-none') : 'opacity-100'
    }`}>
      <div className="h-0.5 bg-slate-100 dark:bg-slate-800">
        <div className="h-full bg-amber-500 transition-all duration-300" style={{ width: `${Math.min(100, progressPercent)}%` }} />
      </div>
      <div className="flex items-center justify-between px-8 py-4">
        <button type="button" disabled={currentChapter <= 0} onClick={onPrev}
          className="flex items-center gap-1 px-4 py-2 rounded-lg text-sm text-slate-600 dark:text-slate-400 hover:bg-slate-200 dark:hover:bg-slate-800 disabled:opacity-30 disabled:pointer-events-none transition-colors">
          <ChevronLeft className="w-4 h-4" />
          {t('book.previous', 'Previous')}
        </button>
        <span className="text-xs text-slate-400">{currentChapter + 1} / {totalChapters}</span>
        <button type="button" disabled={currentChapter >= totalChapters - 1} onClick={onNext}
          className="flex items-center gap-1 px-4 py-2 rounded-lg text-sm text-slate-600 dark:text-slate-400 hover:bg-slate-200 dark:hover:bg-slate-800 disabled:opacity-30 disabled:pointer-events-none transition-colors">
          {t('book.next', 'Next')}
          <ChevronRight className="w-4 h-4" />
        </button>
      </div>
    </div>
  )
}
