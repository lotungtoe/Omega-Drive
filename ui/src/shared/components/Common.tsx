import {
  HardDrive, Upload, Trash2
} from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '../../components/ui/be-ui-button'

export function EmptyState({ onUpload, isDragOver, isTrash }) {
  const { t } = useTranslation()

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        padding: '80px 24px',
        textAlign: 'center',
        borderRadius: 'var(--gd-radius-md)',
        border: isDragOver ? '2px dashed var(--gd-blue)' : '2px dashed var(--gd-outline)',
        backgroundColor: isDragOver ? 'var(--gd-blue-surface)' : 'transparent',
        transition: 'border-color 0.2s ease, background-color 0.2s ease',
      }}
    >
      <div style={{
        width: 64,
        height: 64,
        borderRadius: 'var(--gd-radius-full)',
        backgroundColor: 'var(--gd-surface-variant)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        marginBottom: 16,
      }}>
        {isDragOver
          ? <Upload size={28} style={{ color: 'var(--gd-blue)' }} />
          : isTrash 
            ? <Trash2 size={28} style={{ color: 'var(--gd-on-surface-variant)' }} />
            : <HardDrive size={28} style={{ color: 'var(--gd-on-surface-variant)' }} />
        }
      </div>
      <h3 style={{ 
        fontSize: 16, 
        fontFamily: "'Google Sans', sans-serif",
        fontWeight: 500, 
        marginBottom: 4,
        color: 'var(--gd-on-surface)'
      }}>
        {isDragOver ? t('drive.dragTitle') : isTrash ? 'Trash is empty' : t('drive.emptyTitle')}
      </h3>
      <p style={{ 
        fontSize: 14, 
        color: 'var(--gd-on-surface-variant)',
        marginBottom: isDragOver ? 0 : 20,
      }}>
        {!isTrash && (isDragOver ? t('drive.dragHint') : t('drive.emptyHint'))}
      </p>
      {!isDragOver && !isTrash && (
        <Button variant="primary" size="md" onClick={onUpload}>
          <Upload size={16} /> {t('drive.uploadCta')}
        </Button>
      )}
    </div>
  )
}
