import { useCallback } from 'react';
import { selectDownloadPath, startDownload } from '../services/downloadService';
import { toUserMessage } from '../../../shared/services/errors/toUserMessage';

/**
 * useDownloadFile Hook: Xử lý việc tải file từ Cloud về máy tính cá nhân.
 */
export function useDownloadFile(toast) {
  /**
   * handleDownload: Mở hộp thoại chọn nơi lưu và bắt đầu tải.
   * @param {object} file - Đối tượng file cần tải.
   */
  const handleDownload = useCallback(async (file) => {
    try {
      // 1. Mở cửa sổ hệ thống để người dùng chọn vị trí lưu file (Dùng save thay vì open)
      const savePath = await selectDownloadPath(file);
      
      if (!savePath) return; // Người dùng hủy bỏ việc chọn
      
      toast.show(`Bắt đầu tải xuống "${file.filename}"...`, 'info');
      
      // 2. Gọi API để Backend thực hiện việc tải và ghép các mảnh file
      await startDownload(file.id, savePath);
    } catch (err) {
      const msg = toUserMessage(err);
      console.error("Tải xuống thất bại:", err);
      toast.show(msg.message, 'error');
    }
  }, [toast]);

  return { handleDownload };
}
