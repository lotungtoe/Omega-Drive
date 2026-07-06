# API Context

## Purpose
- Map Tauri command -> handler -> feature/service/provider.

## Open When
- Command missing.
- Invoke fails.
- Wrong payload or response shape.
- Handler wiring is wrong.

## Main Files
- `src-tauri/src/app/tauri_app.rs`
- `src-tauri/src/app/bridge.rs`
- `src-tauri/src/app_runtime.rs`
- `src-tauri/src/api/handlers/mod.rs`
- `src-tauri/src/api/handlers/files.rs`
- `src-tauri/src/api/handlers/folders.rs`
- `src-tauri/src/api/handlers/onboarding.rs`
- `src-tauri/src/api/handlers/upload.rs`
- `src-tauri/src/api/handlers/download.rs`
- `src-tauri/src/api/handlers/playback.rs`
- `src-tauri/src/api/handlers/tenants.rs`
- `src-tauri/src/api/handlers/settings.rs`
- `src-tauri/src/api/handlers/diagnostics.rs`
- `src-tauri/src/api/plugins/mod.rs`
- `src-tauri/src/api/plugins/window_bridge.rs`

## Main Flow
- Command registration starts in `src-tauri/src/app/tauri_app.rs`.
- Handler exports are assembled in `src-tauri/src/api/handlers/mod.rs`.
- Runtime plugin wiring is assembled in `src-tauri/src/api/plugins/mod.rs`.
- Files/folders route into `src-tauri/src/features/drive/*`.
- Upload routes into `src-tauri/src/features/upload/*`.
- Download routes into `src-tauri/src/features/download/*`.
- Playback commands route into `src-tauri/src/features/player/*`.
- Tenant/server menu commands route into `src-tauri/src/api/handlers/tenants.rs`:
  - `get_active_tenants`
  - `get_active_tenant`
  - `list_tenants`
  - `rename_tenant_display_name`
  - `switch_tenant`
- `list_tenants` and `get_active_tenants` now return UI-friendly tenant summaries for the frontend picker/manager:
  - tenant identity fields
  - `dbFileName`
  - optional `displayName`
- `rename_tenant_display_name` only updates `tenant_meta.display_name` inside the target tenant DB:
  - it does not rename the `.db` file
  - it does not mutate provider/runtime tenant identity
- Onboarding commands route into `src-tauri/src/api/handlers/onboarding.rs`:
  - `get_onboarding_state`
  - `load_onboarding_destinations`
  - `save_discord_token`
  - `save_telegram_credentials`
  - `send_telegram_login_code`
  - `submit_telegram_login_code`
  - `submit_telegram_password`
  - `create_onboarding_tenant`
- `get_onboarding_state` is now the lightweight onboarding snapshot:
  - token/session presence
  - Telegram authorization state
  - remembered/current tenant selection
  - whether onboarding is still required for the currently relevant scope
  - Telegram auth uses a cached snapshot keyed by the current credential/session identity
  - the auth probe is timeout-bounded and briefly caches a negative result to avoid repeated MTProto reconnect storms
  - no eager Discord guild list / Telegram dialog scan
- `load_onboarding_destinations` is the heavy path:
  - loads Discord guild choices
  - loads Telegram group/channel choices
  - uses a short-lived in-memory Telegram cache keyed by the current credential/session identity
  - invalidates that cache when Telegram credentials change or login/2FA completes
- Legacy shared-drive setup/status still route through diagnostics/env handlers, but they are no longer the primary happy path:
  - `check_shared_drive_status`
  - `setup_shared_drive`
- Native playback data is not returned through Tauri command payloads; it is served by the internal HTTP bridge in `src-tauri/src/features/player/bridge.rs`.
- `open_video_window` / native playback open now performs playback-runtime warmup for chunk-backed files before MPV starts, then hands media bytes to the internal bridge.

## Boundary Notes
- Shared runtime state comes from `src-tauri/src/app_runtime.rs`.
- `AppState.active_tenant` is now part of that shared runtime state.
- Onboarding reconciliation may update `activeDbFiles.my/shared` and rebind the current runtime when a better valid tenant becomes available for the currently mounted scope.
- `resolve_onboarding_state_inner` now applies a scope-aware gate:
  - missing `shared` alone does not force a global onboarding modal while the current `my` scope is already usable
  - explicit `preferredScope` flows can still reopen onboarding for the missing scope
- `AppState.provider_runtime` is now a replaceable runtime handle behind `RwLock`; tenant switching rebinds DB handles first, then swaps provider runtime.
- Telegram provider status for API/health reads is cached inside the provider runtime:
  - `ProviderAdminGateway::connection_status` for Telegram no longer calls MTProto `is_authorized()` directly on every invoke
  - a provider-owned background probe captures cached auth state and emits `OmegaEvent::TelegramConnectionStatusChanged`
  - the probe uses weak references so an old tenant runtime does not leave a zombie loop behind after rebind
- Provider stream contracts now live in `src-tauri/src/core/ports/stream.rs`.
- Provider credentials still come from `bot.env`, but Discord guild / Telegram chat are resolved from the current tenant descriptor.
- `open_video_window` and the `mpv_*` commands control playback; `/raw/:file_id` and playlist routes feed media bytes.
- Seek/index metadata is not persisted through a new API-facing table; runtime-only hints stay inside the player subsystem.
- `forward_file_to_shared` is a compatibility command only; under tenant DB mode it returns a controlled unsupported error instead of mutating row-level scope.

## Verify
- For desktop/native command behavior, run `cargo tauri dev`.
- For build-level coverage, run `cargo tauri build --debug`.
- Browser-only checks can use `npm --prefix ui run dev`, but they do not validate native `invoke` behavior.
- Backend command coverage for the current onboarding/runtime/provider wave is green on both `cargo check --manifest-path src-tauri/Cargo.toml` and `cargo test --manifest-path src-tauri/Cargo.toml`.

## Debug
- Command not registered -> `src-tauri/src/app/tauri_app.rs`
- Handler export mismatch -> `src-tauri/src/api/handlers/mod.rs`
- Wrong payload or response shape -> target handler file
- If many unrelated UI actions no-op together, inspect `ui/src/api/call.js` before debugging individual handlers
