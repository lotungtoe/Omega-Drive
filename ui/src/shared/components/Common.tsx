import { 
  HardDrive, Upload, ArrowUp, ArrowDown, ArrowUpDown,
  ImageIcon, Film, Music, Archive, Code, Table, FileText, File, Trash2
} from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { getFileType, getColor } from '../utils/index'
import { Button } from '../../components/ui/be-ui-button'

export function FileIcon({ filename, kind, size = 20 }) {
  const { group } = getFileType(filename, kind)
  const color = getColor(filename, kind)

  const iconMap = {
    image: ImageIcon, video: Film, audio: Music, archive: Archive,
    code: Code, sheet: Table, doc: FileText, other: File
  }
  const IconComp = iconMap[group] || File

  return (
    <div className="relative flex items-center justify-center">
      <svg width={size * 1.4} height={size * 1.6} viewBox="0 0 28 32" fill="none">
        <path d="M4 2h14l8 8v20a2 2 0 01-2 2H4a2 2 0 01-2-2V4a2 2 0 012-2z" fill={`${color}18`} stroke={`${color}60`} strokeWidth="1.5"/>
        <path d="M18 2l8 8h-6a2 2 0 01-2-2V2z" fill={`${color}40`}/>
      </svg>
      <div className="absolute" style={{ color }}>
        <IconComp size={size * 0.55} strokeWidth={2} />
      </div>
    </div>
  )
}

export function FolderIcon({ size = 24, color = '#f59e0b' }) {
  return (
    <svg width={size} height={size * 0.85} viewBox="0 0 24 20" fill="none">
      <path d="M0 3a2 2 0 012-2h6.5l2 2.5H22a2 2 0 012 2V18a2 2 0 01-2 2H2a2 2 0 01-2-2V3z"
        fill={color} fillOpacity="0.9" />
      <path d="M0 7h24v11a2 2 0 01-2 2H2a2 2 0 01-2-2V7z"
        fill={color} fillOpacity="0.7"/>
    </svg>
  )
}

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
        transition: 'all 0.2s ease',
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

export function SortButton({ label, field, sort, setSort }) {
  const isActive = sort.field === field
  const toggle = () => setSort(s => s.field === field ? { field, dir: s.dir === 'asc' ? 'desc' : 'asc' } : { field, dir: 'asc' })
  return (
    <Button variant="ghost" size="sm" onClick={toggle} style={{
      fontSize: 12, color: isActive ? 'var(--gd-blue)' : 'var(--gd-on-surface-variant)',
      padding: '4px 0', height: 'auto', borderRadius: 0,
    }}>
      {label}
      {isActive && sort.dir === 'asc' && <ArrowUp size={14} />}
      {isActive && sort.dir !== 'asc' && <ArrowDown size={14} />}
      {!isActive && <ArrowUpDown size={14} style={{ opacity: 0.4 }} />}
    </Button>
  )
}
