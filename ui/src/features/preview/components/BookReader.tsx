import { useState, useEffect, useRef, useCallback } from 'react'
import { Loader2, AlertCircle, BookOpen, X, ChevronLeft, ChevronRight, Menu } from 'lucide-react'
import { invoke } from '@tauri-apps/api/core'
import { useTranslation } from 'react-i18next'

interface SpineEntry {
  index: number
  title: string
  path: string
}

interface NavEntry {
  title: string
  path: string
  index: number | null
  children: NavEntry[]
}

export function BookReader({ file, onClose, onDownload }) {
  const { t } = useTranslation()
  const displayName = file.filename || file.name || ''

  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [spine, setSpine] = useState<SpineEntry[]>([])
  const [nav, setNav] = useState<NavEntry[]>([])
  const [currentChapter, setCurrentChapter] = useState(0)
  const [chapterHtml, setChapterHtml] = useState<string | null>(null)
  const [chapterLoading, setChapterLoading] = useState(false)
  const [showSidebar, setShowSidebar] = useState(false)
  const [expandedVolumes, setExpandedVolumes] = useState<Set<number>>(new Set([0]))
  const shadowRef = useRef<HTMLDivElement>(null)
  const bridgePort = useRef<number | null>(null)

  // fetch spine + nav on mount
  useEffect(() => {
    let cancelled = false

    const init = async () => {
      try {
        setLoading(true)
        setError(null)

        const port = await invoke<number>('get_book_bridge_port')
        bridgePort.current = port
        const baseUrl = `http://127.0.0.1:${port}`

        // fetch spine + nav concurrently
        const [spineRes, navRes] = await Promise.all([
          fetch(`${baseUrl}/book/${file.id}/spine`),
          fetch(`${baseUrl}/book/${file.id}/nav`).catch(() => null),
        ])
        if (!spineRes.ok) throw new Error(await spineRes.text())
        const entries: SpineEntry[] = await spineRes.json()
        if (cancelled) return
        setSpine(entries)

        if (navRes && navRes.ok) {
          const navEntries: NavEntry[] = await navRes.json()
          if (!cancelled) {
            setNav(navEntries)
            setExpandedVolumes(new Set(navEntries.map((_, i) => i)))
          }
        }

        if (entries.length > 0) {
          await loadChapter(0, port, file.id)
        }
      } catch (err) {
        if (!cancelled) setError((err as any)?.message || String(err))
      } finally {
        if (!cancelled) setLoading(false)
      }
    }

    init()
    return () => { cancelled = true }
  }, [file.id])

  const loadChapter = useCallback(async (index: number, port: number, fileId: number) => {
    setChapterLoading(true)
    try {
      const res = await fetch(`http://127.0.0.1:${port}/book/${fileId}/chapter/${index}`)
      if (!res.ok) throw new Error(await res.text())
      const html = await res.text()
      setChapterHtml(html)
      setCurrentChapter(index)
    } catch (err) {
      setError((err as any)?.message || String(err))
    } finally {
      setChapterLoading(false)
    }
  }, [])

  const goToChapter = useCallback((index: number) => {
    if (index < 0 || index >= spine.length) return
    setShowSidebar(false)
    loadChapter(index, bridgePort.current!, file.id)
  }, [spine, loadChapter, file.id])

  // Shadow DOM render
  useEffect(() => {
    if (!chapterHtml || !shadowRef.current) return
    const existing = shadowRef.current.shadowRoot
    if (existing) existing.innerHTML = ''
    const root = shadowRef.current.shadowRoot || shadowRef.current.attachShadow({ mode: 'open' })
    root.innerHTML = chapterHtml
    return () => { if (shadowRef.current?.shadowRoot) shadowRef.current.shadowRoot.innerHTML = '' }
  }, [chapterHtml])

  // Keyboard: ← → ESC
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (showSidebar && e.key === 'Escape') { setShowSidebar(false); return }
      if (e.key === 'ArrowLeft') goToChapter(currentChapter - 1)
      if (e.key === 'ArrowRight') goToChapter(currentChapter + 1)
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [showSidebar, currentChapter, goToChapter, onClose])

  // toggle volume expand
  const toggleVolume = useCallback((idx: number) => {
    setExpandedVolumes(prev => {
      const next = new Set(prev)
      if (next.has(idx)) { next.delete(idx) } else { next.add(idx) }
      return next
    })
  }, [])

  // find chapter index from nav path
  const findChapterIndex = useCallback((path: string): number | null => {
    const entry = spine.find(e => e.path === path)
    return entry ? entry.index : null
  }, [spine])

  const renderNavTree = (entries: NavEntry[], depth: number = 0) => {
    return entries.map((entry, i) => {
      if (entry.children.length > 0) {
        // volume entry
        const volIdx = nav.indexOf(entry)
        const isExpanded = expandedVolumes.has(volIdx)
        return (
          <div key={depth + '-' + i}>
            <button
              type="button"
              onClick={() => toggleVolume(volIdx)}
              className="w-full text-left px-4 py-2 text-sm font-semibold text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-700/50 transition-colors flex items-center gap-2"
            >
              <span className={`text-xs transition-transform ${isExpanded ? 'rotate-90' : ''}`}>▶</span>
              {entry.title}
            </button>
            {isExpanded && (
              <div className="ml-2 border-l border-slate-200 dark:border-slate-700">
                {renderNavTree(entry.children, depth + 1)}
              </div>
            )}
          </div>
        )
      }

      // chapter entry
      const chIndex = entry.index ?? findChapterIndex(entry.path)
      const isActive = chIndex === currentChapter
      return (
        <button
          key={depth + '-' + i}
          type="button"
          onClick={() => chIndex !== null && goToChapter(chIndex)}
          disabled={chIndex === null}
          className={`w-full text-left pl-8 pr-4 py-1.5 text-sm transition-colors ${
            isActive
              ? 'bg-amber-50 dark:bg-amber-900/20 text-amber-700 dark:text-amber-400 font-medium border-l-2 border-amber-500'
              : 'text-slate-600 dark:text-slate-400 hover:bg-slate-50 dark:hover:bg-slate-800/50'
          }`}
        >
          {entry.title}
        </button>
      )
    })
  }

  if (error) {
    return (
      <div className="flex flex-col h-full w-full items-center justify-center p-6 text-center bg-slate-100 dark:bg-slate-900">
        <div className="w-16 h-16 rounded-full bg-red-100 dark:bg-red-900/30 flex items-center justify-center mb-4">
          <AlertCircle className="w-8 h-8 text-red-500" />
        </div>
        <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-2">
          {t('preview.errorTitle', 'Cannot preview')}
        </h3>
        <p className="text-slate-500 max-w-sm mb-6 text-sm">{error}</p>
      </div>
    )
  }

  return (
    <div className="flex flex-col h-full w-full bg-white dark:bg-slate-950 overflow-hidden relative">
      {/* Backdrop */}
      {showSidebar && (
        <div
          className="fixed inset-0 bg-black/40 z-30 transition-opacity"
          onClick={() => setShowSidebar(false)}
        />
      )}

      {/* Sidebar */}
      <aside
        className={`fixed top-0 left-0 h-full w-[300px] sm:w-[340px] bg-white dark:bg-slate-900 border-r border-slate-200 dark:border-slate-800 z-40 shadow-xl transition-transform duration-300 ease-in-out ${
          showSidebar ? 'translate-x-0' : '-translate-x-full'
        }`}
      >
        <div className="h-14 flex items-center px-4 border-b border-slate-200 dark:border-slate-700 shrink-0">
          <BookOpen className="w-5 h-5 text-amber-500 mr-3 shrink-0" />
          <span className="font-medium text-sm text-slate-900 dark:text-slate-100 truncate">
            {displayName}
          </span>
        </div>
        <div className="overflow-y-auto h-[calc(100%-3.5rem)] py-2">
          {nav.length > 0 ? renderNavTree(nav) : (
            spine.map((entry, i) => (
              <button
                key={i}
                type="button"
                onClick={() => goToChapter(i)}
                className={`w-full text-left px-4 py-2 text-sm transition-colors ${
                  i === currentChapter
                    ? 'bg-amber-50 dark:bg-amber-900/20 text-amber-700 dark:text-amber-400 font-medium'
                    : 'text-slate-600 dark:text-slate-400 hover:bg-slate-50 dark:hover:bg-slate-800/50'
                }`}
              >
                {entry.title}
              </button>
            ))
          )}
        </div>
      </aside>

      {/* Top bar */}
      <header className="h-12 flex items-center justify-between px-3 border-b border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 shrink-0 z-10">
        <button
          type="button"
          onClick={() => setShowSidebar(v => !v)}
          className="p-2 -ml-1 rounded-lg hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-600 dark:text-slate-400 transition-colors"
          title={t('book.toc', 'Table of Contents')}
        >
          <Menu className="w-5 h-5" />
        </button>

        <span className="text-sm font-medium text-slate-800 dark:text-slate-200 truncate mx-2">
          {displayName}
        </span>

        <button
          type="button"
          onClick={onClose}
          className="p-2 rounded-lg hover:bg-red-50 dark:hover:bg-red-900/30 text-slate-600 dark:text-slate-400 hover:text-red-500 transition-colors"
          title={t('common.close', 'Close')}
        >
          <X className="w-5 h-5" />
        </button>
      </header>

      {/* Reading area */}
      <main className="flex-1 overflow-y-auto bg-slate-100 dark:bg-slate-900">
        {(loading || chapterLoading) && (
          <div className="flex items-center justify-center h-full">
            <div className="flex flex-col items-center gap-3">
              <Loader2 className="w-8 h-8 animate-spin text-amber-500" />
              <span className="text-sm text-slate-500">
                {loading ? 'Preparing book...' : 'Loading chapter...'}
              </span>
            </div>
          </div>
        )}

        {!loading && !chapterLoading && chapterHtml && (
          <div className="max-w-[750px] mx-auto min-h-full flex flex-col">
            {/* Chapter content */}
            <div
              ref={shadowRef}
              className="flex-1 px-8 sm:px-12 py-8"
              style={{
                fontFamily: 'Georgia, "Noto Serif", serif',
                fontSize: '20px',
                lineHeight: 1.8,
                color: '#1a1a1a',
              }}
            />

            {/* Navigation bar */}
            {spine.length > 1 && (
              <div className="flex items-center justify-between px-8 py-6 border-t border-slate-200 dark:border-slate-800">
                <button
                  type="button"
                  disabled={currentChapter <= 0}
                  onClick={() => goToChapter(currentChapter - 1)}
                  className="flex items-center gap-1 px-4 py-2 rounded-lg text-sm text-slate-600 dark:text-slate-400 hover:bg-slate-200 dark:hover:bg-slate-800 disabled:opacity-30 disabled:pointer-events-none transition-colors"
                >
                  <ChevronLeft className="w-4 h-4" />
                  {t('book.previous', 'Previous')}
                </button>
                <span className="text-xs text-slate-400">
                  {currentChapter + 1} / {spine.length}
                </span>
                <button
                  type="button"
                  disabled={currentChapter >= spine.length - 1}
                  onClick={() => goToChapter(currentChapter + 1)}
                  className="flex items-center gap-1 px-4 py-2 rounded-lg text-sm text-slate-600 dark:text-slate-400 hover:bg-slate-200 dark:hover:bg-slate-800 disabled:opacity-30 disabled:pointer-events-none transition-colors"
                >
                  {t('book.next', 'Next')}
                  <ChevronRight className="w-4 h-4" />
                </button>
              </div>
            )}
          </div>
        )}
      </main>
    </div>
  )
}
