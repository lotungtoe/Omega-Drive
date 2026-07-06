# Download Context

## Purpose
- Map queueing, fetch, assembly, resume, and integrity flow.

## Open When
- Queue state is wrong.
- Download stalls or retries forever.
- Output file is corrupt.
- Resume or byte assembly is wrong.

## Main Files
- `src-tauri/src/features/download/mod.rs`
- `src-tauri/src/features/download/manager.rs`
- `src-tauri/src/features/download/context.rs`
- `src-tauri/src/api/handlers/download.rs`

## Main Flow
- Frontend calls `download_file_to_disk` or `queue_download`.
- Handler creates or queues `DownloadJob`.
- `DownloadManager` drives the queue and cancellation state.
- `DownloadContext` now holds the shared provider-runtime lock instead of a one-time runtime snapshot, so background downloads keep following tenant runtime rebinds.
- `download_jobs.target_path` remains download-only and is intentionally separate from `upload_jobs.source_path`.
- `run_download_job()` and `run_download()` load file metadata plus original parts through `get_original_parts_for_file()`.
- Duplicate parts are deduped; Telegram still wins over Discord when both exist.
- Part assembly now uses logical size from `parts.byte_length` when present.
- Original part payloads are assembled from the generic persisted chunk rows in `parts`.
- Partial resume stays enabled for the current chunk-backed download flow.
- Download still does not depend on a persistent keyframe table; seek/index hints are player-only.

## Verify
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo tauri build --debug`
- For real provider behavior, verify under `cargo tauri dev`

## Debug
- Queue and lifecycle -> `src-tauri/src/features/download/manager.rs`
- Resume and assembly -> `src-tauri/src/features/download/mod.rs`
- If download state also depends on schema or part metadata, open `contexts/db.md`
