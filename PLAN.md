# URL Import — File-Based (No Pipe, No FFmpeg)

**Goal:** Fix URL import truncation (4/118 frames) by replacing pipe-based yt-dlp → ffmpeg with two separate file downloads (video + audio). Player (mpv) merges at playback.

**Root Cause:** yt-dlp stdout pipe always outputs mpegts regardless of `--merge-output-format`. Chaining through ffmpeg causes Windows pipe buffer bottleneck (0% CPU, I/O-bound), truncating at ~4 frames / 98KB with `Connection reset by peer`.

## Core Strategy

1. Download `-f bestvideo` (any ext) + `-f bestaudio` (any ext) as independent files
2. Upload video file normally via `run_upload` with `UploadDataSource::File`
3. Upload audio file, link to video via `default_audio` / `audio[]` columns (reuse `upload_audio_attachment` logic)
4. Player (mpv) merges both at playback — supports all codecs/containers
5. Remove all pipe/ffmpeg code from `streaming_importer.rs`

**Status:** IN PROGRESS
