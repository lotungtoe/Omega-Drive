import { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { motion } from 'framer-motion'
import { X, RefreshCw } from 'lucide-react'
import {
  getSettings,
  saveSettings,
  applySettings,
  getGPUAdapters,
  getLogStatus,
  createFeatureLogFile,
  openLogsDir,
  triggerBackup,
} from '../services/settingsService'
import { enable, isEnabled, disable } from '@tauri-apps/plugin-autostart'

import { toUserMessage } from '../../../shared/services/errors/toUserMessage'
import { UpdaterSection } from './UpdaterSection'
import { setLanguage, LANG_OPTIONS, i18n } from '../../../lang/index'
import { DropdownSelect } from '../../../shared/components/DropdownSelect'
import { ToggleSwitch } from '../../../shared/components/ToggleSwitch'
import { Button } from '../../../components/ui/be-ui-button'

const SettingsSection = ({ title, description, children }) => (
  <div className="gd-settings-section">
    <div style={{ marginBottom: children ? 16 : 0 }}>
      <h3 className="gd-settings-section-title">{title}</h3>
      {description && <p className="gd-settings-section-desc">{description}</p>}
    </div>
    {children && <div className="gd-settings-section-content">{children}</div>}
  </div>
)

const ToggleRow = ({ label, description, path, getConfigValue, updateConfig }) => {
  const isEnabled = !!getConfigValue(path, false)
  return (
    <div className="gd-settings-row">
      <div style={{ flex: 1 }}>
        <div className="gd-settings-row-label">{label}</div>
        {description && <div className="gd-settings-row-desc">{description}</div>}
      </div>
      <ToggleSwitch
        checked={isEnabled}
        onChange={(v) => updateConfig(path, v)}
        disabled={false}
      />
    </div>
  )
}

const InputRow = ({ label, description, path, type = 'number', step = 1, placeholder = '', getConfigValue, updateConfig }) => (
  <div className="gd-settings-row">
    <div style={{ flex: 1 }}>
      <div className="gd-settings-row-label">{label}</div>
      {description && <div className="gd-settings-row-desc">{description}</div>}
    </div>
    <input
      type={type}
      step={step}
      placeholder={placeholder}
      value={getConfigValue(path, '')}
      onChange={e => {
        const val = e.target.value
        let parsed: string | number = val
        if (type === 'number') {
          parsed = val === '' ? '' : Number.parseFloat(val)
        }
        updateConfig(path, parsed)
      }}
      className="gd-settings-input"
    />
  </div>
)

const SelectRow = ({ label, description, path, options, getConfigValue, updateConfig }) => (
  <div className="gd-settings-row">
    <div style={{ flex: 1 }}>
      <div className="gd-settings-row-label">{label}</div>
      {description && <div className="gd-settings-row-desc">{description}</div>}
    </div>
    <DropdownSelect
      value={getConfigValue(path, options[0]?.value)}
      onChange={(v) => updateConfig(path, v)}
      options={options}
      style={{ width: 220 }}
      disabled={false}
      onDoubleClick={undefined}
    />
  </div>
)

export function SettingsModal({ onClose, toast, dark, toggleDark }) {
  const { t } = useTranslation()
  const [loading, setLoading] = useState(true)
  const [config, setConfig] = useState({})
  const [logStatus, setLogStatus] = useState({})
  const [gpuAdapters, setGPUAdapters] = useState([])
  const [dirty, setDirty] = useState(false)
  const [activeTab, setActiveTab] = useState('appearance')

  const languageOptions = LANG_OPTIONS.map(opt => ({
    ...opt,
    label: t(`settings.languageOption.${opt.value}`, { defaultValue: opt.label }),
  }))

  const tabs = [
    { group: t('settings.groupGeneral', 'CHUNG') },
    { id: 'appearance', label: t('settings.appearance') },
    { id: 'startup', label: t('settings.startup') },
    { group: t('settings.groupConnection', 'CONNECTION') },
    { id: 'server', label: t('settings.server') },
    { id: 'upload', label: t('settings.upload') },
    { id: 'download', label: t('settings.download') },
    { id: 'decode', label: t('settings.decode') },
    { id: 'telegram', label: t('settings.telegram') },
    { group: t('settings.groupAdvanced', 'ADVANCED') },
    { id: 'logging', label: t('settings.logging') },
    { id: 'update', label: 'Update' },
  ]

  useEffect(() => {
    let cancelled = false
    const loadAll = async () => {
      try {
        const [res, autostartEnabled] = await Promise.all([
          getSettings(),
          isEnabled().catch((e) => {
            console.warn('autostart check failed', e)
            return false
          }),
        ])
        if (!cancelled) {
          const nextConfig = (res as any).config || {}
          if (!nextConfig.ui) nextConfig.ui = {}
          if (!nextConfig.ui.language && i18n.language) {
            nextConfig.ui.language = i18n.language
          }
          if (!nextConfig.startup) nextConfig.startup = {}
          if (nextConfig.startup.persistent_video_bridge === undefined) {
            nextConfig.startup.persistent_video_bridge = true
          }
          nextConfig.startup.launch_on_boot = autostartEnabled
          setConfig(nextConfig)
        }
      } catch (err) {
        if (!cancelled) {
          const msg = toUserMessage(err)
          console.error('settings.load_failed', err)
          toast.show(msg.message || t('settings.loadFailed'), 'error')
        }
      } finally {
        if (!cancelled) setLoading(false)
      }
    }

    const loadLogStatus = async () => {
      try {
        const status = await getLogStatus()
        if (!cancelled) setLogStatus(status || {})
      } catch (err) {
        if (!cancelled) console.warn('settings.log_status_failed', err)
      }
    }

    loadAll()
    loadLogStatus()

    return () => { cancelled = true }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [onClose])

  // Load GPU adapters when decode tab becomes active
  useEffect(() => {
    if (activeTab !== 'decode') return
    let cancelled = false
    getGPUAdapters().then(list => {
      if (!cancelled) setGPUAdapters(list || [])
    }).catch(() => {})
    return () => { cancelled = true }
  }, [activeTab])

  const refreshLogStatus = async () => {
    try {
      const status = await getLogStatus()
      setLogStatus(status || {})
    } catch (err) {
      console.warn('settings.log_status_failed', err)
    }
  }

  const handleCreateLogFile = async (feature) => {
    const featureLabel = t(`settings.logFeature.${feature}`, { defaultValue: feature })
    try {
      await createFeatureLogFile(feature)
      await refreshLogStatus()
      toast.show(t('settings.logCreateSuccess', { feature: featureLabel }), 'success')
    } catch (err) {
      const msg = toUserMessage(err)
      console.error('settings.log_create_failed', err)
      toast.show(msg.message || t('settings.logCreateFailed'), 'error')
    }
  }

  const handleOpenLogsDir = async () => {
    try {
      await openLogsDir()
    } catch (err) {
      const msg = toUserMessage(err)
      console.error('settings.open_logs_failed', err)
      toast.show(msg.message || t('settings.openLogsFailed'), 'error')
    }
  }

  const [backingUp, setBackingUp] = useState(false)

  const handleBackupNow = async () => {
    setBackingUp(true)
    try {
      await triggerBackup()
      toast.show(t('settings.backupNowSuccess', 'Backup completed!'), 'success')
    } catch (err) {
      const msg = toUserMessage(err)
      console.error('settings.backup_failed', err)
      toast.show(msg.message || t('settings.backupNowFailed', 'Backup failed'), 'error')
    } finally {
      setBackingUp(false)
    }
  }

  const updateConfig = (path, value) => {
    if (path === 'ui.language') {
      setLanguage(value)
    }
    if (path === 'ui.dark_mode' && typeof toggleDark === 'function') {
      if (value !== dark) {
        toggleDark()
      }
    }
    if (path === 'startup.launch_on_boot') {
      const toggleAutostart = async () => {
        try {
          if (value) await enable()
          else await disable()
        } catch (e) {
          console.error('Failed to toggle autostart', e)
        }
      }
      toggleAutostart()
    }
    const keys = path.split('.')
    const next = { ...config }
    let cur = next
    for (let i = 0; i < keys.length - 1; i++) {
      if (!cur[keys[i]]) cur[keys[i]] = {}
      cur[keys[i]] = { ...cur[keys[i]] }
      cur = cur[keys[i]]
    }
    cur[keys[keys.length - 1]] = value

    setConfig(next)
    setDirty(true)
  }

  const getConfigValue = (path, defaultValue = '') => {
    return path.split('.').reduce((obj, key) => obj?.[key], config) ?? defaultValue
  }

  const handleSave = async () => {
    try {
      await saveSettings(config)
      setDirty(false)
      toast.show(t('settings.saveSuccess', 'Saved'), 'success')
    } catch (err) {
      const msg = toUserMessage(err)
      console.error('settings.save_failed', err)
      toast.show(msg.message || t('settings.saveFailed'), 'error')
    }
  }

  const handleApply = async () => {
    try {
      await applySettings(config)
      setDirty(false)
      toast.show(t('settings.applySuccess', 'Applied'), 'success')
    } catch (err) {
      const msg = toUserMessage(err)
      console.error('settings.apply_failed', err)
      toast.show(msg.message || t('settings.applyFailed'), 'error')
    }
  }

  const handleClose = async () => {
    if (dirty) {
      const confirmed = confirm(t('settings.unsavedConfirm', 'You have unsaved changes. Close?'))
      if (!confirmed) return
    }
    onClose()
  }

  const commonProps = { getConfigValue, updateConfig }
  const logFeatures = [
    { key: 'upload', label: t('settings.logFeature.upload') },
    { key: 'download', label: t('settings.logFeature.download') },
    { key: 'player', label: t('settings.logFeature.player') },
    { key: 'drive', label: t('settings.logFeature.drive') },
    { key: 'settings', label: t('settings.logFeature.settings') },
    { key: 'diagnostics', label: t('settings.logFeature.diagnostics') },
  ]

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.95 }}
      animate={{ opacity: 1, scale: 1 }}
      exit={{ opacity: 0, scale: 0.95 }}
      transition={{ duration: 0.15 }}
      className="gd-settings-fullscreen"
      tabIndex={0}
      onKeyDown={(e) => { if (e.key === 'Escape') handleClose() }}
    >
      <div className="gd-settings-sidebar-wrapper">
        <div className="gd-settings-sidebar">
          {tabs.map((tab) => {
            if (tab.group) {
              return <div key={`grp-${tab.group}`} className="gd-settings-group-title">{tab.group}</div>
            }
            return (
              <Button
                key={tab.id}
                variant="ghost"
                size="md"
                onClick={() => setActiveTab(tab.id)}
                className={`gd-settings-tab ${activeTab === tab.id ? 'active' : ''}`}
                style={{ fontWeight: activeTab === tab.id ? 700 : 500 }}
              >
                {tab.label}
              </Button>
            )
          })}
        </div>
      </div>

      <div className="gd-settings-content-wrapper">
        <div className="gd-settings-content">
          {loading ? (
            <div style={{ display: 'flex', justifyContent: 'center', alignItems: 'center', height: '100%', minHeight: 300 }}>
              <RefreshCw className="animate-spin" size={28} style={{ color: 'var(--gd-blue)' }} />
            </div>
          ) : (
            <>
              {activeTab === 'appearance' && (
                <SettingsSection title={t('settings.appearance')} description={t('settings.appearanceDesc')}>
                  <ToggleRow
                    label={t('settings.darkMode')}
                    description={t('settings.darkModeDesc')}
                    path="ui.dark_mode"
                    {...commonProps}
                  />
                  <SelectRow
                    label={t('settings.language')}
                    description={t('settings.languageDesc')}
                    path="ui.language"
                    options={languageOptions}
                    {...commonProps}
                  />
                </SettingsSection>
              )}

              {activeTab === 'startup' && (
                <SettingsSection title={t('settings.startup')} description={t('settings.startupDesc')}>
                  <ToggleRow
                    label={t('settings.launchOnBoot')}
                    description={t('settings.launchOnBootDesc')}
                    path="startup.launch_on_boot"
                    {...commonProps}
                  />
                  <ToggleRow
                    label={t('settings.autoSyncOnStartup')}
                    description={t('settings.autoSyncOnStartupDesc')}
                    path="startup.auto_sync"
                    {...commonProps}
                  />
                  <ToggleRow
                    label={t('settings.persistentVideoBridge')}
                    description={t('settings.persistentVideoBridgeDesc')}
                    path="startup.persistent_video_bridge"
                    {...commonProps}
                  />
                </SettingsSection>
              )}

              {activeTab === 'server' && (
                <SettingsSection title={t('settings.server')} description={t('settings.serverDesc')}>
                  <InputRow label={t('settings.host')} description={t('settings.hostDesc')} path="server.host" type="text" placeholder="127.0.0.1" {...commonProps} />
                  <InputRow label={t('settings.port')} description={t('settings.portDesc')} path="server.port" placeholder="8080" {...commonProps} />
                  <SelectRow
                    label={t('settings.logLevel')}
                    description={t('settings.logLevelDesc')}
                    path="server.log_level"
                    options={[
                      { value: 'error', label: t('settings.logLevelOption.error') },
                      { value: 'warn', label: t('settings.logLevelOption.warn') },
                      { value: 'info', label: t('settings.logLevelOption.info') },
                      { value: 'debug', label: t('settings.logLevelOption.debug') },
                      { value: 'trace', label: t('settings.logLevelOption.trace') },
                    ]}
                    {...commonProps}
                  />
                  <InputRow label={t('settings.autoSync')} description={t('settings.autoSyncDesc')} path="server.auto_sync_interval_s" {...commonProps} />
                  <div className="gd-settings-divider" style={{ margin: '16px 0', borderTop: '1px solid var(--gd-outline)', opacity: 0.5 }} />
                  <h4 style={{ fontSize: 13, fontWeight: 600, color: 'var(--gd-on-surface)', marginBottom: 12, opacity: 0.8 }}>BACKUP</h4>
                  <ToggleRow label={t('settings.backupEnabled')} description={t('settings.backupEnabledDesc')} path="backup.enabled" {...commonProps} />
                  <InputRow label={t('settings.backupInterval')} description={t('settings.backupIntervalDesc')} path="backup.snapshot_interval_days" {...commonProps} />
                  <div className="gd-settings-row" style={{ borderBottom: 'none' }}>
                    <Button
                      variant="primary"
                      size="md"
                      onClick={handleBackupNow}
                      disabled={backingUp}
                      style={{ width: '100%' }}
                    >
                      {backingUp ? t('settings.backingUp', 'Äang sao lÆ°u...') : t('settings.backupNow', 'Sao lÆ°u ngay')}
                    </Button>
                  </div>
                </SettingsSection>
              )}

              {activeTab === 'upload' && (
                <SettingsSection title={t('settings.upload')} description={t('settings.uploadDesc')}>
                  <SelectRow
                    label={t('settings.uploadMode')}
                    description={t('settings.uploadModeDesc')}
                    path="upload.upload_mode"
                    options={[
                      { value: 'safe', label: t('settings.uploadModeSafe') },
                      { value: 'speed', label: t('settings.uploadModeSpeed') },
                    ]}
                    {...commonProps}
                  />
                  <InputRow label={t('settings.chunkSize')} description={t('settings.chunkSizeDesc')} path="upload.general.chunk_mb" {...commonProps} />
                  <InputRow label={t('settings.parallelChunks')} description={t('settings.parallelChunksDesc')} path="upload.general.parallel_sends" {...commonProps} />
                  <InputRow label={t('settings.zipLevel')} description={t('settings.zipLevelDesc')} path="upload.general.zip_level" {...commonProps} />
                  <InputRow label={t('settings.safeRatio')} description={t('settings.safeRatioDesc')} path="upload.general.safe_ratio" step={0.01} {...commonProps} />
                  
                  <div className="gd-settings-divider" style={{ margin: '16px 0', borderTop: '1px solid var(--gd-outline)', opacity: 0.5 }} />
                  
                  <h4 style={{ fontSize: 13, fontWeight: 600, color: 'var(--gd-on-surface)', marginBottom: 12, opacity: 0.8 }}>DISCORD SETTINGS</h4>
                  <InputRow label={t('settings.discordChunkSize', 'Discord Chunk Size (MB)')} description={t('settings.discordChunkSizeDesc', 'Optimal chunk size for Discord.')} path="providers.discord.transfer.chunk_mb" {...commonProps} />
                  <InputRow label={t('settings.discordBatchSize', 'Discord Batch Size')} description={t('settings.discordBatchSizeDesc', 'Number of chunks to batch together.')} path="providers.discord.transfer.batch_size" {...commonProps} />
                  <InputRow label={t('settings.discordParallel')} description={t('settings.discordParallelDesc')} path="providers.discord.transfer.parallel_sends" {...commonProps} />
                  <InputRow label={t('settings.retryCount')} description={t('settings.retryCountDesc')} path="providers.discord.retry.send_retries" {...commonProps} />
                  <InputRow label={t('settings.retryDelay')} description={t('settings.retryDelayDesc')} path="providers.discord.retry.retry_base_delay_s" {...commonProps} />

                  <div className="gd-settings-divider" style={{ margin: '16px 0', borderTop: '1px solid var(--gd-outline)', opacity: 0.5 }} />

                  <h4 style={{ fontSize: 13, fontWeight: 600, color: 'var(--gd-on-surface)', marginBottom: 12, opacity: 0.8 }}>TELEGRAM SETTINGS</h4>
                  <InputRow label={t('settings.telegramChunkSize', 'Telegram Chunk Size (MB)')} description={t('settings.telegramChunkSizeDesc', 'Optimal chunk size for Telegram.')} path="providers.telegram.transfer.chunk_mb" {...commonProps} />
                  <InputRow label={t('settings.telegramParallel')} description={t('settings.telegramParallelDesc')} path="providers.telegram.transfer.parallel_sends" {...commonProps} />
                </SettingsSection>
              )}

              {activeTab === 'download' && (
                <SettingsSection title={t('settings.download')} description={t('settings.downloadDesc')}>
                  <InputRow label={t('settings.httpTimeout')} description={t('settings.httpTimeoutDesc')} path="download.http_timeout_s" {...commonProps} />
                  <InputRow label={t('settings.downloadRetryCount')} description={t('settings.downloadRetryCountDesc')} path="download.retry_count" {...commonProps} />
                  <InputRow label={t('settings.partDelay')} description={t('settings.partDelayDesc')} path="download.part_delay_ms" {...commonProps} />
                  <InputRow label={t('settings.streamBuffer')} description={t('settings.streamBufferDesc')} path="download.stream_buffer_kb" {...commonProps} />
                  <ToggleRow
                    label={t('settings.preventSleep')}
                    description={t('settings.preventSleepDesc')}
                    path="download.prevent_sleep_enabled"
                    {...commonProps}
                  />
                  <ToggleRow
                    label={t('settings.adaptiveSoftLimit')}
                    description={t('settings.adaptiveSoftLimitDesc')}
                    path="download.adaptive_soft_limit"
                    {...commonProps}
                  />
                  <InputRow
                    label={t('settings.bandwidthLimit')}
                    description={t('settings.bandwidthLimitDesc')}
                    path="download.bandwidth_limit_kbps"
                    {...commonProps}
                  />
                  <InputRow
                    label={t('settings.softLimitRatio')}
                    description={t('settings.softLimitRatioDesc')}
                    path="download.soft_limit_ratio"
                    step={0.05}
                    {...commonProps}
                  />
                  <ToggleRow
                    label={t('settings.softLimitWhenPlayerActive')}
                    description={t('settings.softLimitWhenPlayerActiveDesc')}
                    path="download.soft_limit_when_player_active"
                    {...commonProps}
                  />
                  <ToggleRow
                    label={t('settings.softLimitWhenMinimized')}
                    description={t('settings.softLimitWhenMinimizedDesc')}
                    path="download.soft_limit_when_minimized"
                    {...commonProps}
                  />
                  <InputRow
                    label={t('settings.diskCheckInterval')}
                    description={t('settings.diskCheckIntervalDesc')}
                    path="download.disk_check_interval_parts"
                    {...commonProps}
                  />
                  <ToggleRow
                    label={t('settings.autoResumeOnStartup')}
                    description={t('settings.autoResumeOnStartupDesc')}
                    path="download.auto_resume_on_startup"
                    {...commonProps}
                  />
                </SettingsSection>
              )}

              {activeTab === 'decode' && (
                <SettingsSection title={t('settings.decode')} description={t('settings.decodeDesc')}>
                  <SelectRow
                    label={t('settings.d3d11Adapter')}
                    description={t('settings.d3d11AdapterDesc')}
                    path="download.d3d11_adapter"
                    options={[
                      { value: 'Auto', label: t('common.auto') },
                      ...gpuAdapters.filter(a => a !== 'Auto').map(name => ({ value: name, label: name })),
                    ]}
                    {...commonProps}
                  />
                </SettingsSection>
              )}

              {activeTab === 'telegram' && (
                <SettingsSection title={t('settings.telegram')} description={t('settings.telegramDesc')}>
                  <InputRow label={t('settings.telegramFileLimit')} description={t('settings.telegramFileLimitDesc')} path="providers.telegram.limits.file_limit_mb" {...commonProps} />
                </SettingsSection>
              )}

              {activeTab === 'logging' && (
                <SettingsSection title={t('settings.logging')} description={t('settings.loggingDesc')}>
                  <ToggleRow
                    label={t('settings.frontendLogging')}
                    description={t('settings.frontendLoggingDesc')}
                    path="logging.frontend_enabled"
                    {...commonProps}
                  />
                  <div className="gd-settings-row">
                    <div style={{ flex: 1 }}>
                      <div className="gd-settings-row-label">{t('settings.logFolder')}</div>
                      <div className="gd-settings-row-desc">{t('settings.logFolderDesc')}</div>
                    </div>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={handleOpenLogsDir}
                    >
                      {t('settings.openLogFolder')}
                    </Button>
                  </div>
                  {logFeatures.map((feature) => {
                    const enabled = getConfigValue(`logging.feature_enabled.${feature.key}`, true as any)
                    const exists = !!logStatus?.[feature.key]
                    return (
                      <div key={feature.key}>
                        <ToggleRow
                          label={t('settings.logFeatureLabel', { feature: feature.label })}
                          description={t('settings.logFeatureDesc', { feature: feature.label })}
                          path={`logging.feature_enabled.${feature.key}`}
                          {...commonProps}
                        />
                        <div className="gd-settings-row" style={{ paddingTop: 0, borderBottom: 'none' }}>
                          <div style={{ flex: 1 }}>
                            <div className="gd-settings-row-desc" style={{ marginTop: 0 }}>
                              {exists ? 'âœ“ ' + t('settings.logFileExists') : t('settings.logFileMissing')}
                            </div>
                          </div>
                          {!exists && enabled && (
                            <Button
                              variant="outline"
                              size="sm"
                              onClick={() => handleCreateLogFile(feature.key)}
                            >
                              {t('settings.logFileCreate')}
                            </Button>
                          )}
                        </div>
                      </div>
                    )
                  })}
                </SettingsSection>
              )}

              {activeTab === 'update' && (
                <UpdaterSection />
              )}
            </>
          )}
        </div>

        {/* Floating save/apply buttons */}
        <div style={{
          position: 'absolute',
          bottom: 32,
          right: 80,
          display: 'flex',
          gap: 12,
        }}>
          <button
            type="button"
            onClick={handleSave}
            disabled={!dirty}
            className="rounded-2xl border border-[var(--gd-outline)] px-8 py-3 text-[11px] font-bold uppercase text-[var(--gd-on-surface-variant)] transition-all disabled:opacity-50"
          >
            {t('common.save', 'LÆ°u')}
          </button>
          <button
            type="button"
            onClick={handleApply}
            disabled={!dirty}
            className="rounded-2xl bg-blue-500 px-8 py-3 text-[11px] font-bold uppercase text-white shadow-lg shadow-blue-500/20 transition-all hover:bg-blue-600 disabled:opacity-50 focus:ring-2 focus:ring-blue-500/50"
          >
            {t('common.apply', 'Apply')}
          </button>
        </div>

        {/* Discord-style ESC button area */}
        <div className="gd-settings-escape">
          <Button variant="ghost" size="icon" onClick={handleClose} aria-label="Close">
            <X size={20} />
          </Button>
          <span className="gd-settings-escape-text">ESC</span>
        </div>

      </div>
    </motion.div>
  )
}

