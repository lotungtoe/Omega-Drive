import { cn } from '../utils/index'

export function ToggleSwitch({ checked, onChange, disabled }) {
  return (
    <button
      type="button"
      onClick={() => { if (!disabled) onChange(!checked) }}
      className={cn('gd-toggle', checked && 'active')}
      role="switch"
      aria-checked={checked}
      disabled={disabled}
      style={checked ? { backgroundColor: '#3b82f6' } : undefined}
    >
      <span className="gd-toggle-thumb" />
    </button>
  )
}

