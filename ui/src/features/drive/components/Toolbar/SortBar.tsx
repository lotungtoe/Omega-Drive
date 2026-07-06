
import { ArrowUp, ArrowDown } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export const SortBar = ({ sort, setSort }) => {
  const { t } = useTranslation();
  const options = [
    { label: t('sort.name'), field: 'name' },
    { label: t('sort.type'), field: 'type' },
    { label: t('sort.date'), field: 'date' },
    { label: t('sort.size'), field: 'size' },
  ];

  return (
    <div style={{ 
      display: 'flex', 
      alignItems: 'center', 
      gap: 16, 
      marginBottom: 12, 
      paddingBottom: 8, 
      borderBottom: '1px solid var(--gd-outline-variant)' 
    }}>
      {options.map(s => (
        <button type="button" 
          key={s.field} 
          onClick={() => setSort(prev => prev.field === s.field ? { field: s.field, dir: prev.dir === 'asc' ? 'desc' : 'asc' } : { field: s.field, dir: 'asc' })}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 4,
            padding: '4px 8px',
            borderRadius: 'var(--gd-radius-sm)',
            fontSize: 13,
            fontFamily: "'Google Sans', 'Roboto', sans-serif",
            fontWeight: sort.field === s.field ? 600 : 400,
            color: sort.field === s.field ? 'var(--gd-blue)' : 'var(--gd-on-surface-variant)',
            background: sort.field === s.field ? 'var(--gd-blue-surface)' : 'transparent',
            border: 'none',
            cursor: 'pointer',
            transition: 'all 0.15s'
          }}
        >
          {s.label}
          {sort.field === s.field && (sort.dir === 'asc' ? <ArrowUp size={14} /> : <ArrowDown size={14} />)}
        </button>
      ))}
    </div>
  );
};
