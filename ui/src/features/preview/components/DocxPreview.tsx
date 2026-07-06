import { useState, useEffect, useRef } from 'react'
import { Download, Loader2, AlertCircle, FileText } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { invoke } from '@tauri-apps/api/core'
import { getColor, formatSize } from '../../../shared/utils'

export function DocxPreview({ file, onClose: _onClose, onDownload }) {
  const { t } = useTranslation()
  const displayName = file.filename || file.name || ''
  const color = getColor(displayName, file.kind)

  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(null)
  const containerRef = useRef(null)

  useEffect(() => {
    let cancelled = false

    const loadDocx = async () => {
      try {
        setLoading(true)
        setError(null)

        const binaryData = await invoke('retrieve_full_file', { fileId: file.id })
        if (cancelled) return

        // Dynamically import docx-preview to keep initial bundle small
        const { renderAsync } = await import('docx-preview')
        if (cancelled || !containerRef.current) return

        // Clear any previous render
        containerRef.current.innerHTML = ''

        await renderAsync(
          new Uint8Array(binaryData).buffer,
          containerRef.current,
          null, // styleContainer â€“ null = inline styles
          {
            className: 'docx',
            inWrapper: true,
            ignoreWidth: false,
            ignoreHeight: false,
            ignoreFonts: false,
            breakPages: true,
            debug: false,
          }
        )
      } catch (err) {
        console.error('Failed to load DOCX preview:', err)
        if (!cancelled) setError(err.toString())
      } finally {
        if (!cancelled) setLoading(false)
      }
    }

    loadDocx()
    return () => { cancelled = true }
  }, [file.id])

  return (
    <div className="flex flex-col h-full w-full bg-slate-100 dark:bg-slate-900 overflow-hidden relative">
      {/* Header */}
      <div className="h-14 flex items-center justify-between px-4 border-b border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 shrink-0 z-10">
        <div className="flex items-center gap-3 overflow-hidden">
          <div className="p-2 rounded-lg bg-blue-50 dark:bg-blue-900/30 shrink-0">
            <FileText className="w-5 h-5" style={{ color }} />
          </div>
          <div className="flex flex-col min-w-0">
            <span className="font-medium text-sm text-slate-900 dark:text-slate-100 truncate">
              {displayName}
            </span>
            <span className="text-xs text-slate-500">
              {formatSize(file.size)} &bull; Word Document
            </span>
          </div>
        </div>

        <button type="button"
          onClick={() => onDownload(file)}
          className="p-2 rounded-full hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-600 dark:text-slate-400 transition-colors shrink-0"
          title={t('common.download', 'Táº£i xuá»‘ng')}
        >
          <Download className="w-5 h-5" />
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto relative bg-slate-200 dark:bg-slate-800 flex justify-center py-6 px-4">
        {loading && (
          <div className="absolute inset-0 flex items-center justify-center bg-slate-100/60 dark:bg-slate-900/60 backdrop-blur-sm z-10">
            <Loader2 className="w-8 h-8 animate-spin text-blue-500" />
          </div>
        )}

        {error ? (
          <div className="absolute inset-0 flex flex-col items-center justify-center p-6 text-center">
            <div className="w-16 h-16 rounded-full bg-red-100 dark:bg-red-900/30 flex items-center justify-center mb-4">
              <AlertCircle className="w-8 h-8 text-red-500" />
            </div>
            <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-2">
              {t('preview.errorTitle', 'KhĂ´ng thá»ƒ xem trÆ°á»›c')}
            </h3>
            <p className="text-slate-500 max-w-sm mb-6 text-sm">{error}</p>
            <button type="button"
              onClick={() => onDownload(file)}
              className="flex items-center gap-2 px-6 py-2.5 bg-blue-500 hover:bg-blue-600 text-white rounded-xl font-medium transition-colors"
            >
              <Download className="w-4 h-4" />
              {t('common.download', 'Táº£i xuá»‘ng tá»‡p')}
            </button>
          </div>
        ) : (
          <div
            ref={containerRef}
            className="docx-container bg-white shadow-xl max-w-4xl w-full min-h-full"
            style={{ minHeight: loading ? '400px' : undefined }}
          />
        )}
      </div>

      {/* Scoped CSS to normalise docx-preview output */}
      <style>{`
        .docx-container .docx { padding: 2.5rem; font-family: 'Times New Roman', serif; line-height: 1.6; color: #1e293b; }
        .docx-container table { border-collapse: collapse; width: 100%; }
        .docx-container td, .docx-container th { border: 1px solid #cbd5e1; padding: 4px 8px; }
        .docx-container img { max-width: 100%; height: auto; }
      `}</style>
    </div>
  )
}
