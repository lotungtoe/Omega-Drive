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
  const supported = SUPPORTED.has(ext)

  const [content, setContent] = useState(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    if (!supported) return
    let cancelled = false
    const load = async () => {
      try {
        setLoading(true)
        const result: any = await invoke('extract_file_text', {
          fileId: file.id,
          filename: displayName,
        })
        if (cancelled) return
        setContent(result.content as string)
      } catch {
        if (!cancelled) setContent(null)
      } finally {
        if (!cancelled) setLoading(false)
      }
    }
    load()
    return () => {
      cancelled = true
    }
  }, [file.id, file.filename, file.name, supported, displayName])

  if (!supported) return null
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
