import { renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../services/playerService", () => ({

  clearPlaybackPosition: vi.fn().mockResolvedValue(true),
  getPlaybackPosition: vi.fn().mockResolvedValue(null),
  loadBootstrapStatus: vi.fn().mockResolvedValue({}),
  openVideoWindow: vi.fn().mockResolvedValue(true),
  parseBootstrapIssues: vi.fn(() => []),
}));

import {

  clearPlaybackPosition,
  getPlaybackPosition,
  loadBootstrapStatus,
  openVideoWindow,
  parseBootstrapIssues,
} from '../services/playerService';
import { usePlaybackLauncher } from './usePlaybackLauncher';

describe("usePlaybackLauncher", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    sessionStorage.setItem("stalledChecked", "true");
  });

  it("opens native player for ready video files", async () => {
    const openPreview = vi.fn();
    const setBootstrapStatus = vi.fn();

    const { result } = renderHook(() =>
      usePlaybackLauncher({
        bootstrapStatus: null,
        setBootstrapStatus,
        toast: { show: vi.fn() },
        t: (key) => key,
        openPreview,
      })
    );

    expect(loadBootstrapStatus).not.toHaveBeenCalled();

    await result.current.handlePlay({ id: 12, filename: "video.mp4", status: "ready" });

    expect(loadBootstrapStatus).toHaveBeenCalledTimes(1);
    expect(setBootstrapStatus).toHaveBeenCalledWith({});
    expect(getPlaybackPosition).toHaveBeenCalledWith(12);
    expect(openVideoWindow).toHaveBeenCalledWith(12, "video.mp4", undefined);
    expect(openPreview).not.toHaveBeenCalled();
  });

  it("keeps warning flow for non-ready video files", async () => {
    const openPreview = vi.fn();
    const toast = { show: vi.fn() };

    const { result } = renderHook(() =>
      usePlaybackLauncher({
        bootstrapStatus: { ffmpegReady: true },
        setBootstrapStatus: vi.fn(),
        toast,
        t: (key) => key,
        openPreview,
      })
    );

    await result.current.handlePlay({ id: 9, filename: "video.mp4", status: "processing" });

    expect(loadBootstrapStatus).not.toHaveBeenCalled();
    expect(openVideoWindow).not.toHaveBeenCalled();
    expect(openPreview).not.toHaveBeenCalled();
    expect(toast.show).toHaveBeenCalledWith("player.videoNotReady", "info");
    expect(parseBootstrapIssues).toHaveBeenCalled();
  });

  it("keeps non-video preview on the modal path", async () => {
    const openPreview = vi.fn();

    const { result } = renderHook(() =>
      usePlaybackLauncher({
        bootstrapStatus: { ffmpegReady: true, nativePlayerReady: true },
        setBootstrapStatus: vi.fn(),
        toast: { show: vi.fn() },
        t: (key) => key,
        openPreview,
      })
    );

    await result.current.handlePreview({ id: 7, filename: "notes.txt", status: "ready" });

    expect(loadBootstrapStatus).not.toHaveBeenCalled();
    expect(openPreview).toHaveBeenCalledWith({ id: 7, filename: "notes.txt", status: "ready" });
    expect(openVideoWindow).not.toHaveBeenCalled();
  });

  it("routes video preview to the native player path", async () => {
    const openPreview = vi.fn();

    const { result } = renderHook(() =>
      usePlaybackLauncher({
        bootstrapStatus: { ffmpegReady: true, nativePlayerReady: true },
        setBootstrapStatus: vi.fn(),
        toast: { show: vi.fn() },
        t: (key) => key,
        openPreview,
      })
    );

    await result.current.handlePreview({ id: 5, filename: "clip.mkv", status: "ready" });

    expect(loadBootstrapStatus).not.toHaveBeenCalled();
    expect(openVideoWindow).toHaveBeenCalledWith(5, "clip.mkv", undefined);
    expect(openPreview).not.toHaveBeenCalled();
    expect(clearPlaybackPosition).not.toHaveBeenCalled();
  });

  it("prefers backend kind over filename extension for video preview routing", async () => {
    const openPreview = vi.fn();

    const { result } = renderHook(() =>
      usePlaybackLauncher({
        bootstrapStatus: { ffmpegReady: true, nativePlayerReady: true },
        setBootstrapStatus: vi.fn(),
        toast: { show: vi.fn() },
        t: (key) => key,
        openPreview,
      })
    );

    await result.current.handlePreview({ id: 15, filename: "stream.bin", kind: "video", status: "ready" });

    expect(loadBootstrapStatus).not.toHaveBeenCalled();
    expect(openVideoWindow).toHaveBeenCalledWith(15, "stream.bin", undefined);
    expect(openPreview).not.toHaveBeenCalled();
  });
});
