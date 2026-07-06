# URL Import + Audio Fix — Tasks

> Plan: `PLAN.md`

## Phase 1: Readiness & Backup

- [x] Investigate root cause (pipe truncation, mpegts forced, race conditions)
- [x] Confirm approach (file-based download, no pipe, no ffmpeg)
- [x] Snapshot current worktree

## Phase 2: Rewrite `streaming_importer.rs`

- [x] Remove `Fmp4ImportResult` / pipe chain
- [x] Remove `download_to_file` (dead code)
- [x] Add `ImportResult` with `video_path`, `audio_path`
- [x] `start_import_stream` downloads `-f bestvideo` + `-f bestaudio` as files

## Phase 3: Update `external_import.rs` Handler

- [x] Replace `run_streaming_upload` with `run_upload(File)` for video
- [x] Upload audio with `attachment_parent` + `toggle_hidden` + `attach_audio_files`
- [x] Remove `MeteredStream` / `Arc<AtomicU64>` / progress monitoring
- [x] Cleanup temp files after uploads

## Phase 4: Fix audio không phát

- [x] Phase 5b load `default_audio` độc lập (không phụ thuộc `audio[]`)
- [x] Auto-save: thêm file_id vào `audio[]` khi lưu `default_audio`
- [x] Deferred `audio-add`: đợi `duration` property fire → file đã load xong mới add track
- [x] Log lỗi `audio-add` thay vì silent discard

## Phase 5: Verify

- [x] `cargo check` — clean compile
