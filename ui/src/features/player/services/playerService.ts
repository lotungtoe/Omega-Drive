import { call } from "../../../shared/api/call";
import type { BootstrapStatus, JsonRecord } from "../../../shared/api/types";

export async function loadBootstrapStatus(): Promise<BootstrapStatus> {
  return call("get_bootstrap_status", {}, { feature: "diagnostics", action: "get_bootstrap_status" });
}

export function parseBootstrapIssues(status: BootstrapStatus | null | undefined): string[] {
  if (!status) return [];
  const issues: string[] = [];
  if (!status.discordConfigured) issues.push("chua cau hinh Discord bot");
  if (!status.ffmpegReady) issues.push("thieu FFmpeg runtime");
  if (!status.ffprobeReady) issues.push("thieu ffprobe runtime");
  if (!status.nativePlayerReady) issues.push("thieu native player runtime (mpv)");
  return issues;
}

export async function openVideoWindow(
  fileId: number,
  title: string,
  startPositionSec?: number,
): Promise<unknown> {
  return call(
    "open_video_window",
    { fileId, title, startPositionSec },
    { feature: "player", action: "open_video_window" },
  );
}

export async function getPlaybackPosition(fileId: number): Promise<unknown> {
  return call(
    "get_playback_position",
    { fileId },
    { feature: "player", action: "get_playback_position" },
  );
}

export async function clearPlaybackPosition(fileId: number): Promise<unknown> {
  return call(
    "clear_playback_position",
    { fileId },
    { feature: "player", action: "clear_playback_position" },
  );
}

export async function openNativePlayer(
  fileId: number,
  title: string,
  startPositionSec?: number,
): Promise<unknown> {
  return openVideoWindow(fileId, title, startPositionSec);
}

export async function getPlayerConfig(): Promise<unknown> {
  return call(
    "get_video_player_config",
    {},
    { feature: "player", action: "get_video_player_config" },
  );
}

export async function updatePlayerConfig(data: JsonRecord): Promise<unknown> {
  return call(
    "update_video_player_config",
    { data },
    { feature: "player", action: "update_video_player_config" },
  );
}

export async function setPlaybackActive(active: boolean, windowLabel: string): Promise<unknown> {
  return call(
    "playback_active",
    { active, windowLabel },
    { feature: "player", action: "playback_active" },
  );
}

export async function uploadAudioAttachment(
  videoFileId: number,
  filePath: string,
): Promise<number> {
  return call(
    "upload_audio_attachment",
    { videoFileId, filePath },
    { feature: "player", action: "upload_audio_attachment" },
  ) as Promise<number>;
}

export async function addAudioTrack(
  videoFileId: number,
  audioFileId: number,
): Promise<unknown> {
  return call(
    "add_audio_track",
    { videoFileId, audioFileId },
    { feature: "player", action: "add_audio_track" },
  );
}
