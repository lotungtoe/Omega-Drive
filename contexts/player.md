# Player Context

## Purpose
- Map native playback, HTTP bridge, raw byte streaming, prefetch, and playback cache flow.

## Open When
- Playlist or segment output is wrong.
- `/raw/:file_id` byte streaming is wrong.
- Seek latency or request storm behavior is bad.
- Native player window or overlay behavior is wrong.

## Main Files
- `src-tauri/src/features/player/mod.rs`
- `src-tauri/src/features/player/bridge.rs`
- `src-tauri/src/features/player/nativeplayer.rs`
- `src-tauri/src/features/player/playback_cache.rs`
- `src-tauri/src/features/player/singleflight.rs`
- `src-tauri/src/features/player/prefetch.rs`
- `src-tauri/src/features/player/segment_telemetry.rs`
- `src-tauri/src/features/player/video_indexer.rs`
- `src-tauri/src/features/player/segmentgen.rs`
- `src-tauri/src/features/player/range_stream.rs`
- `src-tauri/src/features/player/runtime.rs`
- `src-tauri/src/core/ports/stream.rs`
- `src-tauri/src/providers/telegram_real.rs`
- `ui/src/components/player/NativePlayerOverlay.jsx`
- `ui/src/api/mpv.js`
- `ui/src/services/player/playerService.js`
- `ui/src/hooks/drive/usePlaybackLauncher.js`

## Main Flow
- Playback commands enter `src-tauri/src/features/player/mod.rs`.
- Native playback opens from `nativeplayer.rs` using true `libmpv`.
- Internal HTTP bridge in `bridge.rs` serves:
  - `/raw/:file_id` for direct MPV byte-range reads
  - HLS/native playlist routes for secondary playback paths
- DB-backed `hls_init` is no longer part of the playback contract; native open now requires original chunk-backed media.
- `/raw/:file_id` flow is now MPV-first:
  - warm runtime/provider playback metadata before MPV load for chunk-backed files
  - keep that synchronous warmup small enough to avoid blocking MPV startup on whole-file provider metadata fetches
  - record recent MPV seek targets in runtime state
  - build range plan
  - load original part metadata
  - on a recent foreground seek, bypass sparse-first micro-fetch for the first span and fetch a contiguous seek window with pre-roll
  - serve from sparse cache when possible
  - use block-key singleflight for overlapping misses
  - stream the first sparse miss block through immediately when provider/range allow it
  - parallelize tail segments conservatively
- Telegram uses true provider byte-range streaming.
- Telegram playback warmup now:
  - reuses RAM cache first
  - reuses DB-backed metadata cache second
  - caps synchronous network warmup to a hot window
  - continues the remaining warmup in the background
- Discord keeps resolved-URL range streaming.
- Raw cloud playback caps MPV buffering lower than generic defaults to reduce seek latency.
- Plaintext sparse provider fetches now use a larger 512KB block size.
- Foreground seek policy is now intentionally split:
  - plaintext chunk-backed seeks use a contiguous foreground window with pre-roll
  - dead DB `hls_init` paths are no longer part of the native-open contract
- Prefetch can consume best-effort MP4/MKV hints from `video_indexer.rs`.
- Prefetch can also consume persisted `video_keyframes` hint parts when a recent seek target exists.
- `save_playback_history()` now falls back to the incoming duration when deriving `resume_part_index` for fresh `video_files` rows, so resume writes do not get dropped on newly-uploaded media.
- SQLite now has persistent keyframe assist metadata:
  - `video_keyframes(file_id, pts_ms, part_index, part_offset, stream_index)`
  - the bridge only trusts it as a hint
  - if the nearest keyframe maps to the same part as the first seek span, the foreground window anchors from `part_offset`
  - otherwise the bridge falls back to heuristic pre-roll
- `video_indexer.rs` remains runtime-only best-effort intelligence; it is separate from the persistent upload-time keyframe table.

## Verify
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo tauri build --debug`
- For real seek TTFB, rapid seek request storms, and RAM behavior, verify under `cargo tauri dev`
- If overlay or invoke wiring is involved, also open `contexts/ui.md`

## Debug
- Raw range streaming -> `src-tauri/src/features/player/bridge.rs`
- Cache and dedup -> `playback_cache.rs`, `singleflight.rs`, `runtime.rs`
- Prefetch scheduling -> `prefetch.rs`, `segment_telemetry.rs`, `video_indexer.rs`
- Provider range path -> `src-tauri/src/core/ports/stream.rs`, `src-tauri/src/providers/telegram_real.rs`, `src-tauri/src/providers/telegram_provider.rs`
- Native player window/session -> `nativeplayer.rs`
