import { AnimatePresence } from 'framer-motion'
import { getFileType, getExt } from '../../../shared/utils/index'
import { ImagePreview } from './ImagePreview'
import { AudioPreview } from './AudioPreview'
import { FileDetailsPreview } from './FileDetailsPreview'
import { PdfPreview } from './PdfPreview'
import { TextPreview } from './TextPreview'
import { DocxPreview } from './DocxPreview'
import { SheetPreview } from './SheetPreview'
import { KreuzbergPreview } from './KreuzbergPreview'

export function PreviewModal({ file, onClose, onDownload, dark }) {
  const displayName = file.filename || file.name || ''
  const fileType = getFileType(displayName, file.kind)
  const ext = getExt(displayName)
  
  const isImage = fileType.group === 'image'
  const isAudio = fileType.group === 'audio'
  const isPdf = ext === 'pdf'
  const isDocx = ext === 'docx'
  const isSheet = ['xlsx', 'xls', 'csv', 'tsv', 'ods', 'odp'].includes(ext)
  const isBinaryDoc = ['doc', 'ppt', 'pptx', 'odt', 'rtf', 'epub', 'mobi'].includes(ext)
  const isTextOrCode = fileType.group === 'doc'

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

  return (
    <AnimatePresence mode="wait">
      {renderContent()}
    </AnimatePresence>
  )
}
