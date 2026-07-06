import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("../services/errors/reportError", () => ({
  reportError: vi.fn(() => ({ code: "E_TEST", message: "failed" })),
}));

import { call } from './call';
import { invoke } from "@tauri-apps/api/core";
import { reportError } from '../services/errors/reportError';

async function loadCall() {
  vi.resetModules();
  return (await import("./call")).call;
}

describe("call", () => {
  beforeEach(() => {
    invoke.mockReset();
    reportError.mockClear();
    globalThis.__TAURI_INTERNALS__ = {};
  });

  it("returns invoke result", async () => {
    invoke.mockResolvedValueOnce({ ok: true });
    const res = await call("ping");
    expect(res.ok).toBe(true);
  });

  it("reports and throws on error", async () => {
    invoke.mockRejectedValueOnce(new Error("boom"));
    await expect(call("ping", { a: 1 })).rejects.toMatchObject({
      code: "E_TEST",
      message: "failed",
    });
    expect(reportError).toHaveBeenCalled();
  });

  it("returns nested connection status in browser mock mode", async () => {
    delete globalThis.__TAURI_INTERNALS__;
    const browserCall = await loadCall();

    await expect(browserCall("get_connection_status")).resolves.toEqual({
      discord: { authorized: true },
      telegram: { authorized: true },
    });
    expect(invoke).not.toHaveBeenCalled();
  });

  it("mutates mpv browser mock state consistently", async () => {
    delete globalThis.__TAURI_INTERNALS__;
    const browserCall = await loadCall();

    const initial = await browserCall("mpv_get_status");
    expect(initial).toMatchObject({
      alive: true,
      paused: false,
      position: 120,
      duration: 600,
      volume: 80,
      speed: 1,
      fullscreen: false,
      title: "Testing Mock Video.mp4",
    });

    await browserCall("mpv_play_pause");
    await browserCall("mpv_seek", { position: 250 });
    await browserCall("mpv_set_volume", { volume: 55 });
    await browserCall("mpv_set_speed", { speed: 1.5 });
    await browserCall("mpv_toggle_fullscreen");

    await expect(browserCall("mpv_get_status")).resolves.toMatchObject({
      alive: true,
      paused: true,
      position: 250,
      duration: 600,
      volume: 55,
      speed: 1.5,
      fullscreen: true,
      title: "Testing Mock Video.mp4",
    });
    expect(invoke).not.toHaveBeenCalled();
  });

  it("returns null playback history in browser mock mode", async () => {
    delete globalThis.__TAURI_INTERNALS__;
    const browserCall = await loadCall();

    await expect(browserCall("get_playback_position", { fileId: 12 })).resolves.toEqual({
      position: 0,
    });
    expect(invoke).not.toHaveBeenCalled();
  });
});
