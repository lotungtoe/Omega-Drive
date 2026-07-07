import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { TextPreview } from './TextPreview'
import { getExt } from '../../../shared/utils/index'

const SUPPORTED = new Set([
  'pdf', 'docx', 'pptx',
  'html', 'xml',
  'eml', 'msg', 'pst',
  'zip', 'tar', '7z', 'gz',
])

export function KreuzbergPreview({ file, onClose, onDownload, dark }) {
  const displayName = file.filename || file.name || ''
  const ext = getExt(displayName)

  if (!SUPPORTED.has(ext)) return null

  const [content, setContent] = useState(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    const load = async () => {
      try {
        setLoading(true)
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
  return null
}
