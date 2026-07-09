import { useState, useCallback, useMemo } from 'react'
import { Search, BookOpen, Bookmark, Clock, X } from 'lucide-react'

interface NavEntry {
  title: string
  path?: string
  index?: number
  children: NavEntry[]
}

interface SpineEntry {
  title: string
  path: string
  index: number
}

interface Props {
  show: boolean
  onClose: () => void
  displayName: string
  nav: NavEntry[]
  spine: SpineEntry[]
  currentChapter: number
  onChapterClick: (index: number) => void
  bookmarks: any[]
  history: any[]
}

type Tab = 'contents' | 'bookmarks' | 'history'

export function ReaderSidebar({ show, onClose, displayName, nav, spine, currentChapter, onChapterClick, bookmarks, history }: Props) {
  const [activeTab, setActiveTab] = useState<Tab>('contents')
  const [searchQuery, setSearchQuery] = useState('')
  const [expandedVolumes, setExpandedVolumes] = useState<Set<number>>(new Set([0]))

  const toggleVolume = useCallback((idx: number) => {
    setExpandedVolumes(prev => {
      const next = new Set(prev)
      if (next.has(idx)) { next.delete(idx) } else { next.add(idx) }
      return next
    })
  }, [])

  const findChapterIndex = useCallback((path: string): number | null => {
    const entry = spine.find(e => e.path === path)
    return entry ? entry.index : null
  }, [spine])

  const renderNavTree = (entries: NavEntry[], depth = 0): React.ReactNode => {
    return entries.map((entry, i) => {
      if (entry.children.length > 0) {
        const volIdx = nav.indexOf(entry)
        const isExpanded = expandedVolumes.has(volIdx)
        return (
          <div key={depth + '-' + i}>
            <button type="button" onClick={() => toggleVolume(volIdx)}
              className="w-full text-left px-4 py-2 text-sm font-semibold text-slate-700 dark:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-700/50 transition-colors flex items-center gap-2">
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
      const chIndex = entry.index ?? findChapterIndex(entry.path ?? '')
      const isActive = chIndex === currentChapter
      return (
        <button key={depth + '-' + i} type="button"
          onClick={() => chIndex !== null && onChapterClick(chIndex)} disabled={chIndex === null}
          className={`w-full text-left pl-8 pr-4 py-1.5 text-sm transition-colors ${
            isActive
              ? 'bg-amber-50 dark:bg-amber-900/20 text-amber-700 dark:text-amber-400 font-medium border-l-2 border-amber-500'
              : 'text-slate-600 dark:text-slate-400 hover:bg-slate-50 dark:hover:bg-slate-800/50'
          }`}>
          {entry.title}
        </button>
      )
    })
  }

  const filteredNav = useMemo(() => {
    if (!searchQuery) return nav
    const q = searchQuery.toLowerCase()
    const filter = (entries: NavEntry[]): NavEntry[] => {
      return entries.reduce<NavEntry[]>((acc, e) => {
        const children = e.children.length ? filter(e.children) : []
        if (e.title.toLowerCase().includes(q) || children.length > 0) {
          acc.push({ ...e, children })
        }
        return acc
      }, [])
    }
    return filter(nav)
  }, [nav, searchQuery])

  const recentHistory = useMemo(() => {
    if (history.length > 0) return history.slice(0, 10)
    return spine.slice(-5).reverse()
  }, [history, spine])

  return (
    <>
      {show && (
        <div className="fixed inset-0 bg-black/40 z-30 transition-opacity" onClick={onClose} />
      )}
      <aside className={`fixed top-0 left-0 h-full w-[300px] sm:w-[340px] bg-white dark:bg-slate-900 border-r border-slate-200 dark:border-slate-800 z-40 shadow-xl transition-transform duration-300 ease-in-out ${
        show ? 'translate-x-0' : '-translate-x-full'
      }`}>
        <div className="h-14 flex items-center px-4 border-b border-slate-200 dark:border-slate-700 shrink-0">
          <BookOpen className="w-5 h-5 text-amber-500 mr-3 shrink-0" />
          <span className="font-medium text-sm text-slate-900 dark:text-slate-100 truncate flex-1">{displayName}</span>
          <button type="button" onClick={onClose} className="p-1 hover:bg-slate-100 dark:hover:bg-slate-800 rounded">
            <X className="w-4 h-4 text-slate-500" />
          </button>
        </div>

        {activeTab === 'contents' && (
          <div className="px-4 py-2 border-b border-slate-200 dark:border-slate-700">
            <div className="relative">
              <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
              <input
                type="text"
                placeholder="Tìm kiếm..."
                value={searchQuery}
                onChange={e => setSearchQuery(e.target.value)}
                className="w-full pl-8 pr-3 py-1.5 text-sm rounded-md border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 text-slate-900 dark:text-slate-100 placeholder-slate-400 focus:outline-none focus:ring-1 focus:ring-amber-500"
              />
            </div>
          </div>
        )}

        <div className="overflow-y-auto h-[calc(100%-9.5rem)] py-2">
          {activeTab === 'contents' && (
            filteredNav.length > 0 ? renderNavTree(filteredNav) : (
              spine.map((entry, i) => (
                <button key={i} type="button" onClick={() => onChapterClick(i)}
                  className={`w-full text-left px-4 py-2 text-sm transition-colors ${
                    i === currentChapter
                      ? 'bg-amber-50 dark:bg-amber-900/20 text-amber-700 dark:text-amber-400 font-medium'
                      : 'text-slate-600 dark:text-slate-400 hover:bg-slate-50 dark:hover:bg-slate-800/50'
                  }`}>
                  {entry.title}
                </button>
              ))
            )
          )}

          {activeTab === 'bookmarks' && (
            bookmarks.length > 0 ? bookmarks.map((bm, i) => (
              <button key={i} type="button" onClick={() => onChapterClick(bm.chapter)}
                className="w-full text-left px-4 py-3 text-sm text-slate-700 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-800/50 border-b border-slate-100 dark:border-slate-800">
                <div className="font-medium">{bm.label || 'Bookmark'}</div>
                <div className="text-xs text-slate-400 mt-0.5">Chương {bm.chapter + 1}</div>
              </button>
            )) : (
              <div className="flex flex-col items-center justify-center h-40 text-slate-400">
                <Bookmark className="w-8 h-8 mb-2" />
                <span className="text-sm">Chưa có đánh dấu</span>
              </div>
            )
          )}

          {activeTab === 'history' && (
            recentHistory.length > 0 ? recentHistory.map((entry, i) => {
              const chIndex = typeof entry === 'number' ? entry : (entry.index ?? findChapterIndex(entry.path ?? ''))
              return (
                <button key={i} type="button" onClick={() => onChapterClick(chIndex)}
                  className="w-full text-left px-4 py-3 text-sm text-slate-700 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-800/50 border-b border-slate-100 dark:border-slate-800">
                  <div className="font-medium">{entry.title || `Chương ${chIndex + 1}`}</div>
                </button>
              )
            }) : (
              <div className="flex flex-col items-center justify-center h-40 text-slate-400">
                <Clock className="w-8 h-8 mb-2" />
                <span className="text-sm">Chưa có lịch sử</span>
              </div>
            )
          )}
        </div>

        <div className="absolute bottom-0 left-0 right-0 h-12 border-t border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900 flex items-stretch">
          {([
            { id: 'contents', icon: BookOpen, label: 'Mục lục' },
            { id: 'bookmarks', icon: Bookmark, label: 'Đánh dấu' },
            { id: 'history', icon: Clock, label: 'Lịch sử' },
          ] as const).map(tab => (
            <button key={tab.id} type="button" onClick={() => setActiveTab(tab.id)}
              className={`flex-1 flex flex-col items-center justify-center gap-0.5 text-xs transition-colors ${
                activeTab === tab.id
                  ? 'text-amber-600 dark:text-amber-400 border-t-2 border-amber-500 bg-amber-50/50 dark:bg-amber-900/10'
                  : 'text-slate-500 dark:text-slate-400 hover:bg-slate-50 dark:hover:bg-slate-800/50'
              }`}>
              <tab.icon className="w-4 h-4" />
              <span>{tab.label}</span>
            </button>
          ))}
        </div>
      </aside>
    </>
  )
}
