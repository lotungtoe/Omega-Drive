import { useMemo } from 'react';

/**
 * useFileFiltering Hook: Filter file list by search keyword and bookmark status.
 */
export function useFileFiltering(baseFiles, search) {
  const filteredFiles = useMemo(() => {
    return baseFiles
      // Filter by file name (case-insensitive)
      .filter(f => {
        if (!search) return true;
        const name = f.isFolder ? f.name : f.filename;
        return name?.toLowerCase().includes(search.toLowerCase());
      });
  }, [baseFiles, search]);

  return filteredFiles;
}
