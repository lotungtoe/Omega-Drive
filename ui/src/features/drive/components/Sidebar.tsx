import { memo, useContext, useEffect, useState } from 'react'
import { useDroppable } from '@dnd-kit/core'
import { Home, Clock, Star, Trash2, HardDrive, Plus, Settings, ArrowDownUp, Users, Link } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { formatSize, cn } from '../../../shared/utils/index'
import { invoke } from '@tauri-apps/api/core'
import {
  DRIVE_SECTION_HOME,
  DRIVE_SECTION_MY,
  DRIVE_SECTION_RECENT,
  DRIVE_SECTION_SHARED,
  DRIVE_SECTION_STARRED,
  DRIVE_SECTION_TRANSFERS,
  DRIVE_SECTION_TRASH,
} from '../hooks/driveSections'
import {
  DriveControllerContext,
  MainAppUiActionsContext,
  MainAppUiStateContext,
} from '../pages/useMainAppContext'

function tenantKey(tenant) {
  if (!tenant) {
    return ''
  }
  return `${tenant.scope || 'my'}__${tenant.discordGuildId || '0'}__${tenant.telegramGroupId || '0'}`
}

function DroppableNavItem({ item, active }) {
  const isDropTarget = item.id === DRIVE_SECTION_MY || item.id === DRIVE_SECTION_SHARED
  const { setNodeRef, isOver } = useDroppable({
    id: `sidebar-${item.id}`,
    disabled: !isDropTarget,
    data: { type: 'sidebar', targetFolderId: null, targetScope: item.scope ?? null },
  })

  return (
    <button type="button"
      ref={isDropTarget ? setNodeRef : undefined}
      onClick={item.onClick}
      className={cn('gd-nav-item', active && 'active', isOver && 'ring-2 ring-blue-400')}
      style={{
        backgroundColor: isOver ? 'var(--gd-blue-surface)' : undefined,
        transition: 'background-color 0.15s',
      }}
      id={`sidebar-nav-${item.id}`}
    >
      <item.icon size={20} />
      {item.label}
    </button>
  )
}

export const Sidebar = memo(function Sidebar(props: any) {
  const {
    currentFolderId,
    setCurrentFolderId,
    stats,
    activeSection,
    setActiveSection,
    onNewClick,
    onSettingsClick,
  } = props
  const { t } = useTranslation()
  const uiState = useContext(MainAppUiStateContext)
  const uiActions = useContext(MainAppUiActionsContext)
  const driveController = useContext(DriveControllerContext)
  const [activeTenants, setActiveTenants] = useState({ my: null, shared: null, current: null })
  const [tenantStateLoaded, setTenantStateLoaded] = useState(false)

  const resolvedCurrentFolderId = currentFolderId ?? driveController?.currentFolderId ?? null
  const resolvedSetCurrentFolderId = setCurrentFolderId ?? driveController?.setCurrentFolderId ?? (() => {})
  const resolvedStats = stats ?? driveController?.stats ?? {}
  const resolvedActiveSection = activeSection ?? uiState?.activeSection ?? 'home'
  const resolvedSetActiveSection = setActiveSection ?? uiActions?.setActiveSection ?? (() => {})
  const resolvedOnNewClick = onNewClick ?? uiActions?.openNewFolder ?? (() => {})
  const resolvedOnSettingsClick = onSettingsClick ?? uiActions?.openSettings ?? (() => {})

  type TenantState = { my: any; shared: any; current: any; scope?: string }

  const fetchTenantState = async () => {
    const rememberedTenants = await invoke('get_active_tenants') as TenantState | null
    return rememberedTenants || { my: null, shared: null, current: null }
  }

  useEffect(() => {
    if (uiState?.onboardingVisible) {
      return undefined
    }
    let cancelled = false
    void fetchTenantState()
      .then((nextState) => {
        if (cancelled) return
        setActiveTenants(nextState)
        setTenantStateLoaded(true)
      })
      .catch((err) => {
        if (!cancelled) {
          console.error('[Sidebar] Failed to load active tenants:', err)
          setTenantStateLoaded(true)
        }
      })

    return () => {
      cancelled = true
    }
  }, [uiState?.onboardingVisible])

  const switchTenant = async (tenant) => {
    try {
      const switched = await invoke('switch_tenant', { tenant }) as TenantState | null
      const refreshedTenants = await fetchTenantState()
      setActiveTenants({
        my: refreshedTenants?.my ?? null,
        shared: refreshedTenants?.shared ?? null,
        current: refreshedTenants?.current || switched || null,
      })
      setTenantStateLoaded(true)
      resolvedSetCurrentFolderId(null)
      resolvedSetActiveSection(switched?.scope === 'shared' ? DRIVE_SECTION_SHARED : DRIVE_SECTION_MY)
    } catch (err) {
      console.error('[Sidebar] Failed to switch tenant:', err)
    }
  }

  const activateScopeWithoutTenant = (_scope, fallbackSection) => {
    resolvedSetActiveSection(fallbackSection)
    resolvedSetCurrentFolderId(null)
  }

  const activateRememberedScopeTenant = async (scope, fallbackSection) => {
    activateScopeWithoutTenant(scope, fallbackSection)

    let tenant = activeTenants?.[scope] || null
    if (!tenant && !tenantStateLoaded) {
      try {
        const refreshedTenants = await fetchTenantState()
        setActiveTenants(refreshedTenants)
        setTenantStateLoaded(true)
        tenant = refreshedTenants?.[scope] || null
      } catch (err) {
        console.error('[Sidebar] Failed to refresh tenant state before scope activation:', err)
        setTenantStateLoaded(true)
      }
    }

    if (!tenant) {
      return
    }

    if (tenantKey(activeTenants?.current) === tenantKey(tenant)) {
      return
    }

    void switchTenant(tenant)
  }

  const navItems = [
    {
      icon: Home,
      label: t('sidebar.home'),
      id: DRIVE_SECTION_HOME,
      onClick: () => {
        resolvedSetActiveSection(DRIVE_SECTION_HOME)
        resolvedSetCurrentFolderId(null)
      },
    },
    {
      icon: HardDrive,
      label: t('sidebar.myDrive'),
      id: DRIVE_SECTION_MY,
      scope: 'my',
      onClick: () => {
        void activateRememberedScopeTenant('my', DRIVE_SECTION_MY)
      },
    },
    {
      icon: Users,
      label: t('sidebar.sharedDrive'),
      id: DRIVE_SECTION_SHARED,
      scope: 'shared',
      onClick: () => {
        void activateRememberedScopeTenant('shared', DRIVE_SECTION_SHARED)
      },
    },
    {
      icon: Clock,
      label: t('sidebar.recent'),
      id: DRIVE_SECTION_RECENT,
      onClick: () => {
        resolvedSetActiveSection(DRIVE_SECTION_RECENT)
        resolvedSetCurrentFolderId(null)
      },
    },
    {
      icon: Star,
      label: t('sidebar.starred'),
      id: DRIVE_SECTION_STARRED,
      onClick: () => {
        resolvedSetActiveSection(DRIVE_SECTION_STARRED)
        resolvedSetCurrentFolderId(null)
      },
    },
    {
      icon: ArrowDownUp,
      label: t('sidebar.transfers'),
      id: DRIVE_SECTION_TRANSFERS,
      onClick: () => {
        resolvedSetActiveSection(DRIVE_SECTION_TRANSFERS)
        resolvedSetCurrentFolderId(null)
      },
    },
    {
      icon: Trash2,
      label: t('sidebar.trash'),
      id: DRIVE_SECTION_TRASH,
      onClick: () => {
        resolvedSetActiveSection(DRIVE_SECTION_TRASH)
        resolvedSetCurrentFolderId(null)
      },
    },
  ]

  const usedBytes = resolvedStats.total_size || 0

  return (
    <aside
      style={{
        width: 'var(--gd-sidebar-width)',
        backgroundColor: 'var(--gd-sidebar-bg)',
        borderRight: '1px solid var(--gd-outline-variant)',
        display: 'flex',
        flexDirection: 'column',
        flexShrink: 0,
        userSelect: 'none',
        height: '100%',
        overflow: 'hidden',
      }}
    >
      <div style={{ padding: '16px 16px 8px 16px', display: 'flex', flexDirection: 'column', gap: 12 }}>
        <button type="button" className="gd-fab" onClick={resolvedOnNewClick} id="sidebar-new-button">
          <Plus size={24} style={{ color: 'var(--gd-on-surface)' }} />
          <span>{t('sidebar.newButton')}</span>
        </button>
        <button type="button"
          className="gd-fab"
          onClick={uiActions.openUrlImport}
          id="sidebar-url-import-button"
          style={{
            backgroundColor: 'var(--gd-surface-container-high)',
            boxShadow: 'none',
            border: '1px solid var(--gd-outline-variant)',
          }}
        >
          <Link size={20} style={{ color: 'var(--gd-blue)' }} />
          <span style={{ fontSize: 13, fontWeight: 500 }}>{t('sidebar.importUrl', 'Nhap tu URL')}</span>
        </button>
      </div>

      <nav style={{ padding: '4px 0', flex: '0 0 auto', display: 'flex', flexDirection: 'column', gap: 1 }}>
        {navItems.map((item) => {
          const active = resolvedActiveSection === item.id && !resolvedCurrentFolderId
          return <DroppableNavItem key={item.id} item={item} active={active} />
        })}
      </nav>

      <div style={{ flex: 1 }} />

      <div style={{ height: 1, backgroundColor: 'var(--gd-outline-variant)' }} />

      <div style={{ padding: '8px 0' }}>
        <button type="button" onClick={resolvedOnSettingsClick} className="gd-nav-item" id="sidebar-settings-btn">
          <Settings size={20} />
          {t('sidebar.settings')}
        </button>
      </div>

      <div
        style={{
          padding: '12px 24px 20px',
          borderTop: '1px solid var(--gd-outline-variant)',
        }}
      >
        <div style={{ fontSize: 13, color: 'var(--gd-on-surface-variant)' }}>
          {t('sidebar.storageUsage', { used: formatSize(usedBytes) })}
        </div>
        <div
          style={{
            fontSize: 12,
            color: 'var(--gd-on-surface-variant)',
            marginTop: 4,
          }}
        >
          {t('sidebar.itemCount', { files: resolvedStats.file_count || 0, folders: resolvedStats.folder_count || 0 })}
        </div>
      </div>
    </aside>
  )
})
