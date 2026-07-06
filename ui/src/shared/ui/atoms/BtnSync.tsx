
import { Bot } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export const BtnSync = ({ onClick, title }) => {
  const { t } = useTranslation();
  const resolvedTitle = title || t('header.sync');

  return (
    <button type="button" 
      onClick={onClick} 
      title={resolvedTitle}
      className="gd-icon-btn"
      id="header-sync-btn"
    >
      <Bot size={20} />
    </button>
  );
};
