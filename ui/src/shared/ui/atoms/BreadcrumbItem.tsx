
import { ChevronRight } from 'lucide-react';
import { Button } from '../../../components/ui/be-ui-button';

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
