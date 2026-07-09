import { useState, useCallback, useRef, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'

export interface SpineEntry {
  index: number
  title: string
  path: string
}

export interface NavEntry {
  title: string
  path: string
  index: number | null
  children: NavEntry[]
}

export function useBookReader(fileId: number) {
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [spine, setSpine] = useState<SpineEntry[]>([])
  const [nav, setNav] = useState<NavEntry[]>([])
  const [currentChapter, setCurrentChapter] = useState(0)
  const [chapterHtml, setChapterHtml] = useState<string | null>(null)
  const [chapterLoading, setChapterLoading] = useState(false)
  const [spineTitle, setSpineTitle] = useState('')
  const bridgePort = useRef<number | null>(null)
  const prefetched = useRef<Map<number, string>>(new Map())

  const fetchChapter = useCallback(async (index: number): Promise<string> => {
    const cached = prefetched.current.get(index)
    if (cached) {
      prefetched.current.delete(index)
      return cached
    }
    const res = await fetch(`http://127.0.0.1:${bridgePort.current}/book/${fileId}/chapter/${index}`)
    if (!res.ok) throw new Error(await res.text())
    return res.text()
  }, [fileId])

  const loadChapter = useCallback(async (index: number) => {
    setChapterLoading(true)
    try {
      const html = await fetchChapter(index)
      setChapterHtml(html)
      setCurrentChapter(index)
    } catch (err) {
      setError((err as any)?.message || String(err))
    } finally {
      setChapterLoading(false)
    }
  }, [fetchChapter])

  const prefetchNextChapter = useCallback((index: number) => {
    if (index >= spine.length - 1 || prefetched.current.has(index + 1)) return
    fetchChapter(index + 1).then(html => prefetched.current.set(index + 1, html)).catch(() => {})
  }, [spine.length, fetchChapter])

  // Detect near-bottom for prefetch
  const mainRef = useRef<HTMLDivElement | null>(null)
  const prefetchTriggered = useRef(false)

  const handleScroll = useCallback((scrollTop: number, scrollHeight: number, clientHeight: number) => {
    const pct = scrollHeight > clientHeight ? (scrollTop / (scrollHeight - clientHeight)) * 100 : 100
    if (pct > 85 && !prefetchTriggered.current) {
      prefetchTriggered.current = true
      prefetchNextChapter(currentChapter)
    }
    if (pct < 50) prefetchTriggered.current = false
    return Math.min(100, Math.max(0, pct))
  }, [currentChapter, prefetchNextChapter])

  useEffect(() => {
    let cancelled = false
    const init = async () => {
      try {
        setLoading(true)
        const port = await invoke<number>('get_book_bridge_port')
        bridgePort.current = port
        const base = `http://127.0.0.1:${port}`

        const [spineRes, navRes] = await Promise.all([
          fetch(`${base}/book/${fileId}/spine`),
          fetch(`${base}/book/${fileId}/nav`).catch(() => null),
        ])
        if (!spineRes.ok) throw new Error(await spineRes.text())
        const entries: SpineEntry[] = await spineRes.json()
        if (cancelled) return
        setSpine(entries)
        setSpineTitle(entries[0]?.title || '')

        if (navRes && navRes.ok) {
          const navEntries: NavEntry[] = await navRes.json()
          if (!cancelled) setNav(navEntries)
        }

        if (entries.length > 0) await loadChapter(0)
      } catch (err) {
        if (!cancelled) setError((err as any)?.message || String(err))
      } finally {
        if (!cancelled) setLoading(false)
      }
    }
    init()
    return () => { cancelled = true }
  }, [fileId, loadChapter])

  return {
    loading, error, spine, nav, currentChapter, chapterHtml, chapterLoading, spineTitle,
    setError, setSpineTitle, loadChapter, handleScroll,
  }
}
