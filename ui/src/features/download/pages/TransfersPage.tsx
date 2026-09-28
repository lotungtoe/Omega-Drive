import { useState } from 'react';
import { Pause, Play, X, RefreshCw, Upload, Download } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useDownloads } from '../hooks/useDownloads';
import { useTransfersList } from '../hooks/useTransfersList';
import { toDownloadView, toUploadView, type TransferView, type DownloadJobRow, type UploadFileRow } from '../components/transferView';
import {
  TransferCard,
  TransferEmptyState,
  TransferSublabel,
  ActionBtn,
} from '../components/TransferCard';

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

  const renderActions = (view: TransferView) => {
    if (view.source === 'download') {
      const job = view.ref as DownloadJobRow;
      return <>
        {view.status === 'active' && (
          <ActionBtn onClick={() => pauseJob(job.id)} title={t('downloads.pause')}>
            <Pause size={15} />
          </ActionBtn>
        )}
        {view.status === 'paused' && (
          <ActionBtn onClick={() => resumeJob(job.id)} title={t('downloads.resume')}>
            <Play size={15} />
          </ActionBtn>
        )}
        {view.status === 'failed' && (
          <ActionBtn onClick={() => retryJob(job.id)} title={t('downloads.retry')}>
            <RefreshCw size={15} />
          </ActionBtn>
        )}
        {['queued', 'active', 'paused', 'failed'].includes(view.status) && (
          <ActionBtn onClick={() => cancelJob(job.id)} title={t('downloads.cancel')}>
            <X size={15} />
          </ActionBtn>
        )}
      </>;
    }
    const file = view.ref as UploadFileRow;
    return <>
      {file?.local_path && (
        <ActionBtn onClick={() => resumeUpload(file)} title={t('upload.resumeUpload')}>
          <Play size={15} />
        </ActionBtn>
      )}
      <ActionBtn onClick={() => cancelUpload(file?.id)} title={t('common.cancel')}>
        <X size={15} />
      </ActionBtn>
    </>;
  };

  const renderRow = (view: TransferView) => (
    <TransferCard
      key={view.key}
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
      actions={renderActions(view)}
    />
  );

  const uploadViews = uploads.map((file) => toUploadView(file, liveStats ?? {}, t));
  const downloadViews = downloadJobs.map((job) => toDownloadView(job, dlStats, t));

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
          {uploadViews.map(renderRow)}
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
          {downloadViews.map(renderRow)}
        </div>
      )}
    </section>
  );
}

export default TransfersPage;
