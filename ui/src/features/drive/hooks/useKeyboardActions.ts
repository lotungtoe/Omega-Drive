import { useEffect } from 'react';

/**
 * useKeyboardActions Hook: Lắng nghe các phím tắt toàn cục để tăng trải nghiệm người dùng.
 */
export function useKeyboardActions(search, setSearch, refresh, setShowSettings, setShowNewFolder, setPreviewFile) {
  useEffect(() => {
    const handleKeyDown = (e) => {
      // Nhấn "/" hoặc "Ctrl+K" để tập trung vào ô tìm kiếm
      if ((e.key === '/' && document.activeElement.tagName !== 'INPUT') || (e.key === 'k' && (e.ctrlKey || e.metaKey))) {
        e.preventDefault();
        document.getElementById('header-search-input')?.focus();
      }
      
      // Nhấn "Escape" để đóng tất cả các cửa sổ đang mở hoặc xóa nội dung tìm kiếm
      if (e.key === 'Escape') {
        if (search) setSearch('');
        setShowSettings(false);
        setShowNewFolder(false);
        setPreviewFile(null);
      }
      
      // Nhấn "F5" để làm mới dữ liệu từ server
      if (e.key === 'F5') {
        e.preventDefault();
        refresh();
      }
    };
    
    globalThis.addEventListener('keydown', handleKeyDown);
    return () => globalThis.removeEventListener('keydown', handleKeyDown);
  }, [search, setSearch, refresh, setShowSettings, setShowNewFolder, setPreviewFile]);
}
