import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@tauri-apps/plugin-dialog", () => ({
  save: vi.fn(),
}));

vi.mock("../../../api/index", () => ({
  DriveApi: {
    queueDownload: vi.fn(),
  },
}));

import { save } from "@tauri-apps/plugin-dialog";
import { DriveApi } from "../../../api/index";
import { selectDownloadPath, startDownload } from './downloadService';

describe("downloadService", () => {
  beforeEach(() => {
    save.mockReset();
    DriveApi.queueDownload.mockReset();
  });

  it("selectDownloadPath uses dialog save", async () => {
    save.mockResolvedValueOnce("C:/out.mp4");
    const res = await selectDownloadPath({ filename: "a.mp4" });
    expect(save).toHaveBeenCalledWith(
      expect.objectContaining({ title: "Save file", defaultPath: "a.mp4" })
    );
    expect(res).toBe("C:/out.mp4");
  });

  it("startDownload queues download job", async () => {
    DriveApi.queueDownload.mockResolvedValueOnce({ id: 1 });
    const job = await startDownload(5, "C:/out.mp4");
    expect(job).toEqual({ id: 1 });
    expect(DriveApi.queueDownload).toHaveBeenCalledWith(5, "C:/out.mp4");
  });
});
