# Config And Env Context

## Purpose
- Map runtime config, env vars, feature flags, and rollout switches.

## Open When
- Startup or bootstrap fails.
- Config value seems ignored.
- Provider credentials are missing.
- Feature-gated behavior is wrong.

## Main Files
- `src-tauri/src/core/config.rs`
- `config.json`
- `bot.env`
- `tenant_registry.json`
- `tg.session.json`
- `.cargo/config.toml`
- `src-tauri/src/main.rs`
- `src-tauri/src/app/bootstrap.rs`
- `src-tauri/binaries/`
- `ui/tsconfig.strict.json`

## Main Items
- Config groups: `data`, `download`, `logging`, `providers`, `ram`, `server`, `startup`, `stream`, `upload`
- Tenant DB storage now lives under `<base_dir>/db/` with filenames:
  - `<scope>__<discordGuildOr0>__<telegramChatOr0>.db`
- Remembered tenant selection now lives in `tenant_registry.json`:
  - `activeDbFiles.my`
  - `activeDbFiles.shared`
  - each value is either a DB filename or `null`
- Tenant identity comes from the DB filename, not from env IDs.
- Telegram MTProto authorization is persisted separately in `tg.session.json`; startup/onboarding auto-migrates the legacy SQLite `tg.session` file into the new JSON-backed session store when present.
- `bot.env` is now edited from the in-app onboarding flow and should contain credentials only, not destination ids.
- Download/player controls:
  - retry, timeout, bandwidth limit
  - prefetch concurrency and debounce
  - MPV cache and demuxer sizing
  - playback RAM budget
- For raw cloud playback, the native player currently applies seek-sensitive caps at runtime even if config defaults are higher:
  - `mpv_cache_secs` capped to `5`
  - `mpv_demuxer_max_mb` capped to `64`
  - `mpv_readahead_secs` capped to `5`
- Upload controls:
  - chunk size
  - parallel sends
  - retry
  - provider-specific transfer limits
- Stream controls:
  - `stream.ram_pool_mb`
  - `stream.video_block_encryption_upload_enabled`
  - `stream.video_block_encryption_key`
  - `stream.video_block_encryption_block_kb`
  - plaintext sparse playback block size is currently a runtime constant (`512KB`), not a user config knob
  - foreground seek window sizing and pre-roll are currently runtime constants in `bridge.rs`, not user config knobs
  - upload-time keyframe extraction currently has no user config knob; it is a built-in background ffprobe pass for video files
- Native player runtime depends on `libmpv` sidecar DLLs under `src-tauri/binaries/`

## Env Vars
- Discord credential: `DISCORD_TOKEN`
- Telegram credentials: `TELEGRAM_PHONE`, `TELEGRAM_API_ID`, `TELEGRAM_API_HASH`
- Telemetry: `SENTRY_DSN`
- Removed from the active config model:
  - `DISCORD_GUILD_ID`
  - `DISCORD_SHARED_GUILD_ID`
  - `TELEGRAM_CHAT_ID`
  - `TELEGRAM_SHARED_CHAT_ID`

## Feature Flags
- `discord`
- `telegram`
- `player`
- `telemetry`
- `zip`

## Strictness Gates
- Rust:
  - crate defaults in `src-tauri/src/lib.rs` and `src-tauri/src/main.rs`
  - `cargo lint-strict` alias from `.cargo/config.toml`
- UI:
  - scoped `checkJs + strict` config in `ui/tsconfig.strict.json`
  - `npm --prefix ui run typecheck`
  - `npm --prefix ui run lint:strict`

## Verify
- Strict Rust lint -> `cargo lint-strict`
- Config parse/load -> `cargo check --manifest-path src-tauri/Cargo.toml`
- Scoped UI strict checks -> `npm --prefix ui run lint:strict` and `npm --prefix ui run typecheck`
- Desktop/runtime availability -> `cargo tauri dev` or `cargo tauri build --debug`
- For feature-flag changes, verify both enabled and disabled paths in the touched area
- `cargo lint-strict` is currently green with some intentionally-kept warnings in refactor-grade code and tests; they are documented rather than force-fixed.

## Debug
- Config parse/load -> `src-tauri/src/core/config.rs`
- Bootstrap/runtime wiring -> `src-tauri/src/app/bootstrap.rs`
- Tenant DB path resolution -> `src-tauri/src/app/paths.rs`, `src-tauri/src/core/tenant.rs`
- Tenant registry load/save -> `src-tauri/src/core/tenant_registry.rs`
- Onboarding credential save + Telegram OTP/2FA flow -> `src-tauri/src/api/handlers/onboarding.rs`
- Telegram session file store + legacy migration -> `src-tauri/src/providers/telegram_session.rs`
- Missing native player runtime -> confirm `player` is enabled and `src-tauri/binaries/` contains the required `libmpv` DLLs
