import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { safeListen as listen } from '../../../shared/api/tauri';
import {
  listDownloadJobs,
  pauseDownload,
  resumeDownload,
  cancelDownload,
  retryDownload,
} from '../services/downloadService';
import { toUserMessage } from '../../../shared/services/errors/toUserMessage';

export type DownloadStats = {
  speedBps: number;
  etaSecs: number | null;
  bytesDone: number;
  bytesTotal: number;
};

const basename = (p?: string | null): string => {
  if (!p) return "";
  const parts = String(p).split(/[/\\]/);
  return parts[parts.length - 1] || String(p);
};

export function useDownloads(toast) {
  const { t } = useTranslation();
  const [jobs, setJobs] = useState([]);
  const [stats, setStats] = useState<Record<number, DownloadStats>>({});
  const [loading, setLoading] = useState(true);
  const mountedRef = useRef(true);
  const toastRef = useRef(toast);
  const tRef = useRef(t);
  useEffect(() => {
    toastRef.current = toast;
    tRef.current = t;
  });
  const jobsRef = useRef<any[]>([]);
  const samplesRef = useRef<Record<number, { bytes: number; t: number }>>({});

  const refresh = useCallback(async () => {
    try {
      const data = await listDownloadJobs();
      if (mountedRef.current) {
        const raw = Array.isArray(data) ? data : [];
        setJobs(raw);
        jobsRef.current = raw;
      }
    } catch (err) {
      const msg = toUserMessage(err);
      console.error('Failed to list download jobs:', err);
      toastRef.current?.show?.(msg.message || tRef.current('downloads.loadFailed'), 'error');
    }
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    const runInitialRefresh = async () => {
      await refresh();
      queueMicrotask(() => {
        if (mountedRef.current) {
          setLoading(false);
        }
      });
    };

    void runInitialRefresh();

    let unlistenComplete;
    let unlistenFailed;
    let unlistenQueued;
    let unlistenProgress;

    // ponytail: progress events carry no job id — match by filename to the
    // first downloading job. Good enough for a status line; backend change if exact mapping matters.
    const onProgress = (event) => {
      try {
        const p = (event?.payload ?? {}) as Record<string, any>;
        const bytesDone = Number(p.bytesDone);
        const bytesTotal = Number(p.bytesTotal);
        if (!Number.isFinite(bytesDone) || !Number.isFinite(bytesTotal) || bytesTotal <= 0) return;
        const name = String(p.fileName ?? "");
        if (!name) return;
        const job = jobsRef.current.find((j) => j?.state === "downloading" && basename(j?.target_path) === name);
        if (!job) return;
        const now = performance.now();
        const prevSample = samplesRef.current[job.id];
        let speed = 0;
        if (prevSample && now > prevSample.t) {
          const v = (bytesDone - prevSample.bytes) / ((now - prevSample.t) / 1000);
          if (v > 0) speed = v;
        }
        samplesRef.current[job.id] = { bytes: bytesDone, t: now };
        const eta = speed > 0 ? (bytesTotal - bytesDone) / speed : null;
        setStats((s) => ({ ...s, [job.id]: { speedBps: speed, etaSecs: eta, bytesDone, bytesTotal } }));
      } catch {
        // Never let stats bookkeeping break the page.
      }
    };

    const setup = async () => {
      try {
        [unlistenQueued, unlistenComplete, unlistenFailed, unlistenProgress] = await Promise.all([
          listen('download-queued', () => refresh()),
          listen('download-complete', () => refresh()),
          listen('download-failed', () => refresh()),
          listen('download-progress', onProgress),
        ]);
      } catch (err) {
        console.warn('Failed to listen download events:', err);
      }
    };
    setup();

    return () => {
      mountedRef.current = false;
      if (unlistenQueued) unlistenQueued();
      if (unlistenComplete) unlistenComplete();
      if (unlistenFailed) unlistenFailed();
      if (unlistenProgress) unlistenProgress();
    };
  }, [refresh]);

  const pauseJob = useCallback(
    async (jobId) => {
      try {
        await pauseDownload(jobId);
        refresh();
      } catch (err) {
        const msg = toUserMessage(err);
        console.error('Pause download failed:', err);
        toastRef.current?.show?.(msg.message || tRef.current('downloads.pauseFailed'), 'error');
      }
    },
    [refresh]
  );

  const resumeJob = useCallback(
    async (jobId) => {
      try {
        await resumeDownload(jobId);
        refresh();
      } catch (err) {
        const msg = toUserMessage(err);
        console.error('Resume download failed:', err);
        toastRef.current?.show?.(msg.message || tRef.current('downloads.resumeFailed'), 'error');
      }
    },
    [refresh]
  );

  const cancelJob = useCallback(
    async (jobId) => {
      try {
        await cancelDownload(jobId);
        refresh();
      } catch (err) {
        const msg = toUserMessage(err);
        console.error('Cancel download failed:', err);
        toastRef.current?.show?.(msg.message || tRef.current('downloads.cancelFailed'), 'error');
      }
    },
    [refresh]
  );

  const retryJob = useCallback(
    async (jobId) => {
      try {
        await retryDownload(jobId);
        refresh();
      } catch (err) {
        const msg = toUserMessage(err);
        console.error('Retry download failed:', err);
        toastRef.current?.show?.(msg.message || tRef.current('downloads.retryFailed'), 'error');
      }
    },
    [refresh]
  );

  return {
    jobs,
    loading,
    refresh,
    stats,
    pauseJob,
    resumeJob,
    cancelJob,
    retryJob,
  };
}

