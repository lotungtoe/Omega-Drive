# Upload Context

## Purpose
- Map upload planning, chunking, provider dispatch, persistence, and derivative flow.

## Open When
- Upload plan or profile is wrong.
- Chunking or retry is wrong.
- Provider send fails.
- Part metadata, persistence, or derivative behavior is wrong.

## Main Files
- `src-tauri/src/features/upload/mod.rs`
- `src-tauri/src/features/upload/transfer.rs`
- `src-tauri/src/features/upload/coordinator.rs`
- `src-tauri/src/features/upload/plan.rs`
- `src-tauri/src/features/upload/persistence.rs`
- `src-tauri/src/features/upload/provider_dispatch.rs`
- `src-tauri/src/features/upload/session.rs`
- `src-tauri/src/features/upload/resolution.rs`
- `src-tauri/src/features/upload/metadata.rs`
- `src-tauri/src/features/upload/derivative_upload.rs`
- `src-tauri/src/api/handlers/upload.rs`

## Main Flow
- Frontend calls `upload_file_from_path` or `upload_file_native`.
- Handler resolves profile/rules, then routes into upload feature code.
- `transfer.rs` handles shared-entry orchestration and high-level batching.
- `coordinator.rs` reads the file and prepares original upload chunks.
- Upload target/folder resolution is now local-folder-only:
  - no remote Discord category mapping
  - current tenant selection determines which DB/server the upload belongs to
- Persisted original parts now use one generic chunk-row model in SQLite; schema-level `part_type` distinctions were removed in this wave.
- `provider_dispatch.rs` sends parts to Telegram and/or Discord.
- `persistence.rs` stores part rows and resume state.
- `upload_jobs.source_path` is the canonical resumable upload source location; persisted upload-job progress is part-based, not byte-based.
- Full-file integrity is now a streaming-finalize concern, not an upfront reservation step:
  - upload UI starts at `preparing`
  - `files.checksum` is persisted only after the original upload path succeeds
- `upload_profile_rules.skip_upload_modal_profile` is now the only skip-upload-modal signal; `user_preferences` is gone.
- For original parts:
  - `size` is remote payload size
  - `byte_length` is logical/plaintext size
- Attachment metadata is no longer modeled through `files.role` / `subtitle_id`; the dedicated `attachments` table owns remote attachment rows.
- Shared Discord inline batching is intentionally bounded because the batch gateway still takes `Vec<u8>` payloads:
  - candidate must stay within `8 MB`
  - each grouped batch is capped at `24 MB` aggregate payload
  - bytes are read once and hashed while filling the upload buffer
- Failure cleanup is best-effort but tighter than before:
  - original upload always closes/awaits workers before bubbling an error
  - single-file/shared-batch failures attempt remote cleanup immediately
  - if a part upload succeeds but DB persistence fails, worker-level inline cleanup now runs before the higher-level record cleanup path
- Upload now also owns background persistent keyframe indexing for video files:
  - `metadata.rs` runs ffprobe keyframe extraction
  - keyframes are mapped into `part_index + part_offset` using the original base chunk size
  - `persistence.rs` replaces rows in `video_keyframes`
  - this work is background/best-effort and does not block upload success
- Derivative and preview uploads remain separate from the raw/original part path.

## Verify
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo tauri build --debug`
- For real provider behavior, verify under `cargo tauri dev`
- If chunk rules or DB state changed, also open `contexts/db.md` and `contexts/config-env.md`

## Debug
- Plan and profile resolution -> `transfer.rs`, `plan.rs`, `resolution.rs`
- Chunk preparation and provider fan-out -> `coordinator.rs`, `provider_dispatch.rs`
- Persistence and resume metadata -> `persistence.rs`, `session.rs`, `src-tauri/src/db/upload_jobs.rs`
- If upload and other unrelated UI actions all no-op together, inspect `ui/src/api/call.js` first
