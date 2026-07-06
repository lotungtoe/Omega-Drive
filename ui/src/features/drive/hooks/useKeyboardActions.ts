import { useEffect } from 'react';

/**
 * useKeyboardActions Hook: Listen for global keyboard shortcuts to enhance user experience.
 */
export function useKeyboardActions(search, setSearch, refresh, setShowSettings, setShowNewFolder, setPreviewFile) {
  useEffect(() => {
    const handleKeyDown = (e) => {
      // Press "/" or "Ctrl+K" to focus search input
      if ((e.key === '/' && document.activeElement.tagName !== 'INPUT') || (e.key === 'k' && (e.ctrlKey || e.metaKey))) {
        e.preventDefault();
        document.getElementById('header-search-input')?.focus();
      }
      
      // Press "Escape" to close all open windows or clear search input
      if (e.key === 'Escape') {
        if (search) setSearch('');
        setShowSettings(false);
        setShowNewFolder(false);
        setPreviewFile(null);
      }
      
      // Press "F5" to refresh data from server
      if (e.key === 'F5') {
        e.preventDefault();
        refresh();
      }
    };
    
    globalThis.addEventListener('keydown', handleKeyDown);
    return () => globalThis.removeEventListener('keydown', handleKeyDown);
  }, [search, setSearch, refresh, setShowSettings, setShowNewFolder, setPreviewFile]);
}
