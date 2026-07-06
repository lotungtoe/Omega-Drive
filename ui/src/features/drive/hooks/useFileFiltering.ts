import { useMemo } from 'react';

/**
 * useFileFiltering Hook: Lọc danh sách file theo từ khóa tìm kiếm và trạng thái đánh dấu sao.
 */
export function useFileFiltering(baseFiles, search) {
  const filteredFiles = useMemo(() => {
    return baseFiles
      // Lọc theo tên file (không phân biệt hoa thường)
      .filter(f => {
        if (!search) return true;
        const name = f.isFolder ? f.name : f.filename;
        return name?.toLowerCase().includes(search.toLowerCase());
      });
  }, [baseFiles, search]);

  return filteredFiles;
}
