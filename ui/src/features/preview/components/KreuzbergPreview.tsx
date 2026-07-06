import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { TextPreview } from './TextPreview'
import { FileDetailsPreview } from './FileDetailsPreview'

export function KreuzbergPreview({ file, onClose, onDownload, dark }) {
  const [content, setContent] = useState(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    const load = async () => {
      try {
        setLoading(true)
        const displayName = file.filename || file.name || ''
        const result: any = await invoke('extract_file_text', {
          fileId: file.id,
          filename: displayName,
        })
        setContent(result.content as string)
      } catch {
        setContent(null)
      } finally {
        setLoading(false)
      }
    }
    load()
  }, [file.id, file.filename, file.name])

  if (loading) return null
  if (content != null) {
    return (
      <TextPreview
        key="kreuzberg-text"
        file={file}
        onClose={onClose}
        onDownload={onDownload}
        preloadedContent={content}
      />
    )
  }
  return <FileDetailsPreview file={file} onClose={onClose} onDownload={onDownload} dark={dark} />
}
