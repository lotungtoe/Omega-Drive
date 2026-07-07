
import { useTranslation } from 'react-i18next';
import { SortButton } from '../../../../shared/ui/atoms/SortButton';

export const ListHeader = ({ sort, setSort, dark, isShared }) => {
  const { t } = useTranslation();

  return (
    <div style={{
      display: 'flex',
      alignItems: 'center',
      gap: 12,
      padding: '0 16px',
      borderBottom: '1px solid var(--gd-outline-variant)',
      fontSize: 12,
      fontWeight: 500,
      color: 'var(--gd-on-surface-variant)',
    }}>
      <div style={{ width: 36, flexShrink: 0 }} />
      <div style={{ flex: 1, minWidth: 0, paddingLeft: 8 }}>
        <SortButton label={t('sort.name')} field="name" sort={sort} setSort={setSort} align="left" />
      </div>
      {isShared && (
        <div style={{ width: 180, flexShrink: 0, display: 'flex', justifyContent: 'flex-start', paddingLeft: 12 }} className="hidden lg:flex">
          <span style={{ fontSize: 12, fontWeight: 500, color: 'var(--gd-on-surface-variant)' }}>{t('drive.sharer')}</span>
        </div>
      )}
      <div style={{ width: 100, flexShrink: 0, display: 'flex', justifyContent: 'center' }} className="hidden lg:flex">
        <SortButton label={t('sort.type')} field="type" sort={sort} setSort={setSort} />
      </div>
      <div style={{ width: 160, flexShrink: 0, display: 'flex', justifyContent: 'center' }} className="hidden md:flex">
        <SortButton label={t('sort.modified')} field="date" sort={sort} setSort={setSort} />
      </div>
      <div style={{ width: 100, flexShrink: 0, display: 'flex', justifyContent: 'center' }} className="hidden sm:flex">
        <SortButton label={t('sort.size')} field="size" sort={sort} setSort={setSort} />
      </div>
      <div style={{ width: 120, flexShrink: 0 }} />
    </div>
  );
};
