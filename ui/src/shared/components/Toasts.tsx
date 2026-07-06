import { useState, useCallback, createContext, useContext } from 'react'
import { CheckCircle2, AlertCircle, Info, X } from 'lucide-react'
import { Button } from '../../components/ui/be-ui-button'

// eslint-disable-next-line react-refresh/only-export-components
export const ToastCtx = createContext({ show: () => {} })
// eslint-disable-next-line react-refresh/only-export-components
export const useToast = () => useContext(ToastCtx)

// eslint-disable-next-line react-refresh/only-export-components
export function useToastState() {
  const [toasts, setToasts] = useState([])
  const remove = (id) => setToasts(p => p.filter(t => t.id !== id))
  const show = useCallback((message, type = 'info', duration = 3000, actions = null) => {
    const id = Date.now() + Math.random()
    setToasts(p => [...p, { id, message, type, actions }].slice(-5))
    setTimeout(() => remove(id), duration)
  }, [])
  return { toasts, show, remove }
}

export function ToastContainer({ toasts, remove }) {
  return (
    <div style={{
      position: 'fixed', bottom: 24, left: '50%', transform: 'translateX(-50%)',
      zIndex: 200, display: 'flex', flexDirection: 'column', gap: 8, pointerEvents: 'none'
    }}>
      {toasts.map(t => {
        const token = t.type === 'success' ? 'success' : t.type === 'error' ? 'error' : 'info'
        return (
          <div
            key={t.id}
            style={{
              pointerEvents: 'auto', display: 'flex', alignItems: 'center', gap: 12,
              padding: '12px 16px', borderRadius: '4px', boxShadow: 'var(--gd-shadow-2)',
              fontSize: 14, fontFamily: "'Google Sans', sans-serif", fontWeight: 500,
              minWidth: 280, maxWidth: 500,
              backgroundColor: `var(--gd-toast-${token}-bg)`,
              color: `var(--gd-toast-${token}-text)`,
              border: `1px solid var(--gd-toast-${token}-border)`,
              animation: 'toast-in 0.3s ease-out forwards'
            }}
          >
            {t.type === 'success' && <CheckCircle2 size={18} />}
            {t.type === 'error' && <AlertCircle size={18} />}
            {t.type === 'info' && <Info size={18} />}
            <span style={{ flex: 1 }}>{t.message}</span>
            {Array.isArray(t.actions) && t.actions.length > 0 && (
              <div style={{ display: 'flex', gap: 8 }}>
                {t.actions.map((action, idx) => (
                  <Button
                    key={`${t.id}-action-${idx}`}
                    variant="ghost"
                    size="sm"
                    onClick={() => {
                      action.onClick?.()
                      remove(t.id)
                    }}
                    style={{ fontSize: 12 }}
                  >
                    {action.label}
                  </Button>
                ))}
              </div>
            )}
            <Button variant="ghost" size="sm" onClick={() => remove(t.id)} style={{ opacity: 0.7 }}>
              <X size={16} />
            </Button>
          </div>
        )
      })}
    </div>
  )
}

