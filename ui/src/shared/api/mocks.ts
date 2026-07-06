import { handleUploadPlanMock } from "../../features/upload/components/uploadPlanMocks";
import type { MpvStatus } from "../../features/player/api/mpv";

type MockArgs = Record<string, unknown>;

const createMockMpvState = (): MpvStatus => ({
  alive: true,
  paused: false,
  position: 120,
  duration: 600,
  volume: 80,
  speed: 1,
  fullscreen: false,
  title: "Testing Mock Video.mp4",
});

const browserMockState = {
  mpv: createMockMpvState(),
};

const clamp = (value: number, min: number, max: number) => Math.min(Math.max(value, min), max);

const toFiniteNumber = (value: unknown, fallback: number): number => {
  const next = Number(value);
  return Number.isFinite(next) ? next : fallback;
};

const snapshotMockMpvState = (): MpvStatus => ({ ...browserMockState.mpv });

export function handleMockCall(cmd: string, args: MockArgs = {}): unknown {
  console.info("[MOCK CALL]:", cmd, args);
  const argsRecord = args;

  const uploadResult = handleUploadPlanMock(cmd, args);
  if (uploadResult !== null) return uploadResult;

  switch (cmd) {
    case "get_all_files_paginated":
      return {
        files: [{
          id: 202,
          name: "Video_Test_Sample.mp4",
          filename: "Video_Test_Sample.mp4",
          extension: "mp4",
          size: 10485760,
          updated_at: new Date().toISOString(),
          starred: false,
          status: "ready",
          video_duration: 600,
        }],
        next_cursor: null,
        has_more: false,
      };

    case "get_folders":
      return { folders: [] };

    case "get_trash_paginated":
      return { files: [], next_cursor: null, has_more: false };

    case "get_stats":
      return { total_size: 10485760, total_files: 1, total_folders: 0, trash_count: 0 };

    case "get_connection_status":
      return {
        discord: { connected: true },
        telegram: { authorized: false },
      };

    case "get_version":
      return { version: "1.0.0-mock" };

    case "get_video_player_config":
      return {
        providers: {
          discord: {
            transfer: { parallel_sends: 4, chunk_mb: 25, batch_size: 4 },
            retry: { send_retries: 3, retry_base_delay_s: 2 },
            limits: { hard_limit_mb: 25, file_limit_mb: 25 },
          },
          telegram: {
            transfer: { parallel_sends: 3, chunk_mb: 500, batch_size: 0 },
            retry: { send_retries: 3, retry_base_delay_s: 2 },
            limits: { hard_limit_mb: 0, file_limit_mb: 2000 },
          },
        },
        stream_buffer_kb: 64,
        low_latency_mode: true,
        back_buffer_length: 90,
      };

    case "get_playback_position":
      return { position: 0 };

    case "mpv_get_status":
      return snapshotMockMpvState();

    case "mpv_play_pause":
      browserMockState.mpv.paused = !browserMockState.mpv.paused;
      return snapshotMockMpvState();

    case "mpv_seek":
      browserMockState.mpv.position = clamp(
        toFiniteNumber(argsRecord.position, browserMockState.mpv.position),
        0,
        browserMockState.mpv.duration
      );
      return snapshotMockMpvState();

    case "mpv_set_volume":
      browserMockState.mpv.volume = clamp(
        toFiniteNumber(argsRecord.volume, browserMockState.mpv.volume),
        0,
        100
      );
      return snapshotMockMpvState();

    case "mpv_set_speed":
      browserMockState.mpv.speed = clamp(
        toFiniteNumber(argsRecord.speed, browserMockState.mpv.speed),
        0.25,
        4
      );
      return snapshotMockMpvState();

    case "mpv_toggle_fullscreen":
      browserMockState.mpv.fullscreen = !browserMockState.mpv.fullscreen;
      return snapshotMockMpvState();

    default:
      return {};
  }
}
