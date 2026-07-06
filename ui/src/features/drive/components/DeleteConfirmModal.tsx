import { motion, AnimatePresence } from 'framer-motion'
import { useTranslation } from 'react-i18next'
import { cn } from '../../../shared/utils/index'
import { AlertTriangle, Trash2 } from 'lucide-react'
import { Button } from '../../../components/ui/be-ui-button'

export function DeleteConfirmModal({ isOpen, onClose, onConfirm, item }) {
  const { t } = useTranslation()

  if (!isOpen || !item) return null

  const isFolder = item.isFolder
  const isPermanent = item.status === 'trashed'
  const itemName = item.name || item.filename || ''
  
  // Determine text based on item type and status
  let titleKey = 'modal.deleteFile.title'
  let warningText = t('modal.deleteFile.warning', { name: itemName, defaultValue: `Are you sure you want to move "${itemName}" to trash?` })
  
  if (isPermanent) {
    titleKey = 'modal.deleteForever.title'
    warningText = t('modal.deleteForever.warning', { name: itemName, defaultValue: `Are you sure you want to PERMANENTLY delete "${itemName}"? This action cannot be undone.` })
  } else if (isFolder) {
    titleKey = 'modal.deleteFolder.title'
    warningText = t('modal.deleteFolder.warning', { name: itemName, defaultValue: `Are you sure you want to delete the folder "${itemName}" and all its contents?` })
  }

  let iconBgClass = "bg-[var(--gd-outline-variant)] text-[var(--gd-on-surface-variant)]"
  if (isPermanent) {
    iconBgClass = "bg-red-500/10 text-red-400"
  }

  return (
    <AnimatePresence>
      {isOpen && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          onClick={onClose}
          className="fixed inset-0 z-[150] flex items-center justify-center bg-black/50 backdrop-blur-md"
        >
          <motion.div
            initial={{ scale: 0.95, y: 10, opacity: 0 }}
            animate={{ scale: 1, y: 0, opacity: 1 }}
            exit={{ scale: 0.95, y: 10, opacity: 0 }}
            transition={{ type: "spring", duration: 0.4, bounce: 0.15 }}
            onClick={e => e.stopPropagation()}
            className="w-[420px] rounded-3xl border shadow-2xl p-6 bg-[var(--gd-modal-surface)] border-[var(--gd-modal-border)] text-[var(--gd-modal-text)]"
          >
            <div className="flex items-start gap-4 mb-2">
              <div className={cn("p-3 rounded-2xl flex-shrink-0 mt-1", iconBgClass)}>
                {isPermanent ? <AlertTriangle size={24} /> : <Trash2 size={24} />}
              </div>
              <div className="flex-1 mt-1.5">
                <h2 className="font-semibold text-xl leading-none mb-3">{t(titleKey)}</h2>
                <p className="text-[15px] leading-relaxed text-[var(--gd-on-surface-variant)]">
                  {warningText}
                </p>
              </div>
            </div>

            <div className="flex gap-3 justify-end mt-8">
              <Button variant="ghost" size="md" onClick={onClose}>
                {t('common.cancel')}
              </Button>
              <Button variant="primary" size="md" onClick={() => { onConfirm(item); onClose() }}>
                {isPermanent ? t('drive.deleteForever') : t('common.delete')}
              </Button>
            </div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  )
}
