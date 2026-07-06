
import { Upload } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export const BtnUpload = ({ onClick }) => {
  const { t } = useTranslation();

  return (
    <button type="button" 
      onClick={onClick}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 6,
        padding: '8px 20px',
        backgroundColor: 'var(--gd-blue)',
        color: 'white',
        borderRadius: 'var(--gd-radius-full)',
        border: 'none',
        fontFamily: "'Google Sans', sans-serif",
        fontSize: 14,
        fontWeight: 500,
        cursor: 'pointer',
        transition: 'background-color 0.15s',
        boxShadow: 'var(--gd-shadow-1)'
      }}
      onMouseEnter={e => e.currentTarget.style.backgroundColor = 'var(--gd-blue-hover)'}
      onMouseLeave={e => e.currentTarget.style.backgroundColor = 'var(--gd-blue)'}
      id="app-upload-btn"
    >
      <Upload size={16} />
      {t('common.upload')}
    </button>
  );
};
