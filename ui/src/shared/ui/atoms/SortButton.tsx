import { ArrowUp, ArrowDown, ArrowUpDown } from 'lucide-react';

export const SortButton = ({ label, field, sort, setSort, align = 'center' }) => {
  const isActive = sort.field === field;
  const toggle = () => setSort(s => s.field === field ? { field, dir: s.dir === 'asc' ? 'desc' : 'asc' } : { field, dir: 'asc' });
  
  const isCenter = align === 'center';

  return (
    <button type="button" onClick={toggle} style={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: isCenter ? 'center' : 'flex-start',
      gap: 4,
      fontSize: 12,
      fontFamily: "'Google Sans', 'Roboto', sans-serif",
      fontWeight: 500,
      color: isActive ? 'var(--gd-blue)' : 'var(--gd-on-surface-variant)',
      background: 'none',
      border: 'none',
      cursor: 'pointer',
      padding: '4px 0',
      transition: 'color 0.15s',
      width: '100%',
    }}>
      {/* Add a spacer only for centered alignment. */}
      {isCenter && <div style={{ width: 14 + 4, flexShrink: 0 }} aria-hidden="true" />}
      
      <span style={{ flexShrink: 0 }}>{label}</span>
      
      <div style={{ width: 14, height: 14, display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0 }}>
        {isActive && sort.dir === 'asc' && <ArrowUp size={14} />}
        {isActive && sort.dir !== 'asc' && <ArrowDown size={14} />}
        {!isActive && <ArrowUpDown size={14} style={{ opacity: 0.4 }} />}
      </div>
    </button>
  );
};
