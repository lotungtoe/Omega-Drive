import { memo } from 'react';
import {
  FileVideo, FileAudio, FileImage, FileText, File,
  Archive, FileCode,
} from 'lucide-react';

/* ─── File icon ─────────────────────────────────────────────── */

const KIND_MAP: Record<string, { Icon: React.ElementType; color: string; bg: string }> = {
  video:   { Icon: FileVideo,  color: '#fff', bg: '#8b5cf6' },
  audio:   { Icon: FileAudio,  color: '#fff', bg: '#ec4899' },
  image:   { Icon: FileImage,  color: '#fff', bg: '#0ea5e9' },
  pdf:     { Icon: FileText,   color: '#fff', bg: '#ef4444' },
  word:    { Icon: FileText,   color: '#fff', bg: '#2563eb' },
  excel:   { Icon: FileText,   color: '#fff', bg: '#16a34a' },
  code:    { Icon: FileCode,   color: '#fff', bg: '#f59e0b' },
  archive: { Icon: Archive,    color: '#fff', bg: '#d97706' },
};

const EXT_KIND: Record<string, string> = {
  mp4: 'video', mkv: 'video', avi: 'video', mov: 'video', webm: 'video',
  mp3: 'audio', flac: 'audio', wav: 'audio', aac: 'audio',
  jpg: 'image', jpeg: 'image', png: 'image', gif: 'image', webp: 'image',
  pdf: 'pdf',
  doc: 'word', docx: 'word',
  xls: 'excel', xlsx: 'excel',
  js: 'doc', ts: 'doc', tsx: 'doc', jsx: 'doc', py: 'doc', rs: 'doc',
  zip: 'archive', rar: 'archive', '7z': 'archive', tar: 'archive', gz: 'archive',
};

function getExt(filename: string) {
  const dot = filename.lastIndexOf('.');
  return dot >= 0 ? filename.slice(dot + 1).toLowerCase() : '';
}

export function FileIcon({ filename, kind }: { filename: string; kind?: string | null }) {
  const ext = getExt(filename);
  const resolvedKind = kind || EXT_KIND[ext] || 'file';
  const meta = KIND_MAP[resolvedKind] ?? { Icon: File, color: '#fff', bg: 'var(--gd-outline)' };
  const { Icon, color, bg } = meta;
  const label = ext ? ext.toUpperCase().slice(0, 4) : '?';

  return (
    <div style={{
      width: 48,
      height: 48,
      borderRadius: 10,
      backgroundColor: bg,
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'center',
      flexShrink: 0,
      gap: 2,
    }}>
      <Icon size={18} color={color} />
      <span style={{ fontSize: 9, color, fontWeight: 700, letterSpacing: '0.03em', lineHeight: 1 }}>
        {label}
      </span>
    </div>
  );
}

/* ─── Progress bar ──────────────────────────────────────────── */

export function ThinProgressBar({ percent, indeterminate = false }: { percent: number; indeterminate?: boolean }) {
  return (
    <div style={{
      height: 3,
      borderRadius: 2,
      backgroundColor: 'var(--gd-outline-variant)',
      overflow: 'hidden',
      width: '100%',
    }}>
      <div style={{
        height: '100%',
        borderRadius: 2,
        backgroundColor: 'var(--gd-blue)',
        width: indeterminate ? '40%' : `${percent}%`,
        transition: indeterminate ? 'none' : 'width 0.3s ease',
        animation: indeterminate ? 'gd-indeterminate 1.4s infinite ease-in-out' : 'none',
      }} />
    </div>
  );
}

/* ─── Empty state ───────────────────────────────────────────── */

export const TransferEmptyState = ({ title, icon: Icon }: { title: string; icon: React.ElementType }) => (
  <div style={{
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    justifyContent: 'center',
    padding: '80px 24px',
    textAlign: 'center',
    borderRadius: 'var(--gd-radius-md)',
    border: '2px dashed var(--gd-outline)',
  }}>
    <div style={{
      width: 64, height: 64,
      borderRadius: 'var(--gd-radius-full)',
      backgroundColor: 'var(--gd-surface-variant)',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      marginBottom: 16,
    }}>
      <Icon size={28} style={{ color: 'var(--gd-on-surface-variant)' }} />
    </div>
    <h3 style={{
      fontSize: 16, fontFamily: "'Google Sans', sans-serif",
      fontWeight: 500, margin: 0, color: 'var(--gd-on-surface)',
    }}>
      {title}
    </h3>
  </div>
);

/* ─── Icon action button ────────────────────────────────────── */

export function ActionBtn({ onClick, title, children }: {
  onClick: () => void;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      title={title}
      className="gd-icon-btn"
      style={{ width: 32, height: 32, borderRadius: 'var(--gd-radius-full)', flexShrink: 0 }}
    >
      {children}
    </button>
  );
}

/* ─── Shared transfer card (uploads + downloads) ────────────── */

export type TransferCardProps = {
  filename: string;
  kind?: string | null;
  sublabel: React.ReactNode;
  extra?: React.ReactNode;
  progress: { percent: number; indeterminate?: boolean } | null;
  actions: React.ReactNode;
  dimmed?: boolean;
};

export const TransferCard = memo(function TransferCard({
  filename, kind, sublabel, extra, progress, actions, dimmed,
}: TransferCardProps) {
  return (
    <div style={{
      display: 'flex',
      alignItems: 'center',
      gap: 14,
      padding: '14px 16px',
      borderRadius: 'var(--gd-radius-md)',
      backgroundColor: 'var(--gd-surface)',
      border: '1px solid var(--gd-outline-variant)',
      marginBottom: 6,
      opacity: dimmed ? 0.65 : 1,
      transition: 'background 0.15s, opacity 0.2s',
    }}>
      <FileIcon filename={filename} kind={kind} />
      <div style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column', gap: 5 }}>
        <span style={{
          fontSize: 14, fontWeight: 600,
          overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
          color: 'var(--gd-on-surface)',
        }}>
          {filename}
        </span>
        <span style={{ fontSize: 12, color: 'var(--gd-on-surface-variant)', lineHeight: 1.4 }}>
          {sublabel}
        </span>
        {extra}
        {progress && <ThinProgressBar percent={progress.percent} indeterminate={progress.indeterminate} />}
      </div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 4, flexShrink: 0 }}>
        {actions}
      </div>
    </div>
  );
});
