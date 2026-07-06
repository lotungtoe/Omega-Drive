import { Grid3X3, List as ListIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export const BtnViewToggle = ({ view, setView }) => {
  const { t } = useTranslation();

  return (
    <div style={{ 
      display: 'flex', 
      alignItems: 'center', 
      border: '1px solid var(--gd-outline)',
      borderRadius: 'var(--gd-radius-sm)',
      overflow: 'hidden'
    }}>
      {[
        { id: 'grid', Icon: Grid3X3, title: t('common.viewGrid') },
        { id: 'list', Icon: ListIcon, title: t('common.viewList') }
      ].map(
        ({ id, Icon, title }) => (
        <button type="button"
          key={id}
          onClick={() => setView(id)}
          className="gd-icon-btn"
          style={{
            width: 36,
            height: 36,
            borderRadius: 0,
            backgroundColor: view === id ? 'var(--gd-blue-surface)' : 'transparent',
            color: view === id ? 'var(--gd-blue)' : 'var(--gd-on-surface-variant)',
          }}
          title={title}
        >
          <Icon size={18} />
        </button>
      ))}
    </div>
  );
};
