import { useTranslation } from 'react-i18next';
import { Settings } from 'lucide-react';

export const BtnSettings = ({ onClick, title }) => {
  const { t } = useTranslation();
  const label = title || t('settings.title');

  return (
    <button type="button"
      onClick={onClick}
      title={label}
      className="gd-icon-btn"
      id="header-settings-btn"
    >
      <Settings size={20} />
    </button>
  );
};
