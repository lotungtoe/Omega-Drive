import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("../../../api/index", () => ({
  DriveApi: {
    getAllFilesPaginated: vi.fn(),
    getFolders: vi.fn(),
    getTrashPaginated: vi.fn(),
    getStats: vi.fn(),
    getFile: vi.fn(),
    createFolder: vi.fn(),
    deleteFile: vi.fn(),
    deleteFolder: vi.fn(),
    moveFile: vi.fn(),
    moveFolder: vi.fn(),
    sync: vi.fn(),
    purgeFile: vi.fn(),
    restoreFile: vi.fn(),
    toggleStar: vi.fn(),
  },
}));

import {
  fetchAllDriveData,
  fetchFileById,
  createFolder,
  deleteFile,
  deleteFolder,
  moveFile,
  moveFolder,
  purgeFile,
  restoreFile,
  toggleStar,
} from './driveService';
import { DriveApi } from "../../../api/index";

describe("driveService", () => {
  beforeEach(() => {
    DriveApi.getAllFilesPaginated.mockReset();
    DriveApi.getFolders.mockReset();
    DriveApi.getTrashPaginated.mockReset();
    DriveApi.getStats.mockReset();
    DriveApi.getFile.mockReset();
    DriveApi.createFolder.mockReset();
    DriveApi.deleteFile.mockReset();
    DriveApi.deleteFolder.mockReset();
    DriveApi.moveFile.mockReset();
    DriveApi.moveFolder.mockReset();
    DriveApi.sync.mockReset();
    DriveApi.purgeFile.mockReset();
    DriveApi.restoreFile.mockReset();
    DriveApi.toggleStar.mockReset();
  });

  it("fetchAllDriveData aggregates data", async () => {
    DriveApi.getAllFilesPaginated.mockResolvedValue({ files: [{ id: 1 }] });
    DriveApi.getFolders.mockResolvedValue({ folders: [{ id: 2 }] });
    DriveApi.getTrashPaginated.mockResolvedValue({ files: [{ id: 3 }] });
    DriveApi.getStats.mockResolvedValue({ total_files: 1, total_folders: 1, total_size: 10, trash_count: 1 });

    const res = await fetchAllDriveData();
    expect(res.files.length).toBe(1);
    expect(res.folders.length).toBe(1);
    expect(res.trash.length).toBe(1);
    expect(res.stats.total_size).toBe(10);
  });

  it("fetchAllDriveData defaults missing fields", async () => {
    DriveApi.getAllFilesPaginated.mockResolvedValue({});
    DriveApi.getFolders.mockResolvedValue({});
    DriveApi.getTrashPaginated.mockResolvedValue({});
    DriveApi.getStats.mockResolvedValue({});

    const res = await fetchAllDriveData();
    expect(res.files).toEqual([]);
    expect(res.folders).toEqual([]);
    expect(res.trash).toEqual([]);
    expect(res.stats).toEqual({
      total_size: 0,
      file_count: 0,
      folder_count: 0,
      trash_count: 0,
    });
  });

  it("fetchFileById passes numeric id", async () => {
    DriveApi.getFile.mockResolvedValue({ file: { id: 9 } });
    await fetchFileById("9");
    expect(DriveApi.getFile).toHaveBeenCalledWith(9);
  });

  it("createFolder uses parentId default", async () => {
    DriveApi.createFolder.mockResolvedValue({ folder: { id: 1 } });
    await createFolder("Test");
    expect(DriveApi.createFolder).toHaveBeenCalledWith("Test", null, null);
  });

  it("delete/move/sync call through", async () => {
    DriveApi.deleteFile.mockResolvedValue(true);
    DriveApi.deleteFolder.mockResolvedValue(true);
    DriveApi.moveFile.mockResolvedValue(true);
    DriveApi.moveFolder.mockResolvedValue(true);
    DriveApi.sync.mockResolvedValue({ updatedCount: 0 });

    await deleteFile(1);
    await deleteFolder(2);
    await moveFile(3, 4);
    await moveFolder(5, 6);
    expect(DriveApi.deleteFile).toHaveBeenCalledWith(1);
    expect(DriveApi.deleteFolder).toHaveBeenCalledWith(2);
    expect(DriveApi.moveFile).toHaveBeenCalledWith(3);
    expect(DriveApi.moveFolder).toHaveBeenCalledWith(5, 6);
  });

  it("purge/restore/toggleStar call through", async () => {
    DriveApi.purgeFile.mockResolvedValue(true);
    DriveApi.restoreFile.mockResolvedValue(true);
    DriveApi.toggleStar.mockResolvedValue(true);

    await purgeFile(10);
    await restoreFile(11);
    await toggleStar(12, true, false);

    expect(DriveApi.purgeFile).toHaveBeenCalledWith(10);
    expect(DriveApi.restoreFile).toHaveBeenCalledWith(11);
    expect(DriveApi.toggleStar).toHaveBeenCalledWith(12, true, false);
  });
});
