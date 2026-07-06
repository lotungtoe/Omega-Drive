# Database Context

## Purpose
- Map SQLite schema, migrations, and hot DB access paths.

## Open When
- Schema or migration changes.
- Query or pagination is wrong or slow.
- DB open or migration fails.
- `parts`, `files`, `folders`, or `download_jobs` behavior is wrong.

## Main Files
- `src-tauri/src/db/mod.rs`
- `src-tauri/src/db/migrations.rs`
- `src-tauri/src/db/tenant_meta.rs`
- `src-tauri/src/core/tenant_registry.rs`
- `src-tauri/src/db/upload_jobs.rs`
- `src-tauri/src/db/attachments.rs`
- `src-tauri/src/db/files/mod.rs`
- `src-tauri/src/db/files/parts.rs`
- `src-tauri/src/db/files/playback.rs`
- `src-tauri/src/db/folders.rs`
- `src-tauri/src/db/download_jobs.rs`

## Main Tables
- `tenant_meta`
- `folders`
- `files`
- `parts`
- `video_files`
- `audio_files`
- `image_files`
- `video_keyframes`
- `download_jobs`
- `upload_jobs`
- `attachments`
- `drive_stats_cache`
- `provider_quota_cache`
- `pending_operations`
- `upload_profiles`
- `upload_profile_rules`
- `files_fts`
- `_migrations`

## Main Flow
- `src-tauri/src/db/mod.rs` opens SQLite, enables WAL, and runs migrations.
- In debug builds, `DEBUG=1` enables SQLite statement profiling logs on target `db::sql`; each log line includes elapsed milliseconds after the statement completes.
- `DbWriteQueue` owns the shared writer connection.
- `src-tauri/src/db/migrations.rs` owns the single-version fresh schema and legacy reset logic.
- `tenant_registry.json` outside SQLite remembers which DB file is active for `my` and `shared`.
- `tenant_meta` is now a compatibility shim inside each DB, not the source of truth for tenant identity.
  - it also stores optional `display_name` UI metadata for that tenant DB
- `src-tauri/src/db/files/*` splits file metadata, part metadata, and playback progress logic.

## Current Notes
- SQLite writes are serialized through `DbWriteQueue`; compatibility `.lock().await` and `.blocking_lock()` APIs still exist.
- The app now stores one tenant per DB file:
  - `<scope>__<discordGuildOr0>__<telegramChatOr0>.db`
  - runtime switching reopens all shared DB handles on that path
- Active tenant resolution order is:
  - `tenant_registry.json`
  - if registry remembers a DB filename but the file was deleted, bootstrap still recreates that DB on the same tenant identity
  - otherwise filesystem scan fallback in the same scope
  - safe default DB bootstrap when nothing exists yet
- Onboarding-valid tenant logic is stricter than raw filename discovery:
  - a DB is valid for the current Discord token iff `discordGuildId == 0` or that guild is reachable by the live bot token
  - a DB is valid for the current Telegram session iff `telegramGroupId == 0` or that group/channel is reachable by the live MTProto session
  - `activeDbFiles.my/shared` can be rewritten to `null` when no valid DB remains for that scope
- `download_jobs` persists download target state only.
- `upload_jobs` persists upload source path + part/state progress only:
  - `state` now tracks the real upload lifecycle (`uploading`, `processing`, `done`, `error`)
  - `error` / `error_code` are populated when upload cleanup falls into the failure path
- Recent files are backed by `files.last_accessed_at`.
- Shared-drive sharer sync uses `files.sharer_id`.
- `files.checksum` is no longer expected to exist during target reservation:
  - original upload finalizes the full-file BLAKE3 hash at the end of the raw upload path
  - shared inline Discord batch hashes while reading payload bytes into RAM
- Fresh schema removes row-level `drive_scope`, `files.local_path`, `files.role`, `folders.discord_id`, `parts.part_type`, `parts.duration`, and `parts.attachment_name`.
- `tenant_meta.display_name` is additive and backward-safe:
  - fresh schema includes the column
  - old tenant DBs lazily add `display_name` on first display-name read/write, so renaming a non-active DB does not require a destructive schema reset
- Some high-level callers still receive compatibility fields synthesized from joins/runtime context:
  - `drive_scope` from `tenant_meta.scope`
  - `local_path` from `upload_jobs.source_path`
  - `role = 'main'`
- `parts` now stores only the original/raw payload path needed by current playback/download behavior:
  - fresh schema keeps one generic original-part row model
  - `size` is remote payload size
  - `byte_length` is logical/plaintext size for original parts
  - compatibility helpers currently synthesize `part_type = 'chunk'`
  - `get_original_part_by_index()` is the safe single-part fallback for raw playback lookups
- Media child tables no longer store DB `hls_init`, `subtitle_id`, `encoder`, `completed`, or `playback_updated_at`.
- `video_files` / `image_files` store `resolution` text instead of raw `width` + `height` columns.
- `attachments` is the standalone metadata table for subtitle/dub-style remote objects keyed by `owner_file_id`; it no longer stores a DB timestamp column.
- `video_keyframes` is now the persistent seek-assist table:
  - `file_id`
  - `pts_ms`
  - `part_index`
  - `part_offset`
  - `stream_index`
  - `part_offset` is logical/plaintext offset inside the original part, not ciphertext offset
  - nearest-keyframe lookup is by `(file_id, stream_index, pts_ms)`
- `drive_stats_cache` and `provider_quota_cache` avoid scan-heavy reads.
- `pending_operations` is the durable outbox table for future crash recovery work.

## Verify
- `cargo test --manifest-path src-tauri/Cargo.toml migrations -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- For hot-query/index work, verify with `EXPLAIN QUERY PLAN`
- Current workspace no longer has the `libsql_ffi` vs `libsqlite3_sys` linker conflict; full `cargo test --manifest-path src-tauri/Cargo.toml` is back to being the normal DB/backend gate.

## Debug
- Schema mismatch -> `src-tauri/src/db/migrations.rs`
- File metadata and parts -> `src-tauri/src/db/files/*`
- Upload source tracking -> `src-tauri/src/db/upload_jobs.rs`
- Attachment metadata -> `src-tauri/src/db/attachments.rs`
- Download queue state -> `src-tauri/src/db/download_jobs.rs`
