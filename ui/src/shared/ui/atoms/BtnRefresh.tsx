
import { RefreshCw } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export const BtnRefresh = ({ onClick, loading, title }: { onClick: any; loading?: any; title?: any }) => {
  const { t } = useTranslation();
  const resolvedTitle = title || t('common.refresh');

  return (
    <button type="button" 
      onClick={onClick} 
      title={resolvedTitle}
      className="gd-icon-btn"
      id="header-refresh-btn"
      style={loading ? { animation: 'spin 1s linear infinite' } : {}}
    >
      <RefreshCw size={20} />
    </button>
  );
};
