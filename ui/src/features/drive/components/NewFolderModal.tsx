import { useState, useRef, useEffect } from 'react'
import { motion } from 'framer-motion'
import { useTranslation } from 'react-i18next'
import { Button } from '../../../components/ui/be-ui-button'

export function NewFolderModal({ onClose, onCreate }) {
  const { t } = useTranslation()
  const [name, setName] = useState('')
  const inputRef = useRef()
  useEffect(() => inputRef.current?.focus(), [])

  const submit = () => {
    if (name.trim()) { onCreate(name.trim()); onClose() }
  }

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      onClick={onClose}
      className="fixed inset-0 z-[150] flex items-center justify-center bg-black/50 backdrop-blur-sm"
    >
      <motion.div
        initial={{ scale: 0.95, y: 10 }}
        animate={{ scale: 1, y: 0 }}
        exit={{ scale: 0.95, y: 10 }}
        onClick={e => e.stopPropagation()}
        className="w-[360px] rounded-2xl border shadow-2xl p-6 bg-[var(--gd-modal-surface)] border-[var(--gd-modal-border)] text-[var(--gd-modal-text)]"
      >
        <h2 className="font-semibold text-base mb-4">{t('modal.newFolder.title')}</h2>
        <input
          ref={inputRef}
          value={name}
          onChange={e => setName(e.target.value)}
          onKeyDown={e => {
            if (e.key === 'Enter') submit()
            if (e.key === 'Escape') onClose()
          }}
          placeholder={t('modal.newFolder.placeholder')}
          className="w-full px-4 py-2.5 rounded-xl border text-sm focus:outline-none focus:ring-2 transition-all bg-[var(--gd-input-bg)] border-[var(--gd-input-border)] text-[var(--gd-modal-text)] placeholder-[var(--gd-modal-text-secondary)] focus:ring-blue-500/40"
        />
        <div className="flex gap-2 mt-4">
          <Button variant="ghost" size="md" onClick={onClose} className="flex-1">
            {t('common.cancel')}
          </Button>
          <Button variant="primary" size="md" onClick={submit} disabled={!name.trim()} className="flex-1">
            {t('common.create')}
          </Button>
        </div>
      </motion.div>
    </motion.div>
  )
}
