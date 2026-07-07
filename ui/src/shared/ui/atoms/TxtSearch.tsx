import { Search } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export const TxtSearch = ({ value, onChange, dark: _dark }: { value: any; onChange: any; dark?: any }) => {
  const { t } = useTranslation();

  return (
    <div className="gd-search-container" style={{
      display: 'flex',
      alignItems: 'center',
      backgroundColor: 'var(--gd-surface-variant)',
      borderRadius: 'var(--gd-radius-lg)',
      padding: '0 16px',
      width: '100%',
      maxWidth: 720,
      height: 48,
      transition: 'all 0.2s',
      border: '1px solid transparent'
    }}>
      <Search size={20} style={{ color: 'var(--gd-on-surface-variant)', marginRight: 12 }} />
      <input 
        id="header-search-input"
        type="text" 
        placeholder={t('header.searchPlaceholder')}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        style={{
          flex: 1,
          border: 'none',
          background: 'transparent',
          color: 'var(--gd-on-surface)',
          fontSize: 16,
          outline: 'none',
        }}
      />
    </div>
  );
};
