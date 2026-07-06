import { useState, useEffect } from 'react'
import { Download, Loader2, AlertCircle, FileText } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { invoke } from '@tauri-apps/api/core'
import { getColor, formatSize, getExt } from '../../../shared/utils'
import { PrismLight as SyntaxHighlighter } from 'react-syntax-highlighter'
import { vscDarkPlus, vs } from 'react-syntax-highlighter/dist/esm/styles/prism'

// Map extensions to Prism languages
const getLanguage = (ext) => {
  const map = {
    js: 'javascript', jsx: 'jsx', ts: 'typescript', tsx: 'tsx',
    py: 'python', rs: 'rust', go: 'go', java: 'java',
    c: 'c', cpp: 'cpp', cs: 'csharp', php: 'php',
    html: 'html', css: 'css', scss: 'scss', less: 'less',
    json: 'json', xml: 'xml', yaml: 'yaml', yml: 'yaml',
    md: 'markdown', sql: 'sql', sh: 'bash', bash: 'bash',
    csv: 'csv', txt: 'text', log: 'text'
  }
  return map[ext] || 'text'
}

export function TextPreview({ file, onClose: _onClose, onDownload }) {
  const { t } = useTranslation()
  const displayName = file.filename || file.name || ''
  const color = getColor(displayName, file.kind)
  const ext = getExt(displayName)
  const language = getLanguage(ext)
  
  const [content, setContent] = useState('')
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(null)
  
  // Try to detect dark mode for syntax highlighting theme
  const isDarkMode = document.documentElement.classList.contains('dark')

  useEffect(() => {
    const loadText = async () => {
      try {
        setLoading(true)
        setError(null)
        
        // Check size limit (e.g. 1MB = 1048576 bytes)
        if (file.size > 1048576) {
          throw new Error('File is too large to preview (> 1MB)')
        }

        const binaryData = await invoke('retrieve_full_file', { fileId: file.id })
        
        const decoder = new TextDecoder('utf-8')
        const text = decoder.decode(new Uint8Array(binaryData))
        setContent(text)
      } catch (err) {
        console.error("Failed to load text preview:", err)
        setError(err.toString())
      } finally {
        setLoading(false)
      }
    }

    loadText()
  }, [file.id, file.size])

  return (
    <div className="flex flex-col h-full w-full bg-slate-50 dark:bg-slate-900 overflow-hidden relative">
      {/* Header */}
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
              {formatSize(file.size)} &bull; {language}
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

      {/* Content Area */}
      <div className="flex-1 overflow-auto relative">
        {loading && (
          <div className="absolute inset-0 flex items-center justify-center bg-slate-50/50 dark:bg-slate-900/50 backdrop-blur-sm z-10">
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
          <div className="min-h-full p-4">
            <SyntaxHighlighter
              language={language}
              style={isDarkMode ? vscDarkPlus : vs}
              customStyle={{
                margin: 0,
                borderRadius: '0.5rem',
                fontSize: '0.875rem',
                lineHeight: '1.5',
                backgroundColor: isDarkMode ? '#1e1e1e' : '#ffffff',
                border: isDarkMode ? '1px solid #334155' : '1px solid #e2e8f0',
              }}
              showLineNumbers={true}
              wrapLines={true}
              wrapLongLines={true}
            >
              {content}
            </SyntaxHighlighter>
          </div>
        )}
      </div>
    </div>
  )
}
