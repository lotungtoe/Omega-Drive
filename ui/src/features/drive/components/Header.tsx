import { memo, useContext } from 'react'
import { Menu } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { TxtSearch } from '../../../shared/ui/atoms/TxtSearch'
import { Button } from '../../../components/ui/be-ui-button'
import { BtnThemeToggle } from '../../../shared/ui/atoms/BtnThemeToggle'
import { BtnSync } from '../../../shared/ui/atoms/BtnSync'
import { BtnRefresh } from '../../../shared/ui/atoms/BtnRefresh'
import { Logo } from '../../../shared/ui/atoms/Logo'
import {
  DriveControllerContext,
  MainAppUiActionsContext,
  MainAppUiStateContext,
} from '../pages/useMainAppContext'

const DiscordIcon = ({ size = 24, color = "currentColor", ...props }) => (
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 127.14 96.36" width={size} height={size} fill={color} {...props}>
    <path d="M107.7,8.07A105.15,105.15,0,0,0,81.47,0a72.06,72.06,0,0,0-3.36,6.83A97.68,97.68,0,0,0,49,6.83,72.37,72.37,0,0,0,45.64,0,105.89,105.89,0,0,0,19.39,8.09C2.79,32.65-1.71,56.6.54,80.21h0A105.73,105.73,0,0,0,32.71,96.36,77.7,77.7,0,0,0,39.6,85.25a68.42,68.42,0,0,1-10.85-5.18c.91-.66,1.8-1.34,2.66-2a75.57,75.57,0,0,0,64.32,0c.87.71,1.76,1.39,2.66,2a68.68,68.68,0,0,1-10.87,5.19,77,77,0,0,0,6.89,11.1A105.25,105.25,0,0,0,126.6,80.22h0C129.24,52.84,122.09,29.11,107.7,8.07ZM42.45,65.69C36.18,65.69,31,60,31,53s5-12.74,11.43-12.74S54,46,53.89,53,48.84,65.69,42.45,65.69Zm42.24,0C78.41,65.69,73.31,60,73.31,53s5-12.74,11.43-12.74S96.2,46,96.12,53,91.08,65.69,84.69,65.69Z"/>
  </svg>
)

const TelegramIcon = ({ size = 24, color = "currentColor", ...props }) => (
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width={size} height={size} fill={color} {...props}>
    <path d="M11.944 0A12 12 0 0 0 0 12a12 12 0 0 0 12 12 12 12 0 0 0 12-12A12 12 0 0 0 12 0a12 12 0 0 0-.056 0zm4.962 7.224c.1-.002.321.023.465.14a.506.506 0 0 1 .171.325c.016.093.036.306.02.472-.18 1.898-.962 6.502-1.36 8.627-.168.9-.499 1.201-.82 1.23-.696.065-1.225-.46-1.9-.902-1.056-.693-1.653-1.124-2.678-1.8-1.185-.78-.417-1.21.258-1.91.177-.184 3.247-2.977 3.307-3.23.007-.032.014-.15-.056-.212s-.174-.041-.249-.024c-.106.024-1.793 1.14-5.061 3.345-.48.33-.913.49-1.302.48-.428-.008-1.252-.241-1.865-.44-.752-.245-1.349-.374-1.297-.789.027-.216.325-.437.893-.663 3.498-1.524 5.83-2.529 6.998-3.014 3.332-1.394 4.025-1.636 4.476-1.636z"/>
  </svg>
)

export const Header = memo(function Header(props: any) {
  const {
    search, setSearch, dark, setDark,
    discordOnline, telegramOnline, loading,
    handleSync, handleRefresh, onToggleSidebar,
  } = props
  const { t } = useTranslation()
  const uiState = useContext(MainAppUiStateContext)
  const uiActions = useContext(MainAppUiActionsContext)
  const driveController = useContext(DriveControllerContext)

  const resolvedSearch = search ?? uiState?.search ?? ''
  const resolvedSetSearch = setSearch ?? uiActions?.setSearch ?? (() => {})
  const resolvedDark = dark ?? uiState?.dark ?? false
  const resolvedSetDark = setDark ?? uiActions?.setDark ?? (() => {})
  const resolvedDiscordOnline = discordOnline ?? uiState?.discordOnline ?? false
  const resolvedTelegramOnline = telegramOnline ?? uiState?.telegramOnline ?? false
  const resolvedLoading = loading ?? driveController?.loading ?? false
  const resolvedHandleSync = handleSync ?? driveController?.sync ?? (() => {})
  const resolvedHandleRefresh = handleRefresh ?? driveController?.refresh ?? (() => {})
  const resolvedToggleSidebar = onToggleSidebar ?? uiActions?.toggleSidebar ?? (() => {})

  return (
    <header style={{
      height: 'var(--gd-header-height)', display: 'flex', alignItems: 'center',
      padding: '0 16px', gap: '16px', borderBottom: '1px solid var(--gd-outline-variant)',
      backgroundColor: 'var(--gd-sidebar-bg)', flexShrink: 0, position: 'relative'
    }}>
      <Button variant="ghost" size="icon"
        onClick={resolvedToggleSidebar}
        title={t('header.toggleSidebar')}
        id="header-sidebar-toggle"
      >
        <Menu size={20} />
      </Button>

      <Logo dark={resolvedDark} />

      <div style={{ flex: 1, display: 'flex', justifyContent: 'center' }}>
        <TxtSearch value={resolvedSearch} onChange={resolvedSetSearch} dark={resolvedDark} />
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: '4px', marginLeft: 'auto', flexShrink: 0 }}>
        <div style={{
          display: 'flex', alignItems: 'center', gap: '12px',
          marginRight: '8px'
        }}>
          {/* Discord Status Badge */}
          <div 
            title={resolvedDiscordOnline ? t('header.discordOnline', 'Discord: Online') : t('header.discordOffline', 'Discord: Offline')}
            style={{
              display: 'flex', alignItems: 'center', gap: '8px',
              padding: '4px 12px', borderRadius: '16px',
              backgroundColor: 'rgba(88, 101, 242, 0.1)',
              border: '1px solid rgba(88, 101, 242, 0.15)',
              backdropFilter: 'blur(12px)',
              color: 'var(--gd-on-surface)',
              fontSize: '13px', fontWeight: 500
            }}
          >
            <DiscordIcon size={16} color="#5865F2" />
            <span style={{ position: 'relative', display: 'flex', width: '8px', height: '8px' }}>
              {resolvedDiscordOnline && <span className="animate-ping" style={{ position: 'absolute', width: '100%', height: '100%', borderRadius: '50%', backgroundColor: '#23a559', opacity: 0.7 }} />}
              <span style={{ position: 'relative', width: '100%', height: '100%', borderRadius: '50%', backgroundColor: resolvedDiscordOnline ? '#23a559' : '#80848e' }} />
            </span>
          </div>

          {/* Telegram Status Badge */}
          <div 
            title={resolvedTelegramOnline ? t('header.telegramOnline', 'Telegram: Online') : t('header.telegramOffline', 'Telegram: Offline')}
            style={{
              display: 'flex', alignItems: 'center', gap: '8px',
              padding: '4px 12px', borderRadius: '16px',
              backgroundColor: 'rgba(36, 161, 222, 0.1)',
              border: '1px solid rgba(36, 161, 222, 0.15)',
              backdropFilter: 'blur(12px)',
              color: 'var(--gd-on-surface)',
              fontSize: '13px', fontWeight: 500
            }}
          >
            <TelegramIcon size={16} color="#24A1DE" />
            <span style={{ position: 'relative', display: 'flex', width: '8px', height: '8px' }}>
              {resolvedTelegramOnline && <span className="animate-ping" style={{ position: 'absolute', width: '100%', height: '100%', borderRadius: '50%', backgroundColor: '#23a559', opacity: 0.7 }} />}
              <span style={{ position: 'relative', width: '100%', height: '100%', borderRadius: '50%', backgroundColor: resolvedTelegramOnline ? '#23a559' : '#80848e' }} />
            </span>
          </div>
        </div>

        <BtnSync onClick={resolvedHandleSync} />
        <BtnRefresh onClick={resolvedHandleRefresh} loading={resolvedLoading} />
        <BtnThemeToggle dark={resolvedDark} setDark={resolvedSetDark} />
      </div>
    </header>
  )
})
