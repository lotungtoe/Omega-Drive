import { AlertCircle, Download } from 'lucide-react'
import { useTranslation } from 'react-i18next'

export function PreviewErrorState({ error, file, onDownload }) {
  const { t } = useTranslation()
  return (
    <div className="absolute inset-0 flex flex-col items-center justify-center p-6 text-center">
      <div className="w-16 h-16 rounded-full bg-red-100 dark:bg-red-900/30 flex items-center justify-center mb-4">
        <AlertCircle className="w-8 h-8 text-red-500" />
      </div>
      <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-2">
        {t('preview.errorTitle', 'Cannot preview')}
      </h3>
      <p className="text-slate-500 max-w-sm mb-6">
        {error}
      </p>
      <button type="button"
        onClick={() => onDownload(file)}
        className="flex items-center gap-2 px-6 py-2.5 bg-indigo-500 hover:bg-indigo-600 text-white rounded-xl font-medium transition-colors"
      >
        <Download className="w-4 h-4" />
        {t('common.download', 'Download file')}
      </button>
    </div>
  )
}
