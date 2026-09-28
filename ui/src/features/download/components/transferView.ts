import { formatSize } from '../../../shared/utils/index';
import { resolveUploadStat, type UploadLiveStat } from '../hooks/uploadProgress';

export type TransferStatus = 'active' | 'paused' | 'failed' | 'queued' | 'processing' | 'done';

/** Live per-job numbers (mirrors useDownloads stats shape). */
export type DownloadStatsLike = {
  speedBps?: number;
  etaSecs?: number | null;
  bytesDone?: number;
  bytesTotal?: number;
};

export type Ti18n = (k: string, o?: Record<string, string | number | null | undefined>) => string;

/** Minimal download-job shape the mapper reads. */
export type DownloadJobRow = {
  id?: number;
  target_path?: string | null;
  state?: string;
  done_parts?: number;
  total_parts?: number;
  speed_bps?: number;
  eta_secs?: number | null;
  bytes_done?: number;
  bytes_total?: number;
  error_code?: string;
  error?: string;
};

/** Minimal upload-file shape the mapper reads. */
export type UploadFileRow = {
  id?: number;
  filename?: string;
  size?: number;
  status?: string;
  local_path?: string | null;
};

/** Single view model for both tabs. Mappers own domain wording; one renderer draws. */
export type TransferView = {
  key: string | number;
  filename: string;
  kind?: string | null;
  source: 'download' | 'upload';
  status: TransferStatus;
  percent: number;
  indeterminate?: boolean;
  speedBps: number;
  etaSecs: number | null;
  bytesDone: number;
  bytesTotal: number;
  /** Precomposed fallback line (used when no live speed line applies). */
  statusText: string;
  extra?: string;
  error?: string;
  /** Original row object, for the actions slot. */
  ref: unknown;
};

type T = Ti18n;

const basename = (p?: string | null): string => {
  if (!p) return 'Unknown';
  const parts = String(p).split(/[/\\]/);
  return parts[parts.length - 1] || String(p);
};

const clampPercent = (done: number, total: number): number => {
  if (!total || total <= 0) return 0;
  return Math.min(Math.round((done / total) * 100), 100);
};

const DOWNLOAD_STATE: Record<string, TransferStatus> = {
  downloading: 'active',
  paused: 'paused',
  failed: 'failed',
  queued: 'queued',
  completed: 'done',
  done: 'done',
};

const DOWNLOAD_LABEL: Record<TransferStatus, string> = {
  active: 'Downloading',
  paused: 'Paused',
  failed: 'Failed',
  queued: 'Queued',
  processing: 'Processing',
  done: 'Completed',
};

export function toDownloadView(job: DownloadJobRow, stats: Record<number, DownloadStatsLike> | undefined, t: T): TransferView {
  const done = Math.max(job?.done_parts || 0, 0);
  const total = Math.max(job?.total_parts || 0, 0);
  const status: TransferStatus = DOWNLOAD_STATE[job?.state] ?? 'queued';
  const st = stats?.[job?.id]
    ?? (Number.isFinite(job?.speed_bps)
      ? { speedBps: job.speed_bps, etaSecs: job.eta_secs ?? null, bytesDone: job.bytes_done ?? 0, bytesTotal: job.bytes_total ?? 0 }
      : null);
  const percent = status === 'done' ? 100 : clampPercent(done, total);
  const label = DOWNLOAD_LABEL[status] ?? job?.state;
  const statusText = `${t('downloads.part', { current: done, total })} · ${label}${job?.error_code ? ` (${job.error_code})` : ''}`;
  return {
    key: job?.id,
    filename: basename(job?.target_path),
    source: 'download',
    status,
    percent,
    speedBps: st?.speedBps ?? 0,
    etaSecs: st?.etaSecs ?? null,
    bytesDone: st?.bytesDone ?? 0,
    bytesTotal: st?.bytesTotal ?? 0,
    statusText,
    error: job?.error,
    ref: job,
  } as TransferView;
}

export function toUploadView(
  file: UploadFileRow,
  live: Record<string, UploadLiveStat> | undefined,
  t: T
): TransferView {
  void t;
  const stat = resolveUploadStat(file ?? {}, live ?? {});
  const isProcessing = file?.status === 'processing' && (!stat || stat.phase === 'processing');
  const percent = stat ? Math.min(Math.round(stat.percent), 100) : 0;
  const bytesTotal = stat && stat.bytesTotal > 0 ? stat.bytesTotal : (file?.size || 0);
  const bytesDone = stat && stat.bytesTotal > 0
    ? stat.bytesDone
    : Math.round((percent / 100) * (file?.size || 0));
  const statusText = isProcessing
    ? 'Processing...'
    : stat
      ? `${percent}%${stat.detail ? ` · ${stat.detail}` : ''}`
      : `${formatSize(file?.size || 0)} · Uploading...`;
  return {
    key: file?.id,
    filename: file?.filename ?? 'Unknown',
    source: 'upload',
    status: isProcessing ? 'processing' : 'active',
    percent,
    indeterminate: isProcessing ? true : undefined,
    speedBps: stat?.speedBps ?? 0,
    etaSecs: stat?.etaSecs ?? null,
    bytesDone,
    bytesTotal,
    statusText,
    extra: isProcessing ? 'Processing (export, mp4, encode)...' : undefined,
    ref: file,
  };
}
