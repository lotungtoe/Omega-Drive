import { useCallback } from 'react';
import { selectDownloadPath, startDownload } from '../services/downloadService';
import { toUserMessage } from '../../../shared/services/errors/toUserMessage';

/**
 * useDownloadFile Hook: Handle downloading files from Cloud to local machine.
 */
export function useDownloadFile(toast) {
  /**
   * handleDownload: Open save dialog and start download.
   * @param {object} file - The file object to download.
   */
  const handleDownload = useCallback(async (file) => {
    try {
      // 1. Open system dialog to select save location (use save instead of open)
      const savePath = await selectDownloadPath(file);
      
      if (!savePath) return; // User cancelled the selection
      
      toast.show(`Starting download "${file.filename}"...`, 'info');
      
      // 2. Call API for Backend to download and reassemble file chunks
      await startDownload(file.id, savePath);
    } catch (err) {
      const msg = toUserMessage(err);
      console.error("Download failed:", err);
      toast.show(msg.message, 'error');
    }
  }, [toast]);

  return { handleDownload };
}
