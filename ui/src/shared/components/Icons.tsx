import { 
  ImageIcon, Film, Music, Archive, Code, Table, FileText, File 
} from 'lucide-react'
import { getFileType, getColor } from '../utils/index'

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
