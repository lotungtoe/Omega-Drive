import { useState, memo } from 'react'
import { Upload, Download, X, Minus, Plus, FileVideo, FileAudio, FileImage, FileText, File, Archive, FileCode, ChevronUp, ChevronDown } from 'lucide-react'
import { useTranslation } from 'react-i18next'

const getExt = (filename) => {
  if (!filename) return ''
  const dot = filename.lastIndexOf('.')
  return dot >= 0 ? filename.slice(dot + 1).toLowerCase() : ''
}

const getFileIcon = (filename) => {
  const ext = getExt(filename)
  if (['mp4', 'mkv', 'avi', 'mov', 'webm'].includes(ext)) return <FileVideo size={20} color="var(--gd-video)" />
  if (['mp3', 'flac', 'wav', 'aac'].includes(ext)) return <FileAudio size={20} color="var(--gd-audio)" />
  if (['jpg', 'jpeg', 'png', 'gif', 'webp'].includes(ext)) return <FileImage size={20} color="var(--gd-image)" />
  if (['pdf', 'doc', 'docx', 'xls', 'xlsx'].includes(ext)) return <FileText size={20} color="var(--gd-document)" />
  if (['zip', 'rar', '7z', 'tar', 'gz'].includes(ext)) return <Archive size={20} color="var(--gd-archive)" />
  if (['js', 'ts', 'tsx', 'jsx', 'py', 'rs'].includes(ext)) return <FileCode size={20} color="var(--gd-code)" />
  return <File size={20} color="var(--gd-on-surface-variant)" />
}

const CircularProgress = ({ value }) => {
  const radius = 10
  const circumference = 2 * Math.PI * radius
  const strokeDashoffset = circumference - (Math.min(Math.max(value, 0), 100) / 100) * circumference
  
  return (
    <div style={{ position: 'relative', width: 24, height: 24 }}>
      <svg width="24" height="24" viewBox="0 0 24 24" style={{ transform: 'rotate(-90deg)' }}>
        <circle cx="12" cy="12" r={radius} stroke="var(--gd-outline-variant)" strokeWidth="2.5" fill="none" />
        <circle 
          cx="12" cy="12" r={radius} 
          stroke="var(--gd-blue)" strokeWidth="2.5" fill="none" 
          strokeDasharray={circumference} 
          strokeDashoffset={strokeDashoffset} 
          strokeLinecap="round" 
          style={{ transition: 'stroke-dashoffset 0.3s ease' }} 
        />
      </svg>
    </div>
  )
}

const SessionItem = ({ s, onCancel }) => {
  const [hovered, setHovered] = useState(false)
  const progress = s.overallProgress || s.percentage || 0
  const displayName = s.fileName || s.detail || "Processing..."
  const isDone = s.phase === 'done'
  
  return (
    <div
      style={{ display: 'flex', alignItems: 'center', padding: '12px 16px', gap: 12 }}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      <div style={{ flexShrink: 0, display: 'flex' }}>
        {getFileIcon(displayName)}
      </div>
      <div style={{ flex: 1, minWidth: 0, fontSize: 13, color: 'var(--gd-on-surface)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>
        {displayName}
      </div>
      <div style={{ flexShrink: 0, display: 'flex', alignItems: 'center' }}>
        {hovered && !isDone ? (
          <button type="button"
            onClick={() => onCancel?.(s.sessionId, s)}
            title="Cancel"
            style={{
              width: 24, height: 24,
              borderRadius: '50%',
              background: 'var(--gd-outline-variant)',
              border: 'none',
              cursor: 'pointer',
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              color: 'var(--gd-on-surface-variant)',
              padding: 0,
              transition: 'background 0.15s',
            }}
            onMouseEnter={e => (e.currentTarget.style.background = '#e53935')}
            onMouseLeave={e => (e.currentTarget.style.background = 'var(--gd-outline-variant)')}
          >
            <X size={14} color="currentColor" />
          </button>
        ) : (
          <CircularProgress value={progress} />
        )}
      </div>
    </div>
  )
}

export const ProgressOverlay = memo(function ProgressOverlay({ progressMap, onClose, removeSession }) {
  const { t } = useTranslation()
  const [isMinimized, setIsMinimized] = useState(false)
  const sessions = Object.values(progressMap).sort((a, b) => b.lastUpdate - a.lastUpdate)
  if (sessions.length === 0) return null

  return (
    <div
      style={{
        position: 'fixed',
        right: 24,
        bottom: 24,
        width: 320,
        backgroundColor: 'var(--gd-surface)',
        borderRadius: '8px',
        boxShadow: '0 4px 12px rgba(0,0,0,0.15)',
        border: '1px solid var(--gd-outline)',
        overflow: 'hidden',
        display: 'flex',
        flexDirection: 'column',
        zIndex: 50,
      }}
    >
      <div 
        style={{ 
          display: 'flex', 
          alignItems: 'center', 
          justifyContent: 'space-between',
          padding: '12px 16px',
          cursor: 'pointer',
          userSelect: 'none'
        }}
        onClick={() => setIsMinimized(!isMinimized)}
      >
        <h3 style={{ fontSize: 14, fontWeight: 500, margin: 0, color: 'var(--gd-on-surface)' }}>
          {t('progress.uploadingNItems', { count: sessions.length, defaultValue: `Uploading ${sessions.length} item(s)` })}
        </h3>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <button type="button" 
            style={{ background: 'none', border: 'none', padding: 4, cursor: 'pointer', color: 'var(--gd-on-surface-variant)', display: 'flex' }}
          >
            {isMinimized ? <ChevronUp size={20} /> : <ChevronDown size={20} />}
          </button>
          <button type="button" 
            onClick={(e) => { e.stopPropagation(); onClose(); }} 
            style={{ background: 'none', border: 'none', padding: 4, cursor: 'pointer', color: 'var(--gd-on-surface-variant)', display: 'flex' }}
          >
            <X size={20} />
          </button>
        </div>
      </div>

      {!isMinimized && (
        <div style={{ maxHeight: 300, overflowY: 'auto' }}>
          {sessions.map(s => <SessionItem key={s.sessionId} s={s} onCancel={removeSession} />)}
        </div>
      )}
    </div>
  )
})
