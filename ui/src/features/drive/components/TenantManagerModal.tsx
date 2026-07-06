import { useEffect, useMemo, useState } from 'react'
import { motion } from 'framer-motion'
import { X } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '../../../components/ui/be-ui-button'

function tenantKey(tenant) {
  return `${tenant.scope}__${tenant.discordGuildId || '0'}__${tenant.telegramGroupId || '0'}`
}

function fallbackTenantLabel(tenant) {
  return `D:${tenant.discordGuildId || '0'} / T:${tenant.telegramGroupId || '0'}`
}

function getStatusStyle(isActive) {
  if (isActive) {
    return {
      backgroundColor: 'rgba(37,99,235,0.16)',
      color: '#60a5fa',
    }
  }
  return {
    backgroundColor: 'var(--gd-outline-variant)',
    color: 'var(--gd-on-surface-variant)',
  }
}

function getSwitchButtonStyle(isActive) {
  if (isActive) {
    return {
      backgroundColor: 'var(--gd-outline-variant)',
      color: 'var(--gd-on-surface-variant)',
    }
  }
  return {
    backgroundColor: '#2563eb',
    color: '#fff',
  }
}

export function TenantManagerModal({
  scope,
  tenants,
  activeTenant,
  loading,
  onClose,
  onSwitchTenant,
  onRenameTenant,
  onOpenSetup,
}) {
  const { t } = useTranslation()
  const [draftNames, setDraftNames] = useState({})
  const activeKey = activeTenant ? tenantKey(activeTenant) : null

  useEffect(() => {
    const handleKeyDown = (e) => {
      if (e.key === 'Escape') {
        onClose()
      }
    }
    globalThis.addEventListener('keydown', handleKeyDown)
    return () => globalThis.removeEventListener('keydown', handleKeyDown)
  }, [onClose])

  const title = useMemo(
    () => (scope === 'shared' ? t('sidebar.sharedDrive') : t('sidebar.myDrive')),
    [scope, t]
  )

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 70,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: 24,
      }}
    >
      {/* Backdrop overlay */}
          <div
            role="button"
            tabIndex={-1}
            aria-label="Close modal"
            onClick={onClose}
            style={{
              position: 'absolute',
              inset: 0,
              backgroundColor: 'var(--gd-overlay)',
              border: 'none',
              padding: 0,
              cursor: 'default',
            }}
          />

      <motion.div
        role="dialog"
        aria-modal="true"
        initial={{ opacity: 0, y: 16, scale: 0.98 }}
        animate={{ opacity: 1, y: 0, scale: 1 }}
        exit={{ opacity: 0, y: 8, scale: 0.98 }}
        transition={{ duration: 0.16, ease: 'easeOut' }}
        style={{
          position: 'relative',
          zIndex: 1,
          width: 'min(720px, 100%)',
          maxHeight: '80vh',
          overflow: 'hidden',
          borderRadius: 20,
          border: '1px solid var(--gd-modal-border)',
          backgroundColor: 'var(--gd-modal-surface)',
          color: 'var(--gd-modal-text)',
          boxShadow: '0 24px 80px rgba(15, 23, 42, 0.22)',
        }}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            padding: '18px 20px 14px',
            borderBottom: '1px solid var(--gd-modal-border)',
          }}
        >
          <div>
            <div style={{ fontSize: 22, fontWeight: 700 }}>{title}</div>
            <div style={{ marginTop: 4, fontSize: 13, color: 'var(--gd-modal-text-secondary)' }}>
              {t(
                'tenantManager.description',
                'Quan ly danh sach DB trong scope nay va dat ten hien thi de de nhin hon.'
              )}
            </div>
          </div>
          <Button
            variant="outline"
            size="icon"
            onClick={onClose}
          >
            <X size={18} />
          </Button>
        </div>

        <div
          style={{
            padding: 20,
            display: 'flex',
            flexDirection: 'column',
            gap: 12,
            overflowY: 'auto',
            maxHeight: 'calc(80vh - 96px)',
          }}
        >
          {tenants.length === 0 ? (
            <div
              style={{
                borderRadius: 16,
                border: '1px dashed var(--gd-input-border)',
                padding: 20,
                display: 'flex',
                flexDirection: 'column',
                gap: 12,
              }}
            >
              <div style={{ fontSize: 15, fontWeight: 600 }}>
                {t('tenantManager.empty', 'Chua co DB tenant nao trong scope nay.')}
              </div>
              <Button
                variant="primary"
                size="md"
                onClick={() => onOpenSetup(scope)}
                className="self-start"
              >
                {t('tenantManager.openSetup', 'Mo setup')}
              </Button>
            </div>
          ) : (
            tenants.map((tenant) => (
              <TenantItem
                key={tenant.dbFileName || tenantKey(tenant)}
                tenant={tenant}
                activeKey={activeKey}
                draftNames={draftNames}
                setDraftNames={setDraftNames}
                onRenameTenant={onRenameTenant}
                onSwitchTenant={onSwitchTenant}
                loading={loading}
                t={t}
              />
            ))
          )}
        </div>
      </motion.div>
    </div>
  )
}

TenantManagerModal.defaultProps = {
  tenants: [],
  activeTenant: null,
  loading: false,
}

function TenantItem({
  tenant,
  activeKey,
  draftNames,
  setDraftNames,
  onRenameTenant,
  onSwitchTenant,
  loading,
  t,
}) {
  const key = tenantKey(tenant)
  const isActive = activeKey === key
  const [isSaving, setIsSaving] = useState(false)

  const statusLabel = isActive
    ? t('tenantManager.active', 'Dang dung')
    : t('tenantManager.inactive', 'San sang')

  const statusStyle = getStatusStyle(isActive)
  const switchButtonStyle = getSwitchButtonStyle(isActive)

  const draftValue = draftNames[key] ?? tenant.displayName ?? ''

  const handleSave = async () => {
    setIsSaving(true)
    try {
      await onRenameTenant(tenant, draftValue)
    } finally {
      setIsSaving(false)
    }
  }

  return (
    <div
      style={{
        borderRadius: 16,
        border: '1px solid var(--gd-modal-border)',
        backgroundColor: 'var(--gd-modal-close-bg)',
        padding: 16,
        display: 'flex',
        flexDirection: 'column',
        gap: 12,
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: 12,
        }}
      >
        <div>
          <div style={{ fontSize: 14, fontWeight: 700 }}>
            {tenant.displayName || fallbackTenantLabel(tenant)}
          </div>
          <div
            style={{
              marginTop: 4,
              fontSize: 12,
              color: 'var(--gd-modal-text-secondary)',
            }}
          >
            {fallbackTenantLabel(tenant)} · {tenant.dbFileName}
          </div>
        </div>
        <div
          style={{
            padding: '4px 10px',
            borderRadius: 999,
            fontSize: 12,
            fontWeight: 700,
            ...statusStyle,
          }}
        >
          {statusLabel}
        </div>
      </div>
      <div style={{ display: 'flex', gap: 10, flexWrap: 'wrap' }}>
        <input
          type="text"
          value={draftValue}
          onChange={(event) =>
            setDraftNames((current) => ({
              ...current,
              [key]: event.target.value,
            }))
          }
          placeholder={t('tenantManager.placeholder', 'Ten hien thi tuy chinh')}
          style={{
            flex: '1 1 280px',
            minWidth: 0,
            borderRadius: 12,
            border: '1px solid var(--gd-input-border)',
            backgroundColor: 'var(--gd-input-bg)',
            color: 'inherit',
            padding: '10px 12px',
            fontSize: 14,
          }}
        />
        <Button
          variant="outline"
          size="md"
          disabled={loading || isSaving}
          onClick={handleSave}
        >
          {isSaving ? t('tenantManager.saving', 'Dang luu...') : t('tenantManager.save', 'Luu ten')}
        </Button>
        <Button
          variant={isActive ? "ghost" : "primary"}
          size="md"
          disabled={loading || isActive}
          onClick={() => onSwitchTenant(tenant)}
        >
          {isActive ? t('tenantManager.current', 'Dang chon') : t('tenantManager.useThis', 'Dung DB nay')}
        </Button>
      </div>
    </div>
  )
}

export { TenantItem }



