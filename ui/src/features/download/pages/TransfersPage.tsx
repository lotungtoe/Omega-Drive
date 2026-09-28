import { memo, useMemo, useState } from 'react';
import { Pause, Play, X, RefreshCw, Upload, Download } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useDownloads } from '../hooks/useDownloads';
import { useTransfersList } from '../hooks/useTransfersList';
import { uploadStatKey, resolveUploadStat, type UploadLiveStat } from '../hooks/uploadProgress';
import {
  toDownloadView,
  toUploadView,
  type TransferView,
  type DownloadJobRow,
  type UploadFileRow,
  type DownloadStatsLike,
} from '../components/transferView';
import {
  TransferCard,
  TransferEmptyState,
  TransferSublabel,
  ActionBtn,
} from '../components/TransferCard';

function ViewCard({ view, actions }: { view: TransferView; actions: React.ReactNode }) {
  return (
    <TransferCard
      filename={view.filename}
      kind={view.kind}
      sublabel={<TransferSublabel view={view} />}
      extra={view.error
        ? <span style={{ fontSize: 11, color: '#ef4444' }}>{view.error}</span>
        : view.extra
          ? <span style={{ fontSize: 11, color: 'var(--gd-on-surface-variant)' }}>{view.extra}</span>
          : undefined}
      progress={view.indeterminate ? { percent: 0, indeterminate: true } : { percent: view.percent }}
      dimmed={view.status === 'done'}
      actions={actions}
    />
  );
}

/* Memo rows: re-render only when their own file/job or stat changes.
   Props must stay referentially stable — mapper runs inside the row. */

const UploadRow = memo(function UploadRow({ file, stat, onResume, onCancel }: {
  file: UploadFileRow;
  stat: UploadLiveStat | null;
  onResume: (file: UploadFileRow) => void;
  onCancel: (id: number) => void;
}) {
  const { t } = useTranslation();
  const single = useMemo(
    () => (stat ? { [uploadStatKey({ fileId: file.id ?? null, fileName: file.filename ?? '' }) ?? '']: stat } : {}),
    [stat, file.id, file.filename]
  );
  const view = toUploadView(file, single, t);
  return (
    <ViewCard
      view={view}
      actions={<>
        {file?.local_path && (
          <ActionBtn onClick={() => onResume(file)} title={t('upload.resumeUpload')}>
            <Play size={15} />
          </ActionBtn>
        )}
        <ActionBtn onClick={() => onCancel(file?.id as number)} title={t('common.cancel')}>
          <X size={15} />
        </ActionBtn>
      </>}
    />
  );
});

const DownloadRow = memo(function DownloadRow({ job, stat, onPause, onResume, onRetry, onCancel }: {
  job: DownloadJobRow;
  stat: DownloadStatsLike | null;
  onPause: (id: number) => void;
  onResume: (id: number) => void;
  onRetry: (id: number) => void;
  onCancel: (id: number) => void;
}) {
  const { t } = useTranslation();
  const single = useMemo(
    () => (stat ? { [job.id as number]: stat } : undefined),
    [stat, job.id]
  );
  const view = toDownloadView(job, single, t);
  return (
    <ViewCard
      view={view}
      actions={<>
        {view.status === 'active' && (
          <ActionBtn onClick={() => onPause(job.id as number)} title={t('downloads.pause')}>
            <Pause size={15} />
          </ActionBtn>
        )}
        {view.status === 'paused' && (
          <ActionBtn onClick={() => onResume(job.id as number)} title={t('downloads.resume')}>
            <Play size={15} />
          </ActionBtn>
        )}
        {view.status === 'failed' && (
          <ActionBtn onClick={() => onRetry(job.id as number)} title={t('downloads.retry')}>
            <RefreshCw size={15} />
          </ActionBtn>
        )}
        {['queued', 'active', 'paused', 'failed'].includes(view.status) && (
          <ActionBtn onClick={() => onCancel(job.id as number)} title={t('downloads.cancel')}>
            <X size={15} />
          </ActionBtn>
        )}
      </>}
    />
  );
});

/* ─── Page ─────────────────────────────────────────────────── */

export function TransfersPage({ toast }: { toast: unknown }) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState('uploads');

  const { jobs: downloadJobs, loading: downloadsLoading, stats: dlStats, pauseJob, resumeJob, cancelJob, retryJob } = useDownloads(toast);
  const { uploads, loading: uploadsLoading, liveStats, resumeUpload, cancelUpload } = useTransfersList(toast);

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

      {/* ── Uploads tab ── */}
      {activeTab === 'uploads' && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
          {uploadsLoading && uploads.length === 0 && (
            <TransferEmptyState title="Loading list..." icon={Upload} />
          )}
          {!uploadsLoading && uploads.length === 0 && (
            <TransferEmptyState title="No files uploading or processing" icon={Upload} />
          )}
          {uploads.map((file) => (
            <UploadRow
              key={file.id}
              file={file}
              stat={resolveUploadStat(file, liveStats ?? {})}
              onResume={resumeUpload}
              onCancel={cancelUpload}
            />
          ))}
        </div>
      )}

      {/* ── Downloads tab ── */}
      {activeTab === 'downloads' && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
          {downloadsLoading && downloadJobs.length === 0 && (
            <TransferEmptyState title={t('downloads.loadingList')} icon={Download} />
          )}
          {!downloadsLoading && downloadJobs.length === 0 && (
            <TransferEmptyState title={t('downloads.empty')} icon={Download} />
          )}
          {downloadJobs.map((job) => (
            <DownloadRow
              key={job.id}
              job={job}
              stat={(dlStats as Record<number, DownloadStatsLike> | undefined)?.[job.id] ?? null}
              onPause={pauseJob}
              onResume={resumeJob}
              onRetry={retryJob}
              onCancel={cancelJob}
            />
          ))}
        </div>
      )}
    </section>
  );
}

export default TransfersPage;
