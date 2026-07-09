import { useState, useCallback, useRef, useEffect } from 'react'

interface ProgressEntry {
  fileId: number
  chapter: number
  scrollY: number
  timestamp: number
}

interface Bookmark {
  fileId: number
  chapter: number
  scrollY: number
  label: string
  timestamp: number
}

const PROGRESS_KEY = 'book_progress'
const BOOKMARK_KEY = 'book_bookmarks'
const MAX_PROGRESS = 20
const SAVE_INTERVAL = 15000

function loadProgress(): ProgressEntry[] {
  try {
    const raw = localStorage.getItem(PROGRESS_KEY)
    if (raw) return JSON.parse(raw) as ProgressEntry[]
  } catch {}
  return []
}

function loadBookmarks(): Bookmark[] {
  try {
    const raw = localStorage.getItem(BOOKMARK_KEY)
    if (raw) return JSON.parse(raw) as Bookmark[]
  } catch {}
  return []
}

export function useBookProgress(fileId: number) {
  const [bookmarks, setBookmarks] = useState<Bookmark[]>(loadBookmarks)
  const lastSave = useRef<ProgressEntry | null>(null)

  const getProgress = useCallback((): ProgressEntry | null => {
    const all = loadProgress()
    return all.find(e => e.fileId === fileId) ?? null
  }, [fileId])

  const saveProgress = useCallback((chapter: number, scrollY: number) => {
    const all = loadProgress().filter(e => e.fileId !== fileId)
    all.push({ fileId, chapter, scrollY, timestamp: Date.now() })
    all.sort((a, b) => b.timestamp - a.timestamp)
    const trimmed = all.slice(0, MAX_PROGRESS)
    localStorage.setItem(PROGRESS_KEY, JSON.stringify(trimmed))
  }, [fileId])

  const autoSaveRef = useRef(saveProgress)
  autoSaveRef.current = saveProgress
  useEffect(() => {
    const timer = setInterval(() => {
      if (lastSave.current) {
        autoSaveRef.current(lastSave.current.chapter, lastSave.current.scrollY)
        lastSave.current = null
      }
    }, SAVE_INTERVAL)
    return () => clearInterval(timer)
  }, [])

  const updateScroll = useCallback((chapter: number, scrollY: number) => {
    lastSave.current = { fileId, chapter, scrollY, timestamp: Date.now() }
  }, [fileId])

  const toggleBookmark = useCallback((chapter: number, scrollY: number, label: string) => {
    const all = loadBookmarks()
    const idx = all.findIndex(b => b.fileId === fileId && b.chapter === chapter && b.scrollY === scrollY)
    if (idx >= 0) {
      all.splice(idx, 1)
    } else {
      all.push({ fileId, chapter, scrollY, label, timestamp: Date.now() })
    }
    localStorage.setItem(BOOKMARK_KEY, JSON.stringify(all))
    setBookmarks(all)
  }, [fileId])

  return { getProgress, saveProgress, updateScroll, toggleBookmark, bookmarks: bookmarks.filter(b => b.fileId === fileId) }
}
