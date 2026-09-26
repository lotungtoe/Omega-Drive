import { ArrowDown, ArrowUp, ArrowUpDown, Bot, ChevronRight, FolderPlus, Grid3X3, List as ListIcon, Moon, RefreshCw, Search, Sun, Upload } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Button } from '../../../components/ui/be-ui-button';
import { cn } from '../../utils/index';

export const BtnNewFolder = ({ onClick }) => {
  const { t } = useTranslation();

  return (
    <Button variant="ghost" size="sm" onClick={onClick} title={t('common.newFolder')}>
      <FolderPlus size={16} />
      <span className="hidden sm:inline">{t('common.newFolder')}</span>
    </Button>
  );
};

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

export const BtnSync = ({ onClick, title }: { onClick: any; title?: any }) => {
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

export function BtnThemeToggle({ dark, setDark }) {
  return (
    <button
      type="button"
      className={cn(
        'flex w-16 h-8 p-1 rounded-full cursor-pointer transition-colors duration-300',
        dark ? 'bg-zinc-950 border border-zinc-800' : 'bg-white border border-zinc-200',
      )}
      onClick={() => setDark(!dark)}
    >
      <div className="flex justify-between items-center w-full">
        <div className={cn(
          'flex justify-center items-center w-6 h-6 rounded-full transition-transform duration-300',
          dark ? 'transform translate-x-0 bg-zinc-800' : 'transform translate-x-8 bg-gray-200',
        )}>
          {dark
            ? <Moon className="w-4 h-4 text-white" strokeWidth={1.5} />
            : <Sun className="w-4 h-4 text-gray-700" strokeWidth={1.5} />
          }
        </div>
        <div className={cn(
          'flex justify-center items-center w-6 h-6 rounded-full transition-transform duration-300',
          dark ? 'bg-transparent' : 'transform -translate-x-8',
        )}>
          {dark
            ? <Sun className="w-4 h-4 text-gray-500" strokeWidth={1.5} />
            : <Moon className="w-4 h-4 text-black" strokeWidth={1.5} />
          }
        </div>
      </div>
    </button>
  )
}

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

export const BreadcrumbItem = ({ label, isLast, onClick, active }) => {
  return (
    <span style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
      {!active && <ChevronRight size={18} style={{ color: 'var(--gd-on-surface-variant)' }} />}
      <Button variant="ghost" size="sm" onClick={onClick} style={{ fontSize: isLast ? 18 : 14 }}>
        {label}
      </Button>
    </span>
  );
};

export const Logo = ({ size = 40, dark }) => {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: '12px', minWidth: '180px' }}>
      <div style={{ position: 'relative', width: size, height: size, flexShrink: 0 }}>
        <img
          src="/black_cat.png"
          alt="Logo"
          style={{
            position: 'absolute',
            inset: 0,
            width: '100%',
            height: '100%',
            objectFit: 'contain',
            opacity: dark ? 0 : 1,
            transition: 'opacity 0.3s ease'
          }}
        />
        <img
          src="/white_cat.png"
          alt="Logo"
          style={{
            position: 'absolute',
            inset: 0,
            width: '100%',
            height: '100%',
            objectFit: 'contain',
            opacity: dark ? 1 : 0,
            transition: 'opacity 0.3s ease'
          }}
        />
      </div>
      <span style={{
        fontFamily: "'Google Sans', sans-serif",
        fontSize: '22px',
        color: 'var(--gd-on-surface-variant)',
        fontWeight: 400,
        letterSpacing: '-0.5px'
      }}>
        Drive
      </span>
    </div>
  );
};

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
      transition: 'border-color 0.2s, background-color 0.2s, box-shadow 0.2s',
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
