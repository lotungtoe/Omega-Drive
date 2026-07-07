import { useCallback, useEffect, useMemo, useState } from 'react'
import { motion } from 'framer-motion'
import { useTranslation } from 'react-i18next'
import { DriveApi } from '../../../api/index'
import { toUserMessage } from '../../../shared/services/errors/toUserMessage'
import { cn } from '../../../shared/utils/index'
import { DropdownSelect } from '../../../shared/components/DropdownSelect'
import { Button } from '../../../components/ui/be-ui-button'

function DiscordSection({
  discordToken,
  setDiscordToken,
  loadingKey,
  state,
  destinationsLoaded,
  discordSelection,
  setDiscordSelection,
  preferredScope,
  handleAction,
  t,
}) {
  const guilds = state?.discordGuilds || []
  return (
    <section className="rounded-2xl border p-4 border-[var(--gd-modal-border)] bg-[var(--gd-modal-close-bg)]">
      <div className="mb-3">
        <h3 className="text-sm font-semibold">{t('onboarding.discordSection', 'Discord')}</h3>
        <p className="mt-1 text-xs text-[var(--gd-modal-text-secondary)]">
          {t(
            'onboarding.discordSectionDesc',
            'Nhap token bot Discord de app doc danh sach server ma bot dang tham gia.'
          )}
        </p>
      </div>
      <div className="flex flex-col gap-3">
        <input
          value={discordToken}
          onChange={(event) => setDiscordToken(event.target.value)}
          placeholder={t('onboarding.discordTokenPlaceholder', 'Discord bot token')}
          className="rounded-xl border px-4 py-2.5 text-sm border-[var(--gd-input-border)] bg-[var(--gd-input-bg)] text-[var(--gd-modal-text)] placeholder:text-[var(--gd-modal-text-secondary)]"
        />
        <div className="flex flex-wrap items-center justify-between gap-3">
          <label className="flex-1 min-w-0">
            <DropdownSelect 
                value={discordSelection}
                disabled={!destinationsLoaded}
                onChange={setDiscordSelection}
                style={undefined}
                onDoubleClick={undefined}
                options={
                  destinationsLoaded
                    ? [
                        { value: '', label: t('onboarding.noneDiscord', 'Khong dung Discord') },
                        ...guilds.map((g) => ({ value: g.id, label: `${g.name} (${g.id})` })),
                      ]
                    : [{ value: '', label: t('onboarding.loadingDestinations', 'Dang tai danh sach...') }]
                }
              />
          </label>
          <Button
            variant="primary"
            size="sm"
            disabled={loadingKey === 'discord'}
            onClick={() =>
              handleAction('discord', () => DriveApi.saveDiscordToken(discordToken), preferredScope, false)
            }
          >
            {loadingKey === 'discord'
              ? t('onboarding.saving', 'Dang luu...')
              : t('onboarding.saveDiscordToken', 'Luu token Discord')}
          </Button>
        </div>
      </div>
    </section>
  )
}

function TelegramSection({
  telegramPhone,
  setTelegramPhone,
  telegramApiId,
  setTelegramApiId,
  telegramApiHash,
  setTelegramApiHash,
  telegramCode,
  setTelegramCode,
  telegramPassword,
  setTelegramPassword,
  loadingKey,
  state,
  destinationsLoaded,
  telegramSelection,
  setTelegramSelection,
  preferredScope,
  handleAction,
  t,
}) {
  const groups = state?.telegramGroups || []
  const isLoggedIn = state?.telegramAuthorized
  return (
    <section className="rounded-2xl border p-4 border-[var(--gd-modal-border)] bg-[var(--gd-modal-close-bg)]">
      <div className="mb-3">
        <h3 className="text-sm font-semibold">{t('onboarding.telegramSection', 'Telegram')}</h3>
        <p className="mt-1 text-xs text-[var(--gd-modal-text-secondary)]">
          {t(
            'onboarding.telegramSectionDesc',
            'Luu so dien thoai va API, sau do gui ma dang nhap MTProto. Neu bat 2FA, app se hoi them mat khau.'
          )}
        </p>
      </div>
      <div className="grid gap-3 md:grid-cols-2">
        <input
          value={telegramPhone}
          onChange={(event) => setTelegramPhone(event.target.value)}
          placeholder={t('onboarding.telegramPhonePlaceholder', 'So dien thoai Telegram')}
          className="rounded-xl border px-4 py-2.5 text-sm border-[var(--gd-input-border)] bg-[var(--gd-input-bg)] text-[var(--gd-modal-text)] placeholder:text-[var(--gd-modal-text-secondary)]"
        />
        <input
          value={telegramApiId}
          onChange={(event) => setTelegramApiId(event.target.value.replace(/[^\d]/g, ''))}
          placeholder={t('onboarding.telegramApiIdPlaceholder', 'Telegram API ID')}
          className="rounded-xl border px-4 py-2.5 text-sm border-[var(--gd-input-border)] bg-[var(--gd-input-bg)] text-[var(--gd-modal-text)] placeholder:text-[var(--gd-modal-text-secondary)]"
        />
        <input
          value={telegramApiHash}
          onChange={(event) => setTelegramApiHash(event.target.value)}
          placeholder={t('onboarding.telegramApiHashPlaceholder', 'Telegram API hash')}
          className="rounded-xl border px-4 py-2.5 text-sm md:col-span-2 border-[var(--gd-input-border)] bg-[var(--gd-input-bg)] text-[var(--gd-modal-text)] placeholder:text-[var(--gd-modal-text-secondary)]"
        />
      </div>
      <div className="mt-3 flex flex-wrap items-center justify-between gap-3">
        <label className="flex-1 min-w-0">
          <DropdownSelect
            value={telegramSelection}
            disabled={!destinationsLoaded || !isLoggedIn}
            onChange={setTelegramSelection}
            style={undefined}
            onDoubleClick={undefined}
            options={
              destinationsLoaded && isLoggedIn
                ? [
                    { value: '', label: t('onboarding.noneTelegram', 'Khong dung Telegram') },
                    ...groups.map((g) => ({ value: g.id, label: `${g.name} (${g.id})` })),
                  ]
                : [{ value: '', label: t('onboarding.loadingDestinations', 'Dang tai danh sach...') }]
            }
          />
        </label>
        <Button
          variant="primary"
          size="sm"
          disabled={loadingKey === 'telegram-login'}
          onClick={() =>
            handleAction(
              'telegram-login',
              async () => {
                let formattedPhone = telegramPhone.trim();
                if (formattedPhone && !formattedPhone.startsWith('+')) {
                  formattedPhone = '+' + formattedPhone;
                }
                await DriveApi.saveTelegramCredentials(
                  formattedPhone,
                  String(Number.parseInt(telegramApiId, 10) || 0),
                  telegramApiHash
                );
                return await DriveApi.sendTelegramLoginCode();
              },
              preferredScope,
              false
            )
          }
        >
          {loadingKey === 'telegram-login'
            ? t('onboarding.loggingIn', 'Dang dang nhap...')
            : t('onboarding.telegramLogin', 'Dang nhap Telegram')}
        </Button>
      </div>

      {state?.telegramLoginStep === 'code' && (
        <div className="mt-4 rounded-2xl border border-dashed border-blue-400/40 p-4">
          <label className="mb-2 block text-xs font-semibold uppercase tracking-wide text-blue-400">
            {t('onboarding.telegramCode', 'Ma dang nhap Telegram')}
          </label>
          <div className="flex flex-col gap-3 md:flex-row">
            <input
              value={telegramCode}
              onChange={(event) => setTelegramCode(event.target.value)}
              placeholder={t('onboarding.telegramCodePlaceholder', 'Nhap ma vua nhan')}
              className="flex-1 rounded-xl border px-4 py-2.5 text-sm border-[var(--gd-input-border)] bg-[var(--gd-input-bg)] text-[var(--gd-modal-text)] placeholder:text-[var(--gd-modal-text-secondary)]"
            />
            <Button
              variant="primary"
              size="sm"
              disabled={loadingKey === 'telegram-submit-code'}
              onClick={() =>
                handleAction(
                  'telegram-submit-code',
                  () => DriveApi.submitTelegramLoginCode(telegramCode),
                  preferredScope,
                  false
                )
              }
            >
              {loadingKey === 'telegram-submit-code'
                ? t('onboarding.confirming', 'Dang xac nhan...')
                : t('onboarding.confirmTelegramCode', 'Xac nhan ma')}
            </Button>
          </div>
        </div>
      )}

      {state?.telegramLoginStep === 'password' && (
        <div className="mt-4 rounded-2xl border border-dashed border-amber-400/40 p-4">
          <label className="mb-2 block text-xs font-semibold uppercase tracking-wide text-amber-400">
            {t('onboarding.telegramPassword', 'Mat khau 2FA Telegram')}
          </label>
          {state?.telegramPasswordHint ? (
            <p className="mb-2 text-xs text-[var(--gd-modal-text-secondary)]">
              {t('onboarding.telegramPasswordHint', {
                defaultValue: 'Goi y: {{hint}}',
                hint: state.telegramPasswordHint,
              })}
            </p>
          ) : null}
          <div className="flex flex-col gap-3 md:flex-row">
            <input
              type="password"
              value={telegramPassword}
              onChange={(event) => setTelegramPassword(event.target.value)}
              placeholder={t('onboarding.telegramPasswordPlaceholder', 'Nhap mat khau 2FA')}
              className="flex-1 rounded-xl border px-4 py-2.5 text-sm border-[var(--gd-input-border)] bg-[var(--gd-input-bg)] text-[var(--gd-modal-text)] placeholder:text-[var(--gd-modal-text-secondary)]"
            />
            <Button
              variant="primary"
              size="sm"
              disabled={loadingKey === 'telegram-password'}
              onClick={() =>
                handleAction(
                  'telegram-password',
                  () => DriveApi.submitTelegramPassword(telegramPassword),
                  preferredScope,
                  false
                )
              }
            >
              {loadingKey === 'telegram-password'
                ? t('onboarding.confirming', 'Dang xac nhan...')
                : t('onboarding.confirmTelegramPassword', 'Xac nhan 2FA')}
            </Button>
          </div>
        </div>
      )}
    </section>
  )
}

export function ProviderOnboardingModal({
  state,
  preferredScope,
  onStateChange,
  onSkip,
  toast,
}) {
  const { t } = useTranslation()
  const [discordToken, setDiscordToken] = useState('')
  const [telegramPhone, setTelegramPhone] = useState('')
  const [telegramApiId, setTelegramApiId] = useState('')
  const [telegramApiHash, setTelegramApiHash] = useState('')
  const [telegramCode, setTelegramCode] = useState('')
  const [telegramPassword, setTelegramPassword] = useState('')
  const [loadingKey, setLoadingKey] = useState('')
  const [discordSelection, setDiscordSelection] = useState('')
  const [telegramSelection, setTelegramSelection] = useState('')

  useEffect(() => {
    if (!state) return
    setDiscordToken(prev => prev || state.discordToken || '')
    setTelegramPhone(prev => prev || state.telegramPhone || '')
    setTelegramApiId(prev => prev || String(state.telegramApiId ?? ''))
    setTelegramApiHash(prev => prev || state.telegramApiHash || '')
  }, [state?.discordToken, state?.telegramPhone, state?.telegramApiId, state?.telegramApiHash])

  const applyState = useCallback(
    (nextState, nextScope = preferredScope, forceVisible = true) => {
      onStateChange(nextState, nextScope, forceVisible)
    },
    [onStateChange, preferredScope]
  )

  const refreshState = useCallback(
    async (nextScope = preferredScope, forceVisible = true) => {
      const nextState = await DriveApi.getOnboardingState()
      applyState(nextState, nextScope, forceVisible)
      return nextState
    },
    [applyState, preferredScope]
  )

  useEffect(() => {
    if (state) return
    void refreshState(preferredScope, true).catch((error) => {
      console.error('[Onboarding] Failed to refresh state:', error)
    })
  }, [preferredScope, refreshState, state])

  useEffect(() => {
    if (!state) return
    setTelegramCode((current) => (state.telegramLoginStep === 'code' ? current : ''))
    setTelegramPassword((current) => (state.telegramLoginStep === 'password' ? current : ''))
  }, [state])

  const needsTenantSelection = Boolean(state?.tenants?.[preferredScope]?.needsSelection || state?.tenants?.my?.needsSelection || state?.tenants?.shared?.needsSelection)

  // Poll every 2s while modal needs tenant selection and has credentials
  useEffect(() => {
    if (!needsTenantSelection) return
    if (!state?.discordTokenPresent && !state?.telegramAuthorized && !state?.telegramCredentialsPresent) return

    let active = true
    const poll = () => {
      void DriveApi.loadOnboardingDestinations()
        .then((nextState) => {
          if (!active) return
          applyState(nextState, preferredScope, true)
        })
        .catch((error) => {
          if (!active) return
          console.error('[Onboarding] Polling destinations failed:', error)
        })
    }

    poll() // immediate first call
    const id = setInterval(poll, 10000)
    return () => {
      active = false
      clearInterval(id)
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [needsTenantSelection, state?.discordTokenPresent, state?.telegramAuthorized, state?.telegramCredentialsPresent, preferredScope])

  const handleAction = async (key, action, nextScope = preferredScope, forceVisible = false) => {
    setLoadingKey(key)
    try {
      const nextState = await action()
      applyState(nextState, nextScope, forceVisible)
      return nextState
    } catch (error) {
      const message = toUserMessage(error)
      console.error(`[Onboarding] ${key} failed:`, error)
      toast.show(message.message || t('onboarding.actionFailed', 'Thao tac that bai.'), 'error')
      return null
    } finally {
      setLoadingKey('')
    }
  }

  const handleCreateTenant = async () => {
    const scope = preferredScope || 'my'
    const nextState = await handleAction(
      `create-${scope}`,
      () => DriveApi.createOnboardingTenant(scope, discordSelection || null, telegramSelection || null),
      scope,
      false
    )
    if (nextState && !nextState?.requiresOnboarding) {
      toast.show(t('onboarding.createSuccess', 'Tao co so du lieu thanh cong!'), 'success')
    }
  }

  const renderStatusPill = (label, ok) => {
    return (
      <span
        className={cn(
          'rounded-full px-2.5 py-1 text-[11px] font-semibold',
          ok ? 'bg-emerald-500/15 text-emerald-500' : 'bg-amber-500/15 text-amber-500'
        )}
      >
        {label}
      </span>
    )
  }

  const destinationsLoaded = Boolean(state?.destinationsLoaded)
  const canCreateTenant = destinationsLoaded && (Boolean(discordSelection) || Boolean(telegramSelection))
  const isCreating = loadingKey.startsWith('create-')

  const needsScope = useMemo(() => {
    if (!state?.tenants) return null
    return [preferredScope, 'my', 'shared'].find((s) => state?.tenants?.[s]?.needsSelection) || null
  }, [state, preferredScope])

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="fixed inset-0 z-[180] flex items-center justify-center bg-black/60 px-4 py-6 backdrop-blur-sm"
    >
      <motion.div
        initial={{ scale: 0.96, y: 10 }}
        animate={{ scale: 1, y: 0 }}
        exit={{ scale: 0.96, y: 10 }}
        className="relative flex max-h-[90vh] w-full max-w-3xl flex-col overflow-hidden rounded-[28px] border shadow-2xl border-[var(--gd-modal-border)] bg-[var(--gd-modal-surface)] text-[var(--gd-modal-text)]"
      >
        <div className="border-b px-6 py-5 border-[var(--gd-modal-border)]">
          <div className="flex flex-wrap items-center gap-2">
            <h2 className="text-xl font-semibold">
              {t('onboarding.title', 'Thiet lap ket noi va tenant')}
            </h2>
            {renderStatusPill(
              state?.discordTokenPresent
                ? t('onboarding.discordReady', 'Discord token: OK')
                : t('onboarding.discordMissing', 'Thieu Discord token'),
              Boolean(state?.discordTokenPresent)
            )}
            {renderStatusPill(
              state?.telegramAuthorized
                ? t('onboarding.telegramReady', 'Telegram auth: OK')
                : t('onboarding.telegramMissing', 'Thieu Telegram auth'),
              Boolean(state?.telegramAuthorized)
            )}
          </div>
          <p className="mt-2 text-sm text-[var(--gd-modal-text-secondary)]">
            {t(
              'onboarding.subtitle',
              'App dung credential hien tai de tao tenant khi can. Khong co DB hop le thi moi can chon server hoac group.'
            )}
          </p>
        </div>

        <div className="flex flex-1 flex-col gap-6 overflow-y-auto px-6 py-6">
          <DiscordSection
            discordToken={discordToken}
            setDiscordToken={setDiscordToken}
            loadingKey={loadingKey}
            state={state}
            destinationsLoaded={destinationsLoaded}
            discordSelection={discordSelection}
            setDiscordSelection={setDiscordSelection}
            preferredScope={preferredScope}
            handleAction={handleAction}
            t={t}
          />

          <TelegramSection
            telegramPhone={telegramPhone}
            setTelegramPhone={setTelegramPhone}
            telegramApiId={telegramApiId}
            setTelegramApiId={setTelegramApiId}
            telegramApiHash={telegramApiHash}
            setTelegramApiHash={setTelegramApiHash}
            telegramCode={telegramCode}
            setTelegramCode={setTelegramCode}
            telegramPassword={telegramPassword}
            setTelegramPassword={setTelegramPassword}
            loadingKey={loadingKey}
            state={state}
            destinationsLoaded={destinationsLoaded}
            telegramSelection={telegramSelection}
            setTelegramSelection={setTelegramSelection}
            preferredScope={preferredScope}
            handleAction={handleAction}
            t={t}
          />
        </div>

        <div className="flex items-center justify-end gap-2 border-t px-6 py-4 border-[var(--gd-modal-border)]">
          {needsScope ? (
            <Button
              variant="primary"
              size="sm"
              disabled={!canCreateTenant || isCreating}
              onClick={() => void handleCreateTenant()}
            >
              {isCreating
                ? t('onboarding.creating', 'Dang tao...')
                : t('onboarding.createTenant', 'Tao co so du lieu')}
            </Button>
          ) : null}
          <Button
            variant="ghost"
            size="sm"
            onClick={onSkip}
          >
            {t('onboarding.skip', 'Bo qua')}
          </Button>
        </div>
      </motion.div>
    </motion.div>
  )
}

