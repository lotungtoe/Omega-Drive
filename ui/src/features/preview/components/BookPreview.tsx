import { useState, useEffect, useRef } from 'react'
import { Download, Loader2, AlertCircle, BookOpen, X } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { invoke } from '@tauri-apps/api/core'
import { getColor, formatSize } from '../../../shared/utils'
import { normalizeError } from '../../../shared/services/errors/normalizeError'

export function BookPreview({ file, onClose, onDownload }) {
  const { t } = useTranslation()
  const displayName = file.filename || file.name || ''
  const color = getColor(displayName, file.kind)

  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(null)
  const viewerRef = useRef(null)

  useEffect(() => {
    let cancelled = false
    let rendition = null

    const loadBook = async () => {
      try {
        setLoading(true)
        setError(null)

        const binaryData = await invoke('retrieve_full_file', { fileId: file.id })
        if (cancelled) return

        const ePub = (await import('epubjs')).default
        if (cancelled || !viewerRef.current) return

        const book = ePub(new Uint8Array(binaryData as any).buffer)
        rendition = book.renderTo(viewerRef.current, {
          width: '100%',
          height: '100%',
          flow: 'scrolled-doc',
          spread: 'none',
        })
        await rendition.display()
      } catch (err) {
        console.error('Failed to load book preview:', err)
        if (!cancelled) setError(normalizeError(err).message)
      } finally {
        if (!cancelled) setLoading(false)
      }
    }

    loadBook()
    return () => {
      cancelled = true
      if (rendition) rendition.destroy?.()
    }
  }, [file.id])

  return (
    <div className="flex flex-col h-full w-full bg-slate-100 dark:bg-slate-900 overflow-hidden relative">
      {/* Header */}
      <div className="h-14 flex items-center justify-between px-4 border-b border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 shrink-0 z-10">
        <div className="flex items-center gap-3 overflow-hidden">
          <div className="p-2 rounded-lg bg-amber-50 dark:bg-amber-900/30 shrink-0">
            <BookOpen className="w-5 h-5" style={{ color }} />
          </div>
          <div className="flex flex-col min-w-0">
            <span className="font-medium text-sm text-slate-900 dark:text-slate-100 truncate">
              {displayName}
            </span>
            <span className="text-xs text-slate-500">
              {formatSize(file.size)} &bull; eBook
            </span>
          </div>
        </div>

        <div className="flex items-center gap-1 shrink-0">
          <button type="button"
            onClick={() => onDownload(file)}
            className="p-2 rounded-full hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-600 dark:text-slate-400 transition-colors"
            title={t('common.download', 'Download')}
          >
            <Download className="w-5 h-5" />
          </button>
          <button type="button"
            onClick={onClose}
            className="p-2 rounded-full hover:bg-red-50 dark:hover:bg-red-900/30 text-slate-600 dark:text-slate-400 hover:text-red-500 dark:hover:text-red-400 transition-colors"
            title={t('common.close', 'Close')}
          >
            <X className="w-5 h-5" />
          </button>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto relative bg-slate-200 dark:bg-slate-800 flex justify-center">
        {loading && (
          <div className="absolute inset-0 flex items-center justify-center bg-slate-100/60 dark:bg-slate-900/60 backdrop-blur-sm z-10">
            <Loader2 className="w-8 h-8 animate-spin text-amber-500" />
          </div>
        )}

        {error ? (
          <div className="absolute inset-0 flex flex-col items-center justify-center p-6 text-center">
            <div className="w-16 h-16 rounded-full bg-red-100 dark:bg-red-900/30 flex items-center justify-center mb-4">
              <AlertCircle className="w-8 h-8 text-red-500" />
            </div>
            <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-2">
              {t('preview.errorTitle', 'Cannot preview')}
            </h3>
            <p className="text-slate-500 max-w-sm mb-6 text-sm">{error}</p>
            <button type="button"
              onClick={() => onDownload(file)}
              className="flex items-center gap-2 px-6 py-2.5 bg-blue-500 hover:bg-blue-600 text-white rounded-xl font-medium transition-colors"
            >
              <Download className="w-4 h-4" />
              {t('common.download', 'Download file')}
            </button>
          </div>
        ) : (
          <div
            ref={viewerRef}
            className="w-full max-w-4xl bg-white shadow-xl min-h-full"
            style={{ minHeight: loading ? '400px' : undefined }}
          />
        )}
      </div>

      {/* ponytail: scrolled-doc flow, no TOC nav — add if users ask for chapter jumping */}
      <style>{`
        .epub-container { padding: 2rem 3rem; color: #1e293b; line-height: 1.8; }
        .epub-container img { max-width: 100%; height: auto; }
        @media (prefers-color-scheme: dark) {
          .epub-container { color: #e2e8f0; }
        }
      `}</style>
    </div>
  )
}
