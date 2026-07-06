
import { Plus } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { DropdownSelect } from '../../../shared/components/DropdownSelect'

function tenantKey(tenant) {
  return `${tenant.scope}__${tenant.discordGuildId || '0'}__${tenant.telegramGroupId || '0'}`
}

function tenantLabel(tenant) {
  if (tenant.displayName && tenant.displayName.trim().length > 0) {
    return tenant.displayName.trim()
  }
  return `D:${tenant.discordGuildId || '0'} / T:${tenant.telegramGroupId || '0'}`
}

export function TenantScopeDropdown({
  scope,
  tenants,
  activeTenant,
  loading,
  disabled,
  onSelectTenant,
  onOpenManager,
  onOpenSetup,
}) {
  const { t } = useTranslation()
  const knownTenants = [...tenants]
  if (activeTenant && !knownTenants.some((tenant) => tenantKey(tenant) === tenantKey(activeTenant))) {
    knownTenants.unshift(activeTenant)
  }

  const selectedValue = activeTenant ? tenantKey(activeTenant) : '__setup__'

  return (
    <div
      style={{
        marginBottom: 16,
        width: 'fit-content',
        display: 'flex',
        alignItems: 'center',
        gap: 8,
      }}
    >
<DropdownSelect 
        value={selectedValue}
        disabled={disabled || loading}
        onDoubleClick={() => onOpenManager?.()}
        onChange={(nextValue) => {
          const nextTenant = knownTenants.find((tenant) => tenantKey(tenant) === nextValue)
          if (nextTenant) onSelectTenant(nextTenant)
        }}
        options={
          knownTenants.length > 0
            ? knownTenants.map((tenant) => ({
                value: tenantKey(tenant),
                label: tenantLabel(tenant),
              }))
            : [
                {
                  value: '__setup__',
                  label: loading
                    ? t('tenantDropdown.loading', 'Dang tai...')
                    : t('tenantDropdown.emptyCompact', 'Chua co DB'),
                },
              ]
        }
        style={{ width: 220, maxWidth: '100%', fontWeight: 600 }}
      />
      <button
        type="button"
        onClick={() => onOpenSetup(scope)}
        disabled={disabled || loading}
        title={t('tenantDropdown.setupCompact', 'Mo setup')}
        aria-label={t('tenantDropdown.setupCompact', 'Mo setup')}
        style={{
          width: 40,
          height: 40,
          borderRadius: 12,
          border: '1px solid var(--gd-outline-variant)',
          backgroundColor: 'var(--gd-surface)',
          color: 'var(--gd-on-surface)',
          display: 'inline-flex',
          alignItems: 'center',
          justifyContent: 'center',
          cursor: disabled || loading ? 'not-allowed' : 'pointer',
          opacity: disabled || loading ? 0.5 : 1,
        }}
      >
        <Plus size={18} />
      </button>
    </div>
  )
}

TenantScopeDropdown.defaultProps = {
  tenants: [],
  activeTenant: null,
  loading: false,
  disabled: false,
  onOpenManager: undefined,
}

