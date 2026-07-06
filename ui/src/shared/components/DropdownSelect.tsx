import { useState, useRef, useEffect } from 'react'
import { createPortal } from 'react-dom'
import { ChevronDown } from 'lucide-react'

export function DropdownSelect({ value, onChange, options, placeholder = '', disabled, style, onDoubleClick }) {
  const [open, setOpen] = useState(false)
  const [menuStyle, setMenuStyle] = useState({ top: 0, left: 0, width: 0 })
  const ref = useRef(null)
  const buttonRef = useRef(null)
  const menuRef = useRef(null)
  const selected = options.find(o => o.value === value)

  useEffect(() => {
    const handler = (e) => {
      if (ref.current && !ref.current.contains(e.target)) {
        if (menuRef.current && menuRef.current.contains(e.target)) return
        setOpen(false)
      }
    }
    document.addEventListener('mousedown', handler)
    return () => document.removeEventListener('mousedown', handler)
  }, [])

  useEffect(() => {
    if (!open) return
    const handler = (e) => {
      if (e.key === 'Escape') setOpen(false)
    }
    document.addEventListener('keydown', handler)
    return () => document.removeEventListener('keydown', handler)
  }, [open])

  const measure = () => {
    if (buttonRef.current) {
      const rect = buttonRef.current.getBoundingClientRect()
      setMenuStyle({
        top: rect.bottom + 4,
        left: rect.left,
        width: rect.width,
      })
    }
  }

  const handleOpen = () => {
    if (disabled) return
    if (open) {
      setOpen(false)
      return
    }
    measure()
    setOpen(true)
  }

  useEffect(() => {
    if (!open) return
    const handler = () => measure()
    document.addEventListener('scroll', handler, true)
    window.addEventListener('resize', handler)
    return () => {
      document.removeEventListener('scroll', handler, true)
      window.removeEventListener('resize', handler)
    }
  }, [open])

  return (
    <div ref={ref} style={{ position: 'relative', ...style }}>
      <button
        ref={buttonRef}
        type="button"
        disabled={disabled}
        onClick={handleOpen}
        onDoubleClick={onDoubleClick}
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: 8,
          width: '100%',
          padding: '10px 12px',
          borderRadius: 12,
          border: '1px solid var(--gd-input-border)',
          background: disabled ? 'var(--gd-surface-variant)' : 'var(--gd-input-bg)',
          color: 'var(--gd-modal-text)',
          fontSize: 14,
          cursor: disabled ? 'not-allowed' : 'pointer',
          opacity: disabled ? 0.5 : 1,
          outline: 'none',
          whiteSpace: 'nowrap',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
        }}
      >
        <span style={{ overflow: 'hidden', textOverflow: 'ellipsis' }}>
          {selected ? selected.label : placeholder || ''}
        </span>
        <ChevronDown
          size={16}
          style={{
            transition: 'transform 0.2s',
            transform: open ? 'rotate(180deg)' : 'rotate(0deg)',
            flexShrink: 0,
          }}
        />
      </button>
      {open && createPortal(
        <div
          ref={menuRef}
          style={{
            position: 'fixed',
            top: menuStyle.top,
            left: menuStyle.left,
            width: menuStyle.width,
            zIndex: 99999,
            marginTop: 0,
            borderRadius: 12,
            overflow: 'hidden',
            background: 'var(--gd-surface)',
            boxShadow: 'var(--gd-shadow-3)',
            border: '1px solid var(--gd-outline-variant)',
          }}
        >
          {options.map((opt) => (
            <button
              key={opt.value}
              type="button"
              onClick={() => {
                onChange(opt.value)
                setOpen(false)
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = opt.value === value ? 'var(--gd-sidebar-active-bg)' : 'var(--gd-surface-variant)'
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background =
                  opt.value === value ? 'var(--gd-sidebar-active-bg)' : 'var(--gd-surface)'
              }}
              style={{
                display: 'block',
                width: '100%',
                padding: '10px 12px',
                border: 'none',
                background: opt.value === value ? 'var(--gd-sidebar-active-bg)' : 'var(--gd-surface)',
                color: 'var(--gd-on-surface)',
                fontSize: 14,
                cursor: 'pointer',
                textAlign: 'left',
                whiteSpace: 'nowrap',
                overflow: 'hidden',
                textOverflow: 'ellipsis',
              }}
            >
              {opt.label}
            </button>
          ))}
        </div>,
        document.body
      )}
    </div>
  )
}

