import { useState, useEffect, useRef } from 'react'
import { Download, Loader2, AlertCircle, FileText, ChevronLeft, ChevronRight, ZoomIn, ZoomOut, Maximize2, Minimize2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { invoke } from '@tauri-apps/api/core'
import { getColor, formatSize } from '../../../shared/utils'
import { Document, Page, pdfjs } from 'react-pdf'
import 'react-pdf/dist/Page/AnnotationLayer.css'
import 'react-pdf/dist/Page/TextLayer.css'

// Configure worker for Vite
pdfjs.GlobalWorkerOptions.workerSrc = new URL(
  'pdfjs-dist/build/pdf.worker.min.mjs',
  import.meta.url,
).toString();

export function PdfPreview({ file, onClose: _onClose, onDownload }) {
  const { t } = useTranslation()
  const displayName = file.filename || file.name || ''
  const color = getColor(displayName, file.kind)
  
  const [pdfData, setPdfData] = useState(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(null)
  
  const [numPages, setNumPages] = useState(null)
  const [pageNumber, setPageNumber] = useState(1)
  const [scale, setScale] = useState(1)
  const [isFullscreen, setIsFullscreen] = useState(false)
  const containerRef = useRef(null)

  useEffect(() => {
    const loadPdf = async () => {
      try {
        setLoading(true)
        setError(null)
        
        const binaryData = await invoke('retrieve_full_file', { fileId: file.id })
        
        // Pass the raw Uint8Array data directly to react-pdf instead of ObjectURL 
        // to avoid some CORS/ObjectURL restrictions in webviews
        setPdfData({ data: new Uint8Array(binaryData) })
      } catch (err) {
        console.error("Failed to load PDF preview:", err)
        setError(err.toString())
      } finally {
        setLoading(false)
      }
    }

    loadPdf()
    return undefined
  }, [file.id])

  const onDocumentLoadSuccess = ({ numPages }) => {
    setNumPages(numPages)
    setPageNumber(1)
  }

  const changePage = (offset) => {
    setPageNumber(prevPageNumber => {
      const newPage = prevPageNumber + offset
      if (newPage >= 1 && newPage <= numPages) return newPage
      return prevPageNumber
    })
  }

  const handleZoomIn = () => setScale(s => Math.min(s + 0.25, 3))
  const handleZoomOut = () => setScale(s => Math.max(s - 0.25, 0.5))
  
  const toggleFullscreen = () => {
    if (document.fullscreenElement) {
      document.exitFullscreen()
    } else {
      containerRef.current?.requestFullscreen().catch(err => {
        console.error(`Error attempting to enable fullscreen: ${err.message}`)
      })
    }
  }

  useEffect(() => {
    const handleFullscreenChange = () => {
      setIsFullscreen(!!document.fullscreenElement)
    }
    document.addEventListener('fullscreenchange', handleFullscreenChange)
    return () => document.removeEventListener('fullscreenchange', handleFullscreenChange)
  }, [])

  // Auto-resize scale based on container width initially
  const [containerWidth, setContainerWidth] = useState(null)
  useEffect(() => {
    if (containerRef.current) {
      setContainerWidth(containerRef.current.clientWidth)
    }
    const handleResize = () => {
      if (containerRef.current) setContainerWidth(containerRef.current.clientWidth)
    }
    window.addEventListener('resize', handleResize)
    return () => window.removeEventListener('resize', handleResize)
  }, [isFullscreen])

  return (
    <div ref={containerRef} className="flex flex-col h-full w-full bg-slate-100 dark:bg-slate-900 overflow-hidden relative">
      {/* Header */}
      {!isFullscreen && (
        <div className="h-14 flex items-center justify-between px-4 border-b border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 shrink-0 z-10">
          <div className="flex items-center gap-3 overflow-hidden">
            <div className="p-2 rounded-lg bg-slate-100 dark:bg-slate-800 shrink-0">
              <FileText className="w-5 h-5" style={{ color }} />
            </div>
            <div className="flex flex-col min-w-0">
              <span className="font-medium text-sm text-slate-900 dark:text-slate-100 truncate">
                {displayName}
              </span>
              <span className="text-xs text-slate-500">
                {formatSize(file.size)} &bull; PDF
              </span>
            </div>
          </div>
          
          <div className="flex items-center gap-2 shrink-0">
            <button type="button"
              onClick={() => onDownload(file)}
              className="p-2 rounded-full hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-600 dark:text-slate-400 transition-colors"
              title={t('common.download', 'Táº£i xuá»‘ng')}
            >
              <Download className="w-5 h-5" />
            </button>
          </div>
        </div>
      )}

      {/* Toolbar */}
      <div className="h-12 flex items-center justify-center gap-4 px-4 bg-white/80 dark:bg-slate-950/80 backdrop-blur-md border-b border-slate-200 dark:border-slate-800 shrink-0 z-10">
        <div className="flex items-center bg-slate-100 dark:bg-slate-800 rounded-lg p-1">
          <button type="button"
            onClick={() => changePage(-1)}
            disabled={pageNumber <= 1 || !numPages}
            className="p-1.5 rounded-md hover:bg-white dark:hover:bg-slate-700 disabled:opacity-50 text-slate-700 dark:text-slate-300"
          >
            <ChevronLeft className="w-4 h-4" />
          </button>
          <span className="px-3 text-sm font-medium text-slate-700 dark:text-slate-300 min-w-[5rem] text-center">
            {pageNumber} / {numPages || '--'}
          </span>
          <button type="button"
            onClick={() => changePage(1)}
            disabled={pageNumber >= numPages || !numPages}
            className="p-1.5 rounded-md hover:bg-white dark:hover:bg-slate-700 disabled:opacity-50 text-slate-700 dark:text-slate-300"
          >
            <ChevronRight className="w-4 h-4" />
          </button>
        </div>

        <div className="w-px h-6 bg-slate-300 dark:bg-slate-700 mx-2" />

        <div className="flex items-center gap-1">
          <button type="button"
            onClick={handleZoomOut}
            disabled={scale <= 0.5}
            className="p-2 rounded-lg hover:bg-slate-200 dark:hover:bg-slate-800 text-slate-700 dark:text-slate-300 disabled:opacity-50"
          >
            <ZoomOut className="w-4 h-4" />
          </button>
          <span className="text-sm font-medium w-12 text-center text-slate-700 dark:text-slate-300">
            {Math.round(scale * 100)}%
          </span>
          <button type="button"
            onClick={handleZoomIn}
            disabled={scale >= 3}
            className="p-2 rounded-lg hover:bg-slate-200 dark:hover:bg-slate-800 text-slate-700 dark:text-slate-300 disabled:opacity-50"
          >
            <ZoomIn className="w-4 h-4" />
          </button>
        </div>

        <div className="w-px h-6 bg-slate-300 dark:bg-slate-700 mx-2" />
        
        <button type="button"
          onClick={toggleFullscreen}
          className="p-2 rounded-lg hover:bg-slate-200 dark:hover:bg-slate-800 text-slate-700 dark:text-slate-300"
        >
          {isFullscreen ? <Minimize2 className="w-4 h-4" /> : <Maximize2 className="w-4 h-4" />}
        </button>
      </div>

      {/* Content Area */}
      <div className="flex-1 overflow-auto relative flex justify-center py-6">
        {loading && (
          <div className="absolute inset-0 flex items-center justify-center bg-slate-100/50 dark:bg-slate-900/50 backdrop-blur-sm z-10">
            <Loader2 className="w-8 h-8 animate-spin text-indigo-500" />
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
            <p className="text-slate-500 max-w-sm mb-6">
              {error}
            </p>
            <button type="button"
              onClick={() => onDownload(file)}
              className="flex items-center gap-2 px-6 py-2.5 bg-indigo-500 hover:bg-indigo-600 text-white rounded-xl font-medium transition-colors"
            >
              <Download className="w-4 h-4" />
              {t('common.download', 'Táº£i xuá»‘ng tá»‡p')}
            </button>
          </div>
        ) : (
          pdfData && (
            <div className="shadow-2xl ring-1 ring-slate-900/5 dark:ring-white/10 bg-white">
              <Document
                file={pdfData}
                onLoadSuccess={onDocumentLoadSuccess}
                loading={
                  <div className="w-full h-64 flex items-center justify-center">
                    <Loader2 className="w-6 h-6 animate-spin text-indigo-500" />
                  </div>
                }
                error={
                  <div className="p-8 text-red-500 flex flex-col items-center gap-2">
                    <AlertCircle className="w-8 h-8" />
                    <span>Failed to load PDF</span>
                  </div>
                }
              >
                <Page 
                  pageNumber={pageNumber} 
                  scale={scale} 
                  width={containerWidth ? Math.min(containerWidth - 48, 1000) : undefined}
                  renderTextLayer={true}
                  renderAnnotationLayer={true}
                  className="dark:invert dark:hue-rotate-180" 
                />
              </Document>
            </div>
          )
        )}
      </div>
    </div>
  )
}
