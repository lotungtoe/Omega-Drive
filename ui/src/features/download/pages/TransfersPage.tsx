import { useState } from 'react';
import {
  Pause, Play, X, RefreshCw, Upload, Download,
  FileVideo, FileAudio, FileImage, FileText, File,
  Archive, FileCode,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useDownloads } from '../hooks/useDownloads';
import { useTransfersList } from '../hooks/useTransfersList';
import { formatSize } from '../../../shared/utils/index';

/* â”€â”€â”€ Helpers â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€ */

function getFilename(targetPath?: string | null) {
  if (!targetPath) return 'Unknown';
  const parts = targetPath.split(/[/\\]/);
  return parts[parts.length - 1] || targetPath;
}

function formatPercent(done: number, total: number) {
  if (!total || total <= 0) return 0;
  return Math.min(Math.round((done / total) * 100), 100);
}

function getExt(filename: string) {
  const dot = filename.lastIndexOf('.');
  return dot >= 0 ? filename.slice(dot + 1).toLowerCase() : '';
}

/* â”€â”€â”€ File icon â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€ */

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

function FileIcon({ filename, kind }: { filename: string; kind?: string | null }) {
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

/* â”€â”€â”€ Progress bar â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€ */

function ThinProgressBar({ percent, indeterminate = false }: { percent: number; indeterminate?: boolean }) {
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

/* â”€â”€â”€ Empty state â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€ */

const TransferEmptyState = ({ title, icon: Icon }: { title: string; icon: React.ElementType }) => (
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

/* â”€â”€â”€ Icon action button â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€ */

function ActionBtn({ onClick, title, children }: {
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

/* â”€â”€â”€ Page â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€ */

export function TransfersPage({ toast }: { toast: unknown }) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState('uploads');

  const { jobs: downloadJobs, loading: downloadsLoading, pauseJob, resumeJob, cancelJob, retryJob } = useDownloads(toast);
  const { uploads, loading: uploadsLoading, resumeUpload, cancelUpload } = useTransfersList(toast);

  const tabBtn = (active: boolean) => ({
    padding: '7px 16px',
    backgroundColor: active ? 'var(--gd-blue-surface)' : 'transparent',
    color: active ? 'var(--gd-blue)' : 'var(--gd-on-surface-variant)',
    border: 'none',
    borderRadius: 'var(--gd-radius-full)',
    cursor: 'pointer',
    fontWeight: 500,
    fontSize: 14,
    display: 'flex',
    alignItems: 'center',
    gap: 6,
    transition: 'background 0.15s, color 0.15s',
  } as React.CSSProperties);

  return (
    <section style={{ display: 'flex', flexDirection: 'column', gap: 0 }}>
      {/* CSS for indeterminate animation */}
      <style>{`
        @keyframes gd-indeterminate {
          0%   { transform: translateX(-100%); width: 45%; }
          100% { transform: translateX(280%);  width: 45%; }
        }
      `}</style>

      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 20 }}>
        <h2 style={{ fontSize: 18, fontWeight: 600, margin: 0 }}>{t('sidebar.transfers')}</h2>
      </div>

      {/* Tabs */}
      <div style={{
        display: 'flex', gap: 4,
        borderBottom: '1px solid var(--gd-outline-variant)',
        paddingBottom: 12, marginBottom: 20,
      }}>
        <button type="button" style={tabBtn(activeTab === 'uploads')} onClick={() => setActiveTab('uploads')}>
          <Upload size={15} /> Upload
        </button>
        <button type="button" style={tabBtn(activeTab === 'downloads')} onClick={() => setActiveTab('downloads')}>
          <Download size={15} /> Download
        </button>
      </div>

      {/* â”€â”€ Uploads tab â”€â”€ */}
      {activeTab === 'uploads' && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
          {uploadsLoading && uploads.length === 0 && (
            <TransferEmptyState title="Loading list..." icon={Upload} />
          )}
          {!uploadsLoading && uploads.length === 0 && (
            <TransferEmptyState title="No files uploading or processing" icon={Upload} />
          )}
          {uploads.map((file) => {
            const isProcessing = file.status === 'processing';
            const statusLabel = isProcessing ? 'Processing...' : 'Uploading...';

            return (
              <div key={file.id} style={{
                display: 'flex',
                alignItems: 'center',
                gap: 14,
                padding: '14px 16px',
                borderRadius: 'var(--gd-radius-md)',
                backgroundColor: 'var(--gd-surface)',
                border: '1px solid var(--gd-outline-variant)',
                marginBottom: 6,
                transition: 'background 0.15s',
              }}>
                {/* File icon */}
                <FileIcon filename={file.filename} kind={file.kind} />

                {/* Info */}
                <div style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column', gap: 5 }}>
                  <span style={{
                    fontSize: 14, fontWeight: 600,
                    overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                    color: 'var(--gd-on-surface)',
                  }}>
                    {file.filename}
                  </span>
                  <span style={{ fontSize: 12, color: 'var(--gd-on-surface-variant)', lineHeight: 1.4 }}>
                    {formatSize(file.size || 0)} &nbsp;Â·&nbsp; {statusLabel}
                  </span>
                  <ThinProgressBar percent={0} indeterminate={!isProcessing} />
                  {isProcessing && (
                    <span style={{ fontSize: 11, color: 'var(--gd-on-surface-variant)' }}>
                      Processing (export, mp4, encode)...
                    </span>
                  )}
                </div>

                {/* Actions */}
                <div style={{ display: 'flex', alignItems: 'center', gap: 4, flexShrink: 0 }}>
                  {file.local_path && (
                    <ActionBtn onClick={() => resumeUpload(file)} title={t('upload.resumeUpload')}>
                      <Play size={15} />
                    </ActionBtn>
                  )}
                  <ActionBtn onClick={() => cancelUpload(file.id)} title={t('common.cancel')}>
                    <X size={15} />
                  </ActionBtn>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* â”€â”€ Downloads tab â”€â”€ */}
      {activeTab === 'downloads' && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
          {downloadsLoading && downloadJobs.length === 0 && (
            <TransferEmptyState title={t('downloads.loadingList')} icon={Download} />
          )}
          {!downloadsLoading && downloadJobs.length === 0 && (
            <TransferEmptyState title={t('downloads.empty')} icon={Download} />
          )}
          {downloadJobs.map((job) => {
            const done = Math.max(job.done_parts || 0, 0);
            const total = Math.max(job.total_parts || 0, 0);
            const percent = formatPercent(done, total);
            const filename = getFilename(job.target_path);
            const isActive = job.state === 'downloading';
            const isPaused = job.state === 'paused';
            const isFailed = job.state === 'failed';
            const isQueued = job.state === 'queued';
            const isDone   = job.state === 'completed' || job.state === 'done';
            const canCancel = ['queued', 'downloading', 'paused', 'failed'].includes(job.state);

            const stateLabel = {
              downloading: 'Downloading',
              paused: 'Paused',
              failed: 'Failed',
              queued: 'Queued',
              completed: 'Completed',
              done: 'Completed',
            }[job.state] ?? job.state;

            return (
              <div key={job.id} style={{
                display: 'flex',
                alignItems: 'center',
                gap: 14,
                padding: '14px 16px',
                borderRadius: 'var(--gd-radius-md)',
                backgroundColor: 'var(--gd-surface)',
                border: '1px solid var(--gd-outline-variant)',
                marginBottom: 6,
                opacity: isDone ? 0.65 : 1,
                transition: 'background 0.15s, opacity 0.2s',
              }}>
                {/* File icon */}
                <FileIcon filename={filename} kind={null} />

                {/* Info */}
                <div style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column', gap: 5 }}>
                  <span style={{
                    fontSize: 14, fontWeight: 600,
                    overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                    color: 'var(--gd-on-surface)',
                  }}>
                    {filename}
                  </span>
                  <span style={{ fontSize: 12, color: isFailed ? '#ef4444' : 'var(--gd-on-surface-variant)', lineHeight: 1.4 }}>
                    {t('downloads.part', { current: done, total })}
                    &nbsp;Â·&nbsp;{stateLabel}
                    {job.error_code ? ` (${job.error_code})` : ''}
                  </span>
                  {job.error && (
                    <span style={{ fontSize: 11, color: '#ef4444' }}>{job.error}</span>
                  )}
                  {!isDone && (
                    <ThinProgressBar percent={percent} indeterminate={isQueued} />
                  )}
                  {isDone && (
                    <ThinProgressBar percent={100} />
                  )}
                </div>

                {/* Actions */}
                <div style={{ display: 'flex', alignItems: 'center', gap: 4, flexShrink: 0 }}>
                  {isActive && (
                    <ActionBtn onClick={() => pauseJob(job.id)} title={t('downloads.pause')}>
                      <Pause size={15} />
                    </ActionBtn>
                  )}
                  {isPaused && (
                    <ActionBtn onClick={() => resumeJob(job.id)} title={t('downloads.resume')}>
                      <Play size={15} />
                    </ActionBtn>
                  )}
                  {isFailed && (
                    <ActionBtn onClick={() => retryJob(job.id)} title={t('downloads.retry')}>
                      <RefreshCw size={15} />
                    </ActionBtn>
                  )}
                  {canCancel && (
                    <ActionBtn onClick={() => cancelJob(job.id)} title={t('downloads.cancel')}>
                      <X size={15} />
                    </ActionBtn>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}

export default TransfersPage;

