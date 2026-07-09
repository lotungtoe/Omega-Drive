import { useCallback, useEffect, useState, useRef } from 'react'
import { Loader2, AlertCircle } from 'lucide-react'
import { ReaderSidebar } from './ReaderSidebar'
import { useTranslation } from 'react-i18next'
import { useBookReader } from '../hooks/useBookReader'
import { useBookSettings } from '../hooks/useBookSettings'
import { useBookProgress } from '../hooks/useBookProgress'
import { ReaderTopBar } from './ReaderTopBar'
import { ReaderContent } from './ReaderContent'
import { ReaderBottomNav } from './ReaderBottomNav'

export function BookReader({ file, onClose, onDownload }) {
  const { t } = useTranslation()
  const displayName = file.filename || file.name || ''

  const {
    loading, error, spine, nav, currentChapter, chapterHtml, chapterLoading, spineTitle,
    setSpineTitle, loadChapter, handleScroll,
  } = useBookReader(file.id)

  const { settings, setFont, setSize, setLineHeight, setTheme, cycleTheme } = useBookSettings()
  const { getProgress, saveProgress, updateScroll, bookmarks } = useBookProgress(file.id)

  const [showSidebar, setShowSidebar] = useState(false)
  const [autoHide, setAutoHide] = useState(false)
  const [scrollPercent, setScrollPercent] = useState(0)
  const mainRef = useRef<HTMLDivElement>(null)

  // Restore progress
  const restored = useRef(false)
  useEffect(() => {
    if (restored.current || !spine.length) return
    const saved = getProgress()
    if (saved && saved.chapter > 0) {
      loadChapter(Math.min(saved.chapter, spine.length - 1))
    }
    restored.current = true
  }, [spine, getProgress, loadChapter])

  // Track scroll
  useEffect(() => {
    const el = mainRef.current
    if (!el) return
    const onScroll = () => {
      const pct = handleScroll(el.scrollTop, el.scrollHeight, el.clientHeight)
      setScrollPercent(pct)
      updateScroll(currentChapter, el.scrollTop)
    }
    el.addEventListener('scroll', onScroll)
    return () => el.removeEventListener('scroll', onScroll)
  }, [currentChapter, handleScroll, updateScroll])

  // Save progress on chapter change
  useEffect(() => { saveProgress(currentChapter, 0) }, [currentChapter, saveProgress])

  const goToChapter = useCallback((index: number) => {
    if (index < 0 || index >= spine.length) return
    setShowSidebar(false)
    setSpineTitle(spine[index]?.title || '')
    loadChapter(index)
    if (mainRef.current) mainRef.current.scrollTop = 0
  }, [spine, loadChapter, setSpineTitle])


  // Keyboard
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (showSidebar && e.key === 'Escape') { setShowSidebar(false); return }
      if (e.key === 'ArrowLeft') goToChapter(currentChapter - 1)
      if (e.key === 'ArrowRight') goToChapter(currentChapter + 1)
      if (e.key === 'Escape') onClose()
      if (e.key === 'f' || e.key === 'F') setAutoHide(v => !v)
      if (e.key === 't' || e.key === 'T') cycleTheme()
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [showSidebar, currentChapter, goToChapter, onClose, cycleTheme])

  const toggleFullscreen = useCallback(() => {
    if (!document.fullscreenElement) { document.documentElement.requestFullscreen?.() }
    else { document.exitFullscreen?.() }
  }, [])

  if (error) {
    return (
      <div className="flex flex-col h-full w-full items-center justify-center p-6 text-center bg-slate-100 dark:bg-slate-900">
        <div className="w-16 h-16 rounded-full bg-red-100 dark:bg-red-900/30 flex items-center justify-center mb-4">
          <AlertCircle className="w-8 h-8 text-red-500" />
        </div>
        <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-2">{t('preview.errorTitle', 'Cannot preview')}</h3>
        <p className="text-slate-500 max-w-sm mb-6 text-sm">{error}</p>
      </div>
    )
  }

  return (
    <div className="flex flex-col h-full w-full bg-white dark:bg-slate-950 overflow-hidden relative">
      <ReaderSidebar
        show={showSidebar}
        onClose={() => setShowSidebar(false)}
        displayName={displayName}
        nav={nav}
        spine={spine}
        currentChapter={currentChapter}
        onChapterClick={goToChapter}
          bookmarks={bookmarks}
          history={spine.slice(Math.max(0, currentChapter - 5), currentChapter).reverse()}
      />

      <ReaderTopBar
        title={displayName}
        chapterTitle={spineTitle}
        onToggleSidebar={() => setShowSidebar(v => !v)}
        onClose={onClose}
        settings={settings}
        onFontChange={setFont}
        onSizeChange={setSize}
        onLineHeightChange={setLineHeight}
        onThemeChange={setTheme}
        onCycleTheme={cycleTheme}
        onToggleFullscreen={toggleFullscreen}
        autoHide={autoHide}
      />

      <main ref={mainRef} className="flex-1 overflow-y-auto bg-slate-100 dark:bg-slate-900">
        {(loading || chapterLoading) && (
          <div className="flex items-center justify-center h-full">
            <div className="flex flex-col items-center gap-3">
              <Loader2 className="w-8 h-8 animate-spin text-amber-500" />
              <span className="text-sm text-slate-500">{loading ? 'Preparing book...' : 'Loading chapter...'}</span>
            </div>
          </div>
        )}
        {!loading && !chapterLoading && chapterHtml && (
          <div className="max-w-[750px] mx-auto min-h-full flex flex-col">
            <ReaderContent
              chapterHtml={chapterHtml}
              settings={settings}
              onPrevChapter={() => goToChapter(currentChapter - 1)}
              onNextChapter={() => goToChapter(currentChapter + 1)}
              onToggleUI={() => setAutoHide(v => !v)}
            />
          </div>
        )}
      </main>

      {spine.length > 1 && (
        <ReaderBottomNav
          currentChapter={currentChapter}
          totalChapters={spine.length}
          onPrev={() => goToChapter(currentChapter - 1)}
          onNext={() => goToChapter(currentChapter + 1)}
          progressPercent={scrollPercent}
          autoHide={autoHide}
          visible={true}
        />
      )}
    </div>
  )
}
