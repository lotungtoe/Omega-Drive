import { useEffect, useRef } from 'react'
import type { ReaderSettings } from '../utils/injectReaderStyles'
import { injectReaderStyles } from '../utils/injectReaderStyles'

interface Props {
  chapterHtml: string | null
  settings: ReaderSettings
  onPrevChapter: () => void
  onNextChapter: () => void
  onToggleUI: () => void
}

export function ReaderContent({ chapterHtml, settings, onPrevChapter, onNextChapter, onToggleUI }: Props) {
  const shadowRef = useRef<HTMLDivElement>(null)
  const containerRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!chapterHtml || !shadowRef.current) return
    const root = shadowRef.current.shadowRoot || shadowRef.current.attachShadow({ mode: 'open' })
    const styleTag = injectReaderStyles(settings)
    root.innerHTML = styleTag + chapterHtml
  }, [chapterHtml, settings])

  const handleClick = (e: React.MouseEvent) => {
    if (!containerRef.current) return
    const rect = containerRef.current.getBoundingClientRect()
    const x = e.clientX - rect.left
    const third = rect.width / 3
    if (x < third) onPrevChapter()
    else if (x > rect.width - third) onNextChapter()
    else onToggleUI()
  }

  return (
    <div ref={containerRef} className="flex-1 w-full cursor-pointer" onClick={handleClick}>
      {chapterHtml && (
        <div ref={shadowRef} className="max-w-[750px] mx-auto px-8 sm:px-12 py-8 min-h-full" />
      )}
    </div>
  )
}
