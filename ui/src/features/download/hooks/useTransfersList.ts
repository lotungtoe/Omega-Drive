import { useState, useCallback, useEffect, useRef } from 'react';
import { safeListen as listen } from '../../../shared/api/tauri';
import { fetchTransfersPaginated } from '../../drive/services/driveService';
import { toUserMessage } from '../../../shared/services/errors/toUserMessage';
import { resumeUploadByPath } from '../../upload/services/uploadService';
import { DriveApi } from '../../../api/index';
import {
  uploadStatKey,
  uploadBytes,
  nextSpeed,
  type UploadLiveStat,
  type UploadProgressPayload,
  type SpeedSample,
} from './uploadProgress';

export function useTransfersList(toast) {
  const [uploads, setUploads] = useState([]);
  const [liveStats, setLiveStats] = useState<Record<string, UploadLiveStat>>({});
  const [loading, setLoading] = useState(true);
  const [cursor, setCursor] = useState(null);
  const [hasMore, setHasMore] = useState(false);
  const loadingMoreRef = useRef(false);
  const toastRef = useRef(toast);
  const cursorRef = useRef(cursor);
  const hasMoreRef = useRef(hasMore);
  const samplesRef = useRef<Record<string, SpeedSample>>({});
  useEffect(() => {
    toastRef.current = toast;
    cursorRef.current = cursor;
    hasMoreRef.current = hasMore;
  });

  const loadUploads = useCallback(async (reset = false) => {
    try {
      const fetchCursor = reset ? null : cursorRef.current;
      if (!reset && (!hasMoreRef.current || loadingMoreRef.current)) return;

      if (reset) {
        setLoading(true);
      } else {
        loadingMoreRef.current = true;
      }

      const res: any = await fetchTransfersPaginated(fetchCursor, 50);

      const files = Array.isArray(res?.files) ? res.files : [];
      setUploads(prev => reset ? files : [...prev, ...files]);
      setCursor(res.next_cursor);
      setHasMore(res.has_more);
    } catch (err) {
      console.error('Failed to load uploads:', err);
      const msg = toUserMessage(err);
      toastRef.current?.show?.(msg.message || 'Error loading upload list', 'error');
    } finally {
      setLoading(false);
      loadingMoreRef.current = false;
    }
  }, []);

  useEffect(() => {
    void loadUploads(true);

    const refreshOnProgress = listen('upload-progress', (event) => {
      try {
        const p = (event?.payload ?? {}) as Partial<UploadProgressPayload>;
        const phase = String(p.phase ?? "");
        const key = uploadStatKey({ fileId: p.fileId ?? null, fileName: p.fileName ?? "" });
        if (phase === 'done' || phase === 'failed') {
          if (key) {
            delete samplesRef.current[key];
            setLiveStats((prev) => {
              if (!(key in prev)) return prev;
              const next = { ...prev };
              delete next[key];
              return next;
            });
          }
          void loadUploads(true);
          return;
        }
        if (!key) return;
        const percent = Math.min(Math.max(Number(p.overallProgress) || 0, 0), 100);
        const { done, total } = uploadBytes({ platforms: p.platforms ?? [] });
        const now = performance.now();
        const { speed, sample } = nextSpeed(samplesRef.current[key], done, now);
        samplesRef.current[key] = sample;
        const stat: UploadLiveStat = {
          percent,
          detail: String(p.detail ?? ""),
          phase,
          speedBps: speed,
          etaSecs: speed > 0 && total > done ? (total - done) / speed : null,
          bytesDone: done,
          bytesTotal: total,
        };
        // Throttle: backend emits per chunk; skip render if nothing visible changed.
        setLiveStats((prev) => {
          const cur = prev[key];
          if (cur && Math.round(cur.percent) === Math.round(percent) && cur.phase === phase) return prev;
          return { ...prev, [key]: stat };
        });
      } catch {
        // Never let stats bookkeeping break the page.
      }
    });

    return () => {
      refreshOnProgress.then((fn) => fn());
    };
  }, [loadUploads]);

  const resumeUpload = useCallback(async (file: any) => {
    try {
      await resumeUploadByPath(file as any);
    } catch (err) {
      const msg = toUserMessage(err);
      toastRef.current?.show?.(msg.message || 'Could not resume upload', 'error');
    }
  }, []);

  const cancelUpload = useCallback(async (fileId: number) => {
    try {
      await DriveApi.purgeFile(fileId);
      void loadUploads(true);
    } catch (err) {
      const msg = toUserMessage(err);
      toastRef.current?.show?.(msg.message || 'Could not cancel upload', 'error');
    }
  }, [loadUploads]);

  return { uploads, loading, liveStats, hasMore, loadMore: () => loadUploads(false), refresh: () => loadUploads(true), resumeUpload, cancelUpload };
}
