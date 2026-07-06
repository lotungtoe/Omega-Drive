import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { getFileType } from '../../../shared/utils/index';

/**
 * useFileSorting Hook: Sap xep danh sach file theo cac tieu chi khac nhau.
 */
export function useFileSorting(files, sort) {
  const { t } = useTranslation();

  const sortedFiles = useMemo(() => {
    return [...files].sort((a, b) => {
      // Folder luon o tren cung
      if (a.isFolder && !b.isFolder) return -1;
      if (!a.isFolder && b.isFolder) return 1;

      const aName = a.isFolder ? a.name : a.filename;
      const bName = b.isFolder ? b.name : b.filename;

      let cmp = 0;
      if (sort.field === 'name') {
        cmp = (aName || '').localeCompare(bName || '');
      } else if (sort.field === 'size') {
        cmp = (a.size || 0) - (b.size || 0);
      } else if (sort.field === 'date') {
        cmp = new Date(a.created_at || a.last_modified) - new Date(b.created_at || b.last_modified);
      } else if (sort.field === 'type') {
        const aFileType = a.isFolder ? { labelKey: 'fileType.folder', ext: '' } : getFileType(a.filename, a.kind);
        const bFileType = b.isFolder ? { labelKey: 'fileType.folder', ext: '' } : getFileType(b.filename, b.kind);
        const aType = t(aFileType.labelKey, { ext: (aFileType.ext || '').toUpperCase() });
        const bType = t(bFileType.labelKey, { ext: (bFileType.ext || '').toUpperCase() });
        cmp = aType.localeCompare(bType);
      }

      return sort.dir === 'asc' ? cmp : -cmp;
    });
  }, [files, sort, t]);

  return sortedFiles;
}
