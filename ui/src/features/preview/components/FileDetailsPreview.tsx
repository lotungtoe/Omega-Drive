import { X, Download } from 'lucide-react'
import { motion } from 'framer-motion'
import { useTranslation } from 'react-i18next'
import { FileIcon } from '../../../shared/components/Icons'
import { getFileType, getColor, formatSize, formatDateSafe, cn } from '../../../shared/utils'
import { Button } from '../../../components/ui/be-ui-button'

export function FileDetailsPreview({ file, onClose, onDownload, dark }) {
  const { t } = useTranslation()
  const displayName = file.filename || file.name || ''
  const fileType = getFileType(displayName, file.kind)
  const label = t(fileType.labelKey, { ext: (fileType.ext || '').toUpperCase() })
  const color = getColor(displayName, file.kind)

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      onClick={onClose}
      className="fixed inset-0 z-[100] flex items-center justify-center bg-black/70 backdrop-blur-md p-6"
    >
      <motion.div
        initial={{ scale: 0.96, y: 12 }}
        animate={{ scale: 1, y: 0 }}
        exit={{ scale: 0.96, y: 12 }}
        transition={{ duration: 0.2 }}
        onClick={e => e.stopPropagation()}
        className={cn(
          "relative w-full rounded-2xl border shadow-2xl overflow-hidden",
          "max-w-lg",
          dark ? "bg-[#16161f] border-white/10" : "bg-white border-gray-200"
        )}
      >
      <div className="h-1 w-full" style={{ background: `linear-gradient(90deg, ${color}, transparent)` }} />
      <Button variant="ghost" size="icon" onClick={onClose} className="absolute top-3 right-3 z-20">
        <X size={16} />
      </Button>

      <div className="flex flex-col items-center py-10 px-8">
        <div className="w-20 h-20 rounded-2xl flex items-center justify-center mb-5" style={{ background: `${color}18` }}>
          <FileIcon filename={displayName} kind={file.kind} size={32} />
        </div>
        <h3 className={cn("text-base font-semibold text-center mb-1", dark ? "text-white" : "text-gray-800")}>
          {displayName}
        </h3>
        <p className={cn("text-sm mb-6", dark ? "text-slate-500" : "text-gray-400")}>
          {label} Â· {formatSize(file.size)}
        </p>

        <div className={cn("w-full rounded-xl border divide-y text-sm", dark ? "border-white/[0.07] divide-white/[0.05]" : "border-gray-100 divide-gray-100")}>
          {[
            [t('modal.preview.fileName'), displayName],
            [t('modal.preview.type'), label],
            [t('modal.preview.size'), formatSize(file.size)],
            [t('modal.preview.dateAdded'), formatDateSafe(file.created_at, 'vi-VN', { day: '2-digit', month: '2-digit', year: 'numeric', hour: '2-digit', minute: '2-digit' })],
          ].map(([k, v]) => (
            <div key={k} className="flex items-center justify-between px-4 py-2.5">
              <span className={dark ? "text-slate-500" : "text-gray-400"}>{k}</span>
              <span className={cn("font-medium max-w-[200px] truncate text-right", dark ? "text-slate-200" : "text-gray-700")}>{v}</span>
            </div>
          ))}
        </div>

        <Button variant="primary" size="md" onClick={onDownload} className="mt-4 w-full">
          <Download size={14} /> {t('drive.download')}
        </Button>
      </div>
      </motion.div>
    </motion.div>
  )
}
