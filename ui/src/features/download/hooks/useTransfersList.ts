import { useState, useCallback, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { fetchTransfersPaginated } from '../../drive/services/driveService';
import { toUserMessage } from '../../../shared/services/errors/toUserMessage';
import { resumeUploadByPath } from '../../upload/services/uploadService';
import { DriveApi } from '../../../api/index';

export function useTransfersList(toast) {
  const [uploads, setUploads] = useState([]);
  const [loading, setLoading] = useState(true);
  const [cursor, setCursor] = useState(null);
  const [hasMore, setHasMore] = useState(false);
  const loadingMoreRef = useRef(false);

  const loadUploads = useCallback(async (reset = false) => {
    try {
      const fetchCursor = reset ? null : cursor;
      if (!reset && (!hasMore || loadingMoreRef.current)) return;

      if (reset) {
        setLoading(true);
      } else {
        loadingMoreRef.current = true;
      }

      const res = await fetchTransfersPaginated(fetchCursor, 50);
      
      setUploads(prev => reset ? res.files : [...prev, ...res.files]);
      setCursor(res.next_cursor);
      setHasMore(res.has_more);
    } catch (err) {
      console.error('Failed to load uploads:', err);
      const msg = toUserMessage(err);
      toast?.show?.(msg.message || 'Error loading upload list', 'error');
    } finally {
      setLoading(false);
      loadingMoreRef.current = false;
    }
  }, [cursor, hasMore, toast]);

  useEffect(() => {
    void loadUploads(true);

    const refreshOnProgress = listen('upload-progress', (event) => {
      const phase = event.payload?.phase;
      if (phase === 'done' || phase === 'failed') {
        void loadUploads(true);
      }
    });

    return () => {
      refreshOnProgress.then((fn) => fn());
    };
  }, [loadUploads]);

  const resumeUpload = useCallback(async (file: { local_path?: string | null; folder_id?: number | null; drive_scope?: string | null }) => {
    try {
      await resumeUploadByPath(file);
    } catch (err) {
      const msg = toUserMessage(err);
      toast?.show?.(msg.message || 'Could not resume upload', 'error');
    }
  }, [toast]);

  const cancelUpload = useCallback(async (fileId: number) => {
    try {
      await DriveApi.purgeFile(fileId);
      void loadUploads(true);
    } catch (err) {
      const msg = toUserMessage(err);
      toast?.show?.(msg.message || 'Could not cancel upload', 'error');
    }
  }, [loadUploads, toast]);

  return { uploads, loading, hasMore, loadMore: () => loadUploads(false), refresh: () => loadUploads(true), resumeUpload, cancelUpload };
}
