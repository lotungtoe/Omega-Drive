# UI Context

## Purpose
- Map frontend entry points, invoke wrappers, player launch flow, and verification commands.

## Open When
- Frontend invoke fails.
- UI flow or component wiring is wrong.
- Native player launch path is wrong.
- Need frontend build or lint commands.

## Main Files
- `ui/src/main.tsx`
- `ui/src/App.tsx`
- `ui/src/api/index.ts`
- `ui/src/api/call.ts`
- `ui/src/api/mpv.ts`
- `ui/src/components/modals/ProviderOnboardingModal.tsx`
- `ui/src/pages/main-app/MainAppContent.tsx`
- `ui/src/components/drive/Sidebar.tsx`
- `ui/src/services/featureLog.ts`
- `ui/src/services/errors/normalizeError.ts`
- `ui/src/services/errors/reportError.ts`
- `ui/src/services/player/playerService.ts`
- `ui/src/hooks/drive/usePlaybackLauncher.ts`
- `ui/src/hooks/drive/useMainAppUiState.ts`
- `ui/src/components/modals/PreviewModal.tsx`
- `ui/src/components/player/NativePlayerOverlay.tsx`
- `ui/tsconfig.json`
- `ui/tsconfig.strict.json`

## Main Flow
- Most UI code does not call `invoke()` directly.
- `ui/src/api/call.ts` wraps `invoke()` and normalizes frontend errors.
- Browser mock behavior still lives in `ui/src/api/call.ts` when `window.__TAURI_INTERNALS__` is missing.
- `ui/src/api/index.ts` exports grouped backend access.
- `MainAppContent.tsx` loads `get_onboarding_state` on mount and decides whether the onboarding overlay should be visible.
- `MainAppContent.tsx` now also owns the tenant DB dropdown shown below the breadcrumb area for the current scoped drive root.
- `MainAppContent.tsx` now also owns the scope-specific tenant manager modal:
  - double-click `Drive của tôi` or `Drive công cộng` in the sidebar to open it
  - it lists only that scope's DBs
  - it allows rename + switch without touching the tenant DB filename
- `ProviderOnboardingModal.tsx` owns:
  - Discord token save
  - Telegram credential save
  - Telegram login-code submit
  - optional Telegram 2FA password submit
  - tenant creation for `my` / `shared` when no valid DB exists
  - it only self-refreshes onboarding state when the parent has not already provided one
  - it lazily calls `load_onboarding_destinations` only when the tenant-selection UI is actually needed
- Sidebar root switching now calls:
  - `get_active_tenants`
  - `switch_tenant`
- Tenant DB list rendering moved out of the sidebar:
  - `MainAppContent.tsx` calls `list_tenants`
  - the dropdown filters DBs by the current scope (`my` or `shared`)
  - the dropdown prefers `displayName` when present
  - the `+` button beside the dropdown reopens onboarding/setup for that scope
- Sidebar still remembers one active DB for `My Drive` and one active DB for `Shared Drive`; it does not persist a DB list in JSON.
- Sidebar now prefers onboarding-valid tenants:
  - if a scope already has a remembered valid tenant, clicking the root switches to it
  - if a scope has no valid active DB, clicking `My Drive` or `Shared Drive` only navigates there; it no longer auto-opens onboarding
  - double-clicking those roots opens the tenant manager modal for that scope instead
- `Bỏ qua` on the onboarding modal only dismisses the overlay for the current UI session; manual scope clicks can reopen it later.
- The global onboarding overlay should no longer appear just because `Shared Drive` has no tenant while the active `My Drive` scope is already usable.
- Upload modal skip behavior no longer reads/saves `user_preferences`; it relies on per-rule `skipUploadModalProfile` from batch resolution.
- Video `play` and `preview` route through player service and playback launcher into the native backend-owned playback path.
- `NativePlayerOverlay.tsx` talks to `mpv_*` commands directly.
- UI does not own MPV byte streaming; it only launches and controls the backend/native player session.
- For chunk-backed cloud playback, the backend now warms runtime/provider seek metadata before the MPV session starts; UI contracts did not change.
- The breadcrumb-area dropdown is now the primary way to change the DB inside the currently active `My Drive` or `Shared Drive` scope.
- Legacy `forwardFileToShared()` remains in the API surface, but backend tenant mode now rejects it with a controlled unsupported error.
- App-owned frontend source under `ui/src/**` is now `ts/tsx` with extensionless internal imports.
- `ui/index.html` now boots `/src/main.tsx`, and `ui/vite.config.ts` is the active config.
- Vendored browser libraries under `ui/public/lib/**` remain JS and are outside this migration scope.
- UI type-checking now uses:
  - `ui/tsconfig.json` for the repo-wide compile contract
  - `ui/tsconfig.strict.json` for the focused high-signal typecheck gate
- `ui/tsconfig.strict.json` now enables:
  - `strict`
  - `noUncheckedIndexedAccess`
  - `exactOptionalPropertyTypes`
  - `useUnknownInCatchVariables`
  - `noImplicitReturns`
  - `noFallthroughCasesInSwitch`
- The focused strict gate currently covers:
  - `src/api/**/*.ts`
  - `src/services/**/*.ts`
  - `src/utils/**/*.ts`
  - `src/lang/**/*.ts`
  - `src/debug/uploadPlanMocks.ts`
  - `vite.config.ts`

## Verify
- `npm --prefix ui run dev`
- `npm --prefix ui run build`
- `npm --prefix ui run lint`
- `npm --prefix ui run lint:strict`
- `npm --prefix ui run test`
- `npm --prefix ui run typecheck`
- For real desktop invoke, window bridge, and MPV behavior, verify under `cargo tauri dev`
- Current UI verify status is green for `lint`, `test`, `typecheck`, and `build`.
- `lint:strict` now resolves the same `ts/tsx` app surface through the TypeScript-aware ESLint config.
- The remaining frontend build warning is Vite chunk-size guidance for later bundle-splitting work, not a broken UI build.

## Debug
- Invoke and response shape -> `ui/src/api/call.ts`, `ui/src/api/index.ts`
- Frontend feature log -> `ui/src/services/featureLog.ts`
- If many unrelated clicks do nothing, first check `Boolean(window.__TAURI_INTERNALS__)` in DevTools
- Native overlay/player wiring -> `ui/src/components/player/NativePlayerOverlay.tsx`, `ui/src/api/mpv.ts`
