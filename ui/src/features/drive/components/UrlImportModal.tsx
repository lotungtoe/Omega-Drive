import { useState, useEffect } from 'react';
import { X, Link, Download, Info, CheckCircle, AlertCircle, Loader2, PlayCircle } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useTranslation } from 'react-i18next';
import { DropdownSelect } from '../../../shared/components/DropdownSelect';
import { formatSize, cn } from '../../../shared/utils/index';

export const UrlImportModal = ({ dark, onClose, onImportStarted }) => {
  const { t } = useTranslation();
  const [url, setUrl] = useState('');
  const [loading, setLoading] = useState(false);
  const [metadata, setMetadata] = useState(null);
  const [error, setError] = useState(null);
  const [isImporting, setIsImporting] = useState(false);
  const [cookiesBrowser, setCookiesBrowser] = useState('');
  const [availableBrowsers, setAvailableBrowsers] = useState([]);

  useEffect(() => {
    invoke('get_available_browsers').then(setAvailableBrowsers).catch(() => {});
  }, []);

  const browserOptions = [
    { value: '', label: t('import.cookiesNone', 'None') },
    ...availableBrowsers.map(name => ({ value: name, label: name.charAt(0).toUpperCase() + name.slice(1) })),
  ];

  const cb = cookiesBrowser || null;

  const fetchMetadata = async () => {
    if (!url) return;
    setLoading(true);
    setError(null);
    setMetadata(null);
    try {
      const data = await invoke('get_url_metadata', { url, cookiesBrowser: cb });
      setMetadata(data);
    } catch (err) {
      console.error('Failed to fetch metadata:', err);
      setError(err?.message || 'An error occurred');
    } finally {
      setLoading(false);
    }
  };

  const handleImport = async () => {
    if (!metadata) return;
    setIsImporting(true);
    try {
      await onImportStarted(url, metadata, cb);
      onClose();
    } catch (err) {
      setError(err?.message || 'An error occurred');
      setIsImporting(false);
    }
  };

  return (
    <div className={cn("gd-modal-overlay", dark && "dark")}>
      <div 
        className="gd-modal-container"
        style={{
          width: '100%',
          maxWidth: '560px',
          background: 'var(--gd-surface-container-high)',
          borderRadius: '24px',
          boxShadow: '0 24px 48px rgba(0,0,0,0.2)',
          overflow: 'hidden',
          display: 'flex',
          flexDirection: 'column',
          animation: 'modal-pop 0.3s cubic-bezier(0.34, 1.56, 0.64, 1)'
        }}
      >
        {/* Header */}
        <div style={{
          padding: '24px 24px 16px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          borderBottom: '1px solid var(--gd-outline-variant)'
        }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
            <div style={{
              width: 40,
              height: 40,
              borderRadius: 12,
              backgroundColor: 'var(--gd-blue-surface)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color: 'var(--gd-blue)'
            }}>
              <Link size={20} />
            </div>
            <div>
              <h2 style={{ fontSize: 20, fontWeight: 600, color: 'var(--gd-on-surface)', margin: 0 }}>
                {t('import.urlTitle', 'Import from URL')}
              </h2>
              <p style={{ fontSize: 13, color: 'var(--gd-on-surface-variant)', margin: 0 }}>
                {t('import.urlSubtitle', 'Download video, audio, or files directly to storage')}
              </p>
            </div>
          </div>
          <button type="button" 
            onClick={onClose}
            className="gd-icon-btn"
            style={{ color: 'var(--gd-on-surface-variant)' }}
          >
            <X size={20} />
          </button>
        </div>

        {/* Content */}
        <div style={{ padding: '24px', display: 'flex', flexDirection: 'column', gap: 20 }}>
          {/* Input Area */}
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            <label style={{ fontSize: 13, fontWeight: 500, color: 'var(--gd-on-surface-variant)' }}>
              {t('import.urlLabel', 'Source link (YouTube, Facebook, Direct Link...)')}
            </label>
            <div style={{ display: 'flex', gap: 12 }}>
              <div style={{ flex: 1, position: 'relative' }}>
                <input
                  type="text"
                  value={url}
                  onChange={(e) => setUrl(e.target.value)}
                  placeholder="https://..."
                  style={{
                    width: '100%',
                    height: 48,
                    padding: '0 16px',
                    backgroundColor: 'var(--gd-surface-container-lowest)',
                    border: '1px solid var(--gd-outline)',
                    borderRadius: '12px',
                    color: 'var(--gd-on-surface)',
                    fontSize: 15,
                    outline: 'none',
                    transition: 'border-color 0.2s'
                  }}
                  onKeyDown={(e) => e.key === 'Enter' && fetchMetadata()}
                />
              </div>
              <button type="button"
                onClick={fetchMetadata}
                disabled={loading || !url}
                className="gd-btn-filled"
                style={{
                  height: 48,
                  padding: '0 24px',
                  borderRadius: '12px',
                  backgroundColor: 'var(--gd-blue)',
                  color: 'white',
                  fontWeight: 600,
                  opacity: (loading || !url) ? 0.6 : 1,
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8
                }}
              >
                {loading ? <Loader2 size={18} className="animate-spin" /> : <Info size={18} />}
                {t('import.fetchBtn', 'Check')}
              </button>
            </div>
          </div>

          {/* Browser Cookies */}
          <div style={{ display: 'flex', alignItems: 'center', gap: 12, paddingLeft: 2 }}>
            <label style={{ fontSize: 13, fontWeight: 500, color: 'var(--gd-on-surface-variant)', whiteSpace: 'nowrap' }}>
              {t('import.cookiesLabel', 'Browser cookies')}
            </label>
            <DropdownSelect
              value={cookiesBrowser}
              onChange={setCookiesBrowser}
              options={browserOptions}
              placeholder=""
              disabled={false}
              style={{ width: 140 }}
              onDoubleClick={undefined}
            />
          </div>

          {/* Metadata Display */}
          {error && (
            <div style={{
              padding: '16px',
              backgroundColor: 'var(--gd-error-container)',
              color: 'var(--gd-on-error-container)',
              borderRadius: '12px',
              display: 'flex',
              gap: 12,
              fontSize: 14
            }}>
              <AlertCircle size={20} style={{ flexShrink: 0 }} />
              <span>{error}</span>
            </div>
          )}

          {metadata && (
            <div style={{
              padding: '16px',
              backgroundColor: 'var(--gd-surface-container-low)',
              borderRadius: '16px',
              border: '1px solid var(--gd-outline-variant)',
              display: 'flex',
              gap: 16,
              animation: 'fade-in 0.3s ease'
            }}>
              {metadata.thumbnail ? (
                <div style={{
                  width: 120,
                  height: 68,
                  borderRadius: '8px',
                  overflow: 'hidden',
                  flexShrink: 0,
                  position: 'relative',
                  backgroundColor: '#000'
                }}>
                  <img src={metadata.thumbnail} alt="thumbnail" style={{ width: '100%', height: '100%', objectFit: 'cover' }} />
                  <div style={{
                    position: 'absolute',
                    bottom: 4,
                    right: 4,
                    backgroundColor: 'rgba(0,0,0,0.7)',
                    padding: '2px 4px',
                    borderRadius: 4,
                    fontSize: 10,
                    color: 'white'
                  }}>
                    {metadata.duration_string || 'Media'}
                  </div>
                </div>
              ) : (
                <div style={{
                  width: 120,
                  height: 68,
                  borderRadius: '8px',
                  backgroundColor: 'var(--gd-surface-container-high)',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  color: 'var(--gd-on-surface-variant)',
                  flexShrink: 0
                }}>
                  <PlayCircle size={32} />
                </div>
              )}
              <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 4, overflow: 'hidden' }}>
                <div style={{ 
                  fontSize: 15, 
                  fontWeight: 600, 
                  color: 'var(--gd-on-surface)',
                  whiteSpace: 'nowrap',
                  overflow: 'hidden',
                  textOverflow: 'ellipsis'
                }}>
                  {metadata.title || 'Untitled Media'}
                </div>
                <div style={{ fontSize: 13, color: 'var(--gd-on-surface-variant)' }}>
                  {metadata.uploader || 'External Source'}
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginTop: 4 }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 12, color: 'var(--gd-blue)' }}>
                    <Download size={14} />
                    {metadata.filesize_approx ? formatSize(metadata.filesize_approx) : 'Unknown size'}
                  </div>
                  <div style={{ fontSize: 12, color: 'var(--gd-on-surface-variant)', padding: '2px 6px', backgroundColor: 'var(--gd-surface-container-high)', borderRadius: 4 }}>
                    {metadata.ext?.toUpperCase() || 'FILE'}
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div style={{
          padding: '16px 24px 24px',
          display: 'flex',
          justifyContent: 'flex-end',
          gap: 12,
          backgroundColor: 'var(--gd-surface-container-high)'
        }}>
          <button type="button"
            onClick={onClose}
            className="gd-btn-text"
            style={{ padding: '0 20px', height: 40, borderRadius: 20, color: 'var(--gd-on-surface)' }}
          >
            {t('common.cancel', 'Cancel')}
          </button>
          <button type="button"
            onClick={handleImport}
            disabled={!metadata || isImporting}
            className="gd-btn-filled"
            style={{
              padding: '0 24px',
              height: 40,
              borderRadius: 20,
              backgroundColor: 'var(--gd-blue)',
              color: 'white',
              fontWeight: 600,
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              opacity: (!metadata || isImporting) ? 0.6 : 1
            }}
          >
            {isImporting ? <Loader2 size={18} className="animate-spin" /> : <CheckCircle size={18} />}
            {t('import.importBtn', 'Start import')}
          </button>
        </div>
      </div>

      <style>{`
        @keyframes modal-pop {
          from { opacity: 0; transform: scale(0.95) translateY(10px); }
          to { opacity: 1; transform: scale(1) translateY(0); }
        }
        @keyframes fade-in {
          from { opacity: 0; }
          to { opacity: 1; }
        }
      `}</style>
    </div>
  );
};
