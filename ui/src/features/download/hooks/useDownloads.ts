import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { listen } from '@tauri-apps/api/event';
import {
  listDownloadJobs,
  pauseDownload,
  resumeDownload,
  cancelDownload,
  retryDownload,
} from '../services/downloadService';
import { toUserMessage } from '../../../shared/services/errors/toUserMessage';

export function useDownloads(toast) {
  const { t } = useTranslation();
  const [jobs, setJobs] = useState([]);
  const [loading, setLoading] = useState(true);
  const mountedRef = useRef(true);
  const toastRef = useRef(toast);
  toastRef.current = toast;
  const tRef = useRef(t);
  tRef.current = t;

  const refresh = useCallback(async () => {
    try {
      const data = await listDownloadJobs();
      if (mountedRef.current) {
        setJobs(Array.isArray(data) ? data : []);
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

    const setup = async () => {
      try {
        [unlistenQueued, unlistenComplete, unlistenFailed] = await Promise.all([
          listen('download-queued', () => refresh()),
          listen('download-complete', () => refresh()),
          listen('download-failed', () => refresh()),
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
    pauseJob,
    resumeJob,
    cancelJob,
    retryJob,
  };
}

