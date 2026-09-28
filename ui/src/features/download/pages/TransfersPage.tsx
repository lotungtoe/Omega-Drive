import { useState } from 'react';
import { Pause, Play, X, RefreshCw, Upload, Download } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useDownloads } from '../hooks/useDownloads';
import { useTransfersList } from '../hooks/useTransfersList';
import { resolveUploadStat } from '../hooks/uploadProgress';
import { formatSize } from '../../../shared/utils/index';
import {
  TransferCard,
  TransferEmptyState,
  ActionBtn,
} from '../components/TransferCard';

/* ─── Helpers ──────────────────────────────────────────────── */

function getFilename(targetPath?: string | null) {
  if (!targetPath) return 'Unknown';
  const parts = targetPath.split(/[/\\]/);
  return parts[parts.length - 1] || targetPath;
}

function formatPercent(done: number, total: number) {
  if (!total || total <= 0) return 0;
  return Math.min(Math.round((done / total) * 100), 100);
}

function formatEta(secs: number | null | undefined, t: (k: string, o?: any) => string): string {
  if (secs == null || !Number.isFinite(secs) || secs < 0) return "";
  const s = Math.round(secs);
  if (s < 60) return t('downloads.timeSecs', { n: s });
  const m = Math.round(s / 60);
  if (m < 60) return t('downloads.timeMins', { n: m });
  return t('downloads.timeHours', { n: Math.round(m / 60) });
}

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
          {uploads.map((file) => {
            const stat = resolveUploadStat(file, liveStats ?? {});
            const isProcessing = file.status === 'processing' && (!stat || stat.phase === 'processing');
            const percent = stat ? Math.min(Math.round(stat.percent), 100) : 0;
            const total = stat && stat.bytesTotal > 0 ? stat.bytesTotal : (file.size || 0);
            const done = stat && stat.bytesTotal > 0
              ? stat.bytesDone
              : Math.round((percent / 100) * (file.size || 0));
            const showLive = !!stat && !isProcessing;

            const sublabel = isProcessing ? (
              <>Processing...</>
            ) : showLive && stat.speedBps > 0 && total > 0 ? (
              <>
                {formatSize(stat.speedBps)}/s&nbsp;·&nbsp;{formatSize(done)} {t('downloads.ofWord')} {formatSize(total)}
                {stat.etaSecs != null && Number.isFinite(stat.etaSecs) ? t('downloads.eta', { time: formatEta(stat.etaSecs, t) }) : ''}
              </>
            ) : showLive ? (
              <>{percent}%{stat.detail ? ` · ${stat.detail}` : ''}</>
            ) : (
              <>{formatSize(file.size || 0)} &nbsp;·&nbsp; Uploading...</>
            );

            return (
              <TransferCard
                key={file.id}
                filename={file.filename}
                sublabel={sublabel}
                extra={isProcessing ? (
                  <span style={{ fontSize: 11, color: 'var(--gd-on-surface-variant)' }}>
                    Processing (export, mp4, encode)...
                  </span>
                ) : undefined}
                progress={isProcessing ? { percent: 0, indeterminate: true } : { percent }}
                actions={<>
                  {file.local_path && (
                    <ActionBtn onClick={() => resumeUpload(file)} title={t('upload.resumeUpload')}>
                      <Play size={15} />
                    </ActionBtn>
                  )}
                  <ActionBtn onClick={() => cancelUpload(file.id)} title={t('common.cancel')}>
                    <X size={15} />
                  </ActionBtn>
                </>}
              />
            );
          })}
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
          {downloadJobs.map((job) => {
            const done = Math.max(job.done_parts || 0, 0);
            const total = Math.max(job.total_parts || 0, 0);
            const percent = formatPercent(done, total);
            const filename = getFilename(job.target_path);
            const isActive = job.state === 'downloading';
            const isPaused = job.state === 'paused';
            const isFailed = job.state === 'failed';
            const isDone   = job.state === 'completed' || job.state === 'done';
            const canCancel = ['queued', 'downloading', 'paused', 'failed'].includes(job.state);

            // Live event stats first, static preview fields (mock-only) as fallback.
            const st = (dlStats as Record<number, any>)[job.id]
              ?? (Number.isFinite(job.speed_bps)
                ? { speedBps: job.speed_bps, etaSecs: job.eta_secs ?? null, bytesDone: job.bytes_done ?? 0, bytesTotal: job.bytes_total ?? 0 }
                : null);

            const stateLabel = {
              downloading: 'Downloading',
              paused: 'Paused',
              failed: 'Failed',
              queued: 'Queued',
              completed: 'Completed',
              done: 'Completed',
            }[job.state] ?? job.state;

            const sublabel = isActive && st ? (
              <>
                {formatSize(st.speedBps)}/s&nbsp;·&nbsp;{formatSize(st.bytesDone)} {t('downloads.ofWord')} {formatSize(st.bytesTotal)}
                {st.etaSecs != null && Number.isFinite(st.etaSecs) ? t('downloads.eta', { time: formatEta(st.etaSecs, t) }) : ''}
              </>
            ) : (
              <span style={isFailed ? { color: '#ef4444' } : undefined}>
                {t('downloads.part', { current: done, total })}
                &nbsp;·&nbsp;{stateLabel}
                {job.error_code ? ` (${job.error_code})` : ''}
              </span>
            );

            return (
              <TransferCard
                key={job.id}
                filename={filename}
                sublabel={sublabel}
                extra={job.error ? (
                  <span style={{ fontSize: 11, color: '#ef4444' }}>{job.error}</span>
                ) : undefined}
                progress={{ percent: isDone ? 100 : percent }}
                dimmed={isDone}
                actions={<>
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
                </>}
              />
            );
          })}
        </div>
      )}
    </section>
  );
}

export default TransfersPage;
