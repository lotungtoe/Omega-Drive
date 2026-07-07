import { useEffect } from 'react'
import { X } from 'lucide-react'
import { getFileType, getExt } from '../../../shared/utils/index'
import { ImagePreview } from './ImagePreview'
import { AudioPreview } from './AudioPreview'
import { PdfPreview } from './PdfPreview'
import { TextPreview } from './TextPreview'
import { DocxPreview } from './DocxPreview'
import { SheetPreview } from './SheetPreview'
import { KreuzbergPreview } from './KreuzbergPreview'
import { BookReader } from './BookReader'

export function PreviewModal({ file, onClose, onDownload, dark }) {
  const displayName = file.filename || file.name || ''
  const fileType = getFileType(displayName, file.kind)
  const ext = getExt(displayName)
  
  const isImage = fileType.group === 'image'
  const isAudio = fileType.group === 'audio'
  const isPdf = ext === 'pdf'
  const isDocx = ext === 'docx'
  const isSheet = ['xlsx', 'xls', 'csv', 'tsv', 'ods', 'odp'].includes(ext)
  const isBook = ['epub', 'mobi'].includes(ext)
  const isBinaryDoc = ['doc', 'ppt', 'pptx', 'odt', 'rtf'].includes(ext)
  const isTextOrCode = fileType.group === 'doc'

  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', handleKey)
    return () => window.removeEventListener('keydown', handleKey)
  }, [onClose])

  // Full-screen book reader (no modal wrapper)
  if (isBook) {
    return (
      <div className="fixed inset-0 z-[150] bg-white dark:bg-slate-950">
        <BookReader file={file} onClose={onClose} onDownload={onDownload} />
      </div>
    )
  }

  const renderContent = () => {
    if (isImage) {
      return (
        <ImagePreview 
          key="image-preview" 
          file={file} 
          onClose={onClose} 
          onDownload={onDownload} 
        />
      )
    }

    if (isAudio) {
      return (
        <AudioPreview 
          key="audio-preview" 
          file={file} 
          onClose={onClose} 
          onDownload={onDownload} 
        />
      )
    }

    if (isPdf) {
      return (
        <PdfPreview 
          key="pdf-preview" 
          file={file} 
          onClose={onClose} 
          onDownload={onDownload} 
        />
      )
    }

    if (isDocx) {
      return (
        <DocxPreview
          key="docx-preview"
          file={file}
          onClose={onClose}
          onDownload={onDownload}
        />
      )
    }

    if (isSheet) {
      return (
        <SheetPreview
          key="sheet-preview"
          file={file}
          onClose={onClose}
          onDownload={onDownload}
        />
      )
    }

    if (isBinaryDoc) {
      return (
        <KreuzbergPreview 
          key="kreuzberg-preview" 
          file={file} 
          onClose={onClose} 
          onDownload={onDownload} 
          dark={dark} 
        />
      )
    }

    if (isTextOrCode) {
      return (
        <TextPreview 
          key="text-preview" 
          file={file} 
          onClose={onClose} 
          onDownload={onDownload} 
        />
      )
    }

    return null
  }

  return (
    <div
      className="fixed inset-0 z-[150] bg-black/50 backdrop-blur-sm"
      onClick={(e) => { if (e.target === e.currentTarget) onClose() }}
    >
      <button
        type="button"
        onClick={onClose}
        className="absolute top-4 right-4 z-10 p-2 rounded-full bg-white/80 dark:bg-slate-800/80 hover:bg-white dark:hover:bg-slate-700 text-slate-600 dark:text-slate-300 transition-colors shadow-lg"
      >
        <X size={20} />
      </button>
      {renderContent()}
    </div>
  )
}
