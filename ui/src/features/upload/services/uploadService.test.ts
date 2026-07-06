import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
  ask: vi.fn(),
}));

vi.mock("../../../api/index", () => ({
  DriveApi: {
    purgeFile: vi.fn(),
    uploadFile: vi.fn(),
    uploadFilesFromPaths: vi.fn(),
  },
}));

import { open, ask } from "@tauri-apps/plugin-dialog";
import {
  buildUploadEntries,
  selectFiles,
  confirmOverwrite,
  processPurge,
  executeUploadLoop,
  showUploadSummary,
  resumeUploadByPath,
} from './uploadService';
import { DriveApi } from "../../../api/index";

describe("uploadService", () => {
  beforeEach(() => {
    DriveApi.uploadFile.mockReset();
    DriveApi.uploadFilesFromPaths.mockReset();
    DriveApi.purgeFile.mockReset();
    open.mockReset();
    ask.mockReset();
  });

  it("buildUploadEntries detects collisions in current files", () => {
    const entries = buildUploadEntries(
      ["C:/a.mp4", "C:/b.mp4"],
      [{ id: 1, filename: "a.mp4" }],
      [],
      null
    );
    expect(entries[0].collidingFile).toBeTruthy();
    expect(entries[1].collidingFile).toBeFalsy();
  });

  it("buildUploadEntries detects collisions in trash by folder", () => {
    const entries = buildUploadEntries(
      ["C:/a.mp4"],
      [],
      [{ id: 2, filename: "a.mp4", folder_id: 5 }],
      5
    );
    expect(entries[0].collidingFile?.id).toBe(2);
  });

  it("selectFiles normalizes dialog result", async () => {
    open.mockResolvedValueOnce(null);
    expect(await selectFiles()).toBeNull();
    expect(open).toHaveBeenLastCalledWith(
      expect.objectContaining({ title: "Chọn tệp tin để tải lên" })
    );

    open.mockResolvedValueOnce("C:/a.mp4");
    expect(await selectFiles()).toEqual(["C:/a.mp4"]);

    open.mockResolvedValueOnce(["C:/a.mp4", "C:/b.mp4"]);
    expect(await selectFiles()).toEqual(["C:/a.mp4", "C:/b.mp4"]);
  });

  it("confirmOverwrite forwards prompt to dialog", async () => {
    ask.mockResolvedValueOnce(true);
    const result = await confirmOverwrite([
      { filename: "a.mp4", collidingFile: { id: 1 } },
      { filename: "b.mp4", collidingFile: { id: 2 } },
    ]);
    expect(result).toBe(true);
    expect(ask).toHaveBeenCalledTimes(1);
    const [message, options] = ask.mock.calls[0];
    expect(message).toContain("Phát hiện 2 tệp trùng lặp");
    expect(message).toContain("\"a.mp4\"");
    expect(options).toMatchObject({ kind: "warning", title: "Ghi đè tệp trùng lặp" });
  });

  it("processPurge dedupes by file id and collects failures", async () => {
    DriveApi.purgeFile.mockResolvedValueOnce(true).mockRejectedValueOnce(new Error("fail"));
    const blocked = await processPurge([
      { filename: "a.mp4", collidingFile: { id: 1 } },
      { filename: "a-dup.mp4", collidingFile: { id: 1 } },
      { filename: "b.mp4", collidingFile: { id: 2 } },
    ]);

    expect(DriveApi.purgeFile).toHaveBeenCalledTimes(2);
    expect(blocked.has("b.mp4")).toBe(true);
  });

  it("executeUploadLoop counts started/skipped/failed", async () => {
    DriveApi.uploadFilesFromPaths.mockRejectedValueOnce(new Error("batch-fail"));
    DriveApi.uploadFile.mockResolvedValueOnce(true).mockRejectedValueOnce(new Error("fail"));
    const entries = [
      { path: "a", filename: "a", collidingFile: null },
      { path: "b", filename: "b", collidingFile: null },
    ];

    const result = await executeUploadLoop(entries, true, new Set(), null);
    expect(result.started).toBe(1);
    expect(result.failedToStart.length).toBe(1);
  });

  it("executeUploadLoop batches entries with the same selection", async () => {
    DriveApi.uploadFilesFromPaths.mockResolvedValueOnce(true);
    const entries = [
      { path: "a", filename: "a", collidingFile: null },
      { path: "b", filename: "b", collidingFile: null },
    ];

    const planByPath = new Map([
      ["a", { profileId: 7, uploadPlan: { originalUpload: { providers: ["discord"] } } }],
      ["b", { profileId: 7, uploadPlan: { originalUpload: { providers: ["discord"] } } }],
    ]);

    const result = await executeUploadLoop(entries, true, new Set(), null, null, planByPath);
    expect(result.started).toBe(2);
    expect(result.failedToStart).toEqual([]);
    expect(DriveApi.uploadFilesFromPaths).toHaveBeenCalledWith(
      ["a", "b"],
      null,
      null,
      expect.stringMatching(/^upb-/),
      7,
      { originalUpload: { providers: ["discord"] } }
    );
    expect(DriveApi.uploadFile).not.toHaveBeenCalled();
  });

  it("executeUploadLoop skips collisions and blocked entries", async () => {
    DriveApi.uploadFile.mockResolvedValueOnce(true);
    const entries = [
      { path: "a", filename: "a", collidingFile: { id: 1 } },
      { path: "b", filename: "b", collidingFile: null },
      { path: "c", filename: "c", collidingFile: null },
    ];

    const result = await executeUploadLoop(entries, false, new Set(["c"]), "5");
    expect(result.skipped).toBe(1);
    expect(result.started).toBe(1);
    expect(result.failedToStart).toEqual(["c"]);
    expect(DriveApi.uploadFile).toHaveBeenCalledWith("b", 5, undefined, expect.any(String), null, null);
  });

  it("showUploadSummary reports counts", () => {
    const toast = { show: vi.fn() };
    showUploadSummary(toast, {
      started: 2,
      skipped: 1,
      failedToStart: ["a.mp4", "b.mp4", "c.mp4", "d.mp4"],
    });
    expect(toast.show).toHaveBeenCalledWith("Đã bắt đầu tải lên 2 tệp.", "success");
    expect(toast.show).toHaveBeenCalledWith("Đã bỏ qua 1 tệp trùng lặp.", "info");
    expect(toast.show).toHaveBeenCalledWith(
      expect.stringContaining("Không thể tải lên 4 tệp"),
      "error"
    );
  });

  it("resumeUploadByPath validates input and triggers upload", async () => {
    await expect(resumeUploadByPath(null)).rejects.toThrow("Missing local_path");
    DriveApi.uploadFile.mockResolvedValueOnce(true);
    await resumeUploadByPath({ local_path: "C:/a.mp4", folder_id: 7 });
    expect(DriveApi.uploadFile).toHaveBeenCalledWith(
      "C:/a.mp4",
      7,
      "my",
      expect.stringMatching(/^resume-/),
      null,
      null
    );
  });
});
