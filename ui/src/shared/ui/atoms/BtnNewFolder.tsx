
import { FolderPlus } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Button } from '../../../components/ui/be-ui-button';

export const BtnNewFolder = ({ onClick }) => {
  const { t } = useTranslation();

  return (
    <Button variant="ghost" size="sm" onClick={onClick} title={t('common.newFolder')}>
      <FolderPlus size={16} />
      <span className="hidden sm:inline">{t('common.newFolder')}</span>
    </Button>
  );
};
