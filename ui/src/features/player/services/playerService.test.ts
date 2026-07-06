import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("../../../shared/api/call", () => ({
  call: vi.fn(),
}));

import { call } from '../../../shared/api/call';
import {
  loadBootstrapStatus,
  parseBootstrapIssues,
  openVideoWindow,
  getPlaybackPosition,
  clearPlaybackPosition,
  openNativePlayer,
  getPlayerConfig,
  updatePlayerConfig,
  setPlaybackActive,
} from './playerService';

describe("playerService", () => {
  beforeEach(() => {
    call.mockReset();
  });

  it("parseBootstrapIssues returns missing components", () => {
    const issues = parseBootstrapIssues({
      discordConfigured: false,
      ffmpegReady: false,
      ffprobeReady: true,
      nativePlayerReady: false,
    });
    expect(issues.length).toBe(3);
    expect(issues).toContain("thieu native player runtime (mpv)");
  });

  it("parseBootstrapIssues returns empty when all ready", () => {
    const issues = parseBootstrapIssues({
      discordConfigured: true,
      ffmpegReady: true,
      ffprobeReady: true,
      nativePlayerReady: true,
    });
    expect(issues).toEqual([]);
  });

  it("loadBootstrapStatus calls diagnostics endpoint", async () => {
    call.mockResolvedValueOnce({ ok: true });
    const res = await loadBootstrapStatus();
    expect(call).toHaveBeenCalledWith(
      "get_bootstrap_status",
      {},
      { feature: "diagnostics", action: "get_bootstrap_status" }
    );
    expect(res).toEqual({ ok: true });
  });

  it("openVideoWindow calls player endpoint", async () => {
    call.mockResolvedValueOnce(true);
    await openVideoWindow(5, "Video", 12);
    expect(call).toHaveBeenCalledWith(
      "open_video_window",
      { fileId: 5, title: "Video", startPositionSec: 12 },
      { feature: "player", action: "open_video_window" }
    );
  });

  it("getPlaybackPosition calls player endpoint", async () => {
    call.mockResolvedValueOnce({ position: 1 });
    await getPlaybackPosition(6);
    expect(call).toHaveBeenCalledWith(
      "get_playback_position",
      { fileId: 6 },
      { feature: "player", action: "get_playback_position" }
    );
  });

  it("clearPlaybackPosition calls player endpoint", async () => {
    call.mockResolvedValueOnce(true);
    await clearPlaybackPosition(7);
    expect(call).toHaveBeenCalledWith(
      "clear_playback_position",
      { fileId: 7 },
      { feature: "player", action: "clear_playback_position" }
    );
  });

  it("openNativePlayer aliases to openVideoWindow", async () => {
    call.mockResolvedValueOnce(true);
    await openNativePlayer(8, "Title", 12);
    expect(call).toHaveBeenCalledWith(
      "open_video_window",
      { fileId: 8, title: "Title", startPositionSec: 12 },
      { feature: "player", action: "open_video_window" }
    );
  });

  it("player config calls", async () => {
    call.mockResolvedValueOnce({ maxBitrate: 5 });
    await getPlayerConfig();
    expect(call).toHaveBeenCalledWith(
      "get_video_player_config",
      {},
      { feature: "player", action: "get_video_player_config" }
    );

    call.mockResolvedValueOnce(true);
    await updatePlayerConfig({ maxBitrate: 10 });
    expect(call).toHaveBeenCalledWith(
      "update_video_player_config",
      { data: { maxBitrate: 10 } },
      { feature: "player", action: "update_video_player_config" }
    );
  });

  it("setPlaybackActive calls player endpoint", async () => {
    call.mockResolvedValueOnce(true);
    await setPlaybackActive(true, "player");
    expect(call).toHaveBeenCalledWith(
      "playback_active",
      { active: true, windowLabel: "player" },
      { feature: "player", action: "playback_active" }
    );
  });
});
