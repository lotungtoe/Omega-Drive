import { useState, useEffect, useCallback } from 'react'
import { Download, Loader2, AlertCircle, Table } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { invoke } from '@tauri-apps/api/core'
import { getColor, formatSize, getExt } from '../../../shared/utils'

const getFileLabel = (ext) => {
  if (ext === 'csv') return 'CSV'
  if (ext === 'xls') return 'Excel 97-2003'
  return 'Excel'
}

export function SheetPreview({ file, onClose: _onClose, onDownload }) {
  const { t } = useTranslation()
  const displayName = file.filename || file.name || ''
  const color = getColor(displayName, file.kind)
  const ext = getExt(displayName)

  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(null)
  const [sheetNames, setSheetNames] = useState([])
  const [activeSheet, setActiveSheet] = useState(0)
  const [rows, setRows] = useState([])

  const renderSheet = useCallback((workbook, sheetIndex) => {
    const XLSX = workbook.__XLSX
    const sheetName = workbook.SheetNames[sheetIndex]
    const sheet = workbook.Sheets[sheetName]
    const data = XLSX.utils.sheet_to_json(sheet, { header: 1, defval: '' })
    setRows(data)
    setActiveSheet(sheetIndex)
  }, [])

  useEffect(() => {
    let cancelled = false

    const loadSheet = async () => {
      try {
        setLoading(true)
        setError(null)

        const binaryData = await invoke('retrieve_full_file', { fileId: file.id })
        if (cancelled) return

        // Dynamic import for code splitting
        const XLSX = await import('xlsx')
        if (cancelled) return

        const uint8 = new Uint8Array(binaryData)
        const workbook = XLSX.read(uint8, { type: 'array' })
        workbook.__XLSX = XLSX // attach for re-use on sheet switch

        if (cancelled) return

        setSheetNames(workbook.SheetNames)
        renderSheet(workbook, 0)

        // Store workbook reference for sheet switching
        globalThis.__omegaWorkbook = workbook
      } catch (err) {
        console.error('Failed to load sheet preview:', err)
        if (!cancelled) setError(err.toString())
      } finally {
        if (!cancelled) setLoading(false)
      }
    }

    loadSheet()
    return () => {
      cancelled = true
      globalThis.__omegaWorkbook = null
    }
  }, [file.id, renderSheet])

  const switchSheet = (index) => {
    if (globalThis.__omegaWorkbook) {
      renderSheet(globalThis.__omegaWorkbook, index)
    }
  }

  const headerRow = rows[0] || []
  const dataRows = rows.slice(1)
  const fileLabel = getFileLabel(ext)

  return (
    <div className="flex flex-col h-full w-full bg-slate-50 dark:bg-slate-900 overflow-hidden relative">
      {/* Header */}
      <div className="h-14 flex items-center justify-between px-4 border-b border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 shrink-0 z-10">
        <div className="flex items-center gap-3 overflow-hidden">
          <div className="p-2 rounded-lg bg-emerald-50 dark:bg-emerald-900/30 shrink-0">
            <Table className="w-5 h-5" style={{ color }} />
          </div>
          <div className="flex flex-col min-w-0">
            <span className="font-medium text-sm text-slate-900 dark:text-slate-100 truncate">
              {displayName}
            </span>
            <span className="text-xs text-slate-500">
              {formatSize(file.size)} &bull; {fileLabel} {rows.length > 0 ? `\u2022 ${rows.length} hĂ ng` : ''}
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

      {/* Sheet Tabs (only for multi-sheet workbooks) */}
      {sheetNames.length > 1 && (
        <div className="flex items-center gap-1 px-3 py-1.5 border-b border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-950 overflow-x-auto shrink-0">
          {sheetNames.map((name, i) => (
            <button type="button"
              key={name}
              onClick={() => switchSheet(i)}
              className={`px-3 py-1 rounded text-xs font-medium whitespace-nowrap transition-colors ${
                i === activeSheet
                  ? 'bg-emerald-100 dark:bg-emerald-900/40 text-emerald-700 dark:text-emerald-300'
                  : 'text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800'
              }`}
            >
              {name}
            </button>
          ))}
        </div>
      )}

      {/* Content */}
      <div className="flex-1 overflow-auto relative">
        {loading && (
          <div className="absolute inset-0 flex items-center justify-center bg-slate-50/60 dark:bg-slate-900/60 backdrop-blur-sm z-10">
            <Loader2 className="w-8 h-8 animate-spin text-emerald-500" />
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
              className="flex items-center gap-2 px-6 py-2.5 bg-emerald-500 hover:bg-emerald-600 text-white rounded-xl font-medium transition-colors"
            >
              <Download className="w-4 h-4" />
              {t('common.download', 'Táº£i xuá»‘ng tá»‡p')}
            </button>
          </div>
        ) : (
          rows.length > 0 && (
            <div className="p-4">
              <div className="overflow-auto rounded-xl border border-slate-200 dark:border-slate-700 shadow-sm">
                <table className="w-full text-sm border-collapse">
                  <thead>
                    <tr className="bg-emerald-50 dark:bg-emerald-900/20 sticky top-0 z-10">
                      <th className="px-3 py-2 text-left font-semibold text-slate-400 dark:text-slate-500 border-b border-r border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-900 text-xs w-10 sticky left-0">
                        #
                      </th>
                      {headerRow.map((cell, ci) => {
                        const colLabel = cell || String.fromCodePoint(65 + ci);
                        return (
                          <th
                            key={`h-${ci}-${colLabel}`}
                            className="px-3 py-2 text-left font-semibold text-slate-700 dark:text-slate-200 border-b border-r border-slate-200 dark:border-slate-700 whitespace-nowrap"
                          >
                            {colLabel}
                          </th>
                        )
                      })}
                    </tr>
                  </thead>
                  <tbody>
                    {dataRows.map((row, ri) => (
                      <tr
                        key={`r-${ri}-${row[0] ?? ri}`}
                        className="hover:bg-slate-50 dark:hover:bg-slate-800/50 transition-colors"
                      >
                        <td className="px-3 py-1.5 text-xs text-slate-400 dark:text-slate-500 border-b border-r border-slate-200 dark:border-slate-700 bg-slate-50/50 dark:bg-slate-900/50 sticky left-0 font-mono">
                          {ri + 2}
                        </td>
                        {headerRow.map((_, ci) => {
                          const cellVal = row[ci]
                          const cellStr = cellVal !== undefined && cellVal !== '' ? String(cellVal) : ''
                          return (
                            <td
                              key={`c-${ri}-${ci}-${cellStr}`}
                              className="px-3 py-1.5 text-slate-700 dark:text-slate-300 border-b border-r border-slate-200 dark:border-slate-700 whitespace-nowrap max-w-xs truncate"
                              title={cellVal ?? ''}
                            >
                              {cellStr}
                            </td>
                          )
                        })}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <p className="text-xs text-slate-400 mt-2 text-right">
                {rows.length} hĂ ng &times; {headerRow.length} cá»™t
              </p>
            </div>
          )
        )}
      </div>
    </div>
  )
}
