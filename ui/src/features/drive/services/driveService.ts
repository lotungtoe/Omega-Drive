import type { DriveRecord } from "../../../shared/api/types";
import {
  DriveApi,
  type DriveScope,
  type FoldersResponse,
  type IdLike,
  type Nullable,
  type PaginatedFilesResponse,
  type StatsResponse,
} from "../../../api/index";

type DriveMode = "all" | "recent";

type DriveStats = {
  total_size: number;
  file_count: number;
  folder_count: number;
  trash_count: number;
};

type DriveDataSnapshot = {
  files: DriveRecord[];
  filesCursor: string | null;
  filesHasMore: boolean;
  folders: DriveRecord[];
  trash: DriveRecord[];
  trashCursor: string | null;
  trashHasMore: boolean;
  stats: DriveStats;
};

export async function fetchAllDriveData(
  driveScope: Nullable<DriveScope> = null,
  mode: DriveMode = "all",
): Promise<DriveDataSnapshot> {
  const filesRequest =
    mode === "recent"
      ? DriveApi.getRecentFilesPaginated(null, 50, driveScope)
      : DriveApi.getAllFilesPaginated(null, 50, driveScope);
  const [filesRes, foldersRes, trashRes, statsRes] = (await Promise.all([
    filesRequest,
    DriveApi.getFolders(driveScope),
    DriveApi.getTrashPaginated(null, 50, driveScope),
    DriveApi.getStats(driveScope),
  ])) as [PaginatedFilesResponse, FoldersResponse, PaginatedFilesResponse, StatsResponse];

  return {
    files: filesRes.files || [],
    filesCursor: filesRes.next_cursor ?? null,
    filesHasMore: filesRes.has_more ?? false,
    folders: foldersRes.folders || [],
    trash: trashRes.files || [],
    trashCursor: trashRes.next_cursor ?? null,
    trashHasMore: trashRes.has_more ?? false,
    stats: {
      total_size: statsRes.total_size || 0,
      file_count: statsRes.total_files || 0,
      folder_count: statsRes.total_folders || 0,
      trash_count: statsRes.trash_count || 0,
    },
  };
}

export async function fetchFilesPaginated(
  cursor: string | null,
  limit = 50,
  driveScope: Nullable<DriveScope> = null,
): Promise<PaginatedFilesResponse> {
  return DriveApi.getAllFilesPaginated(cursor, limit, driveScope);
}

export async function fetchRecentFilesPaginated(
  cursor: string | null,
  limit = 50,
  driveScope: Nullable<DriveScope> = null,
): Promise<PaginatedFilesResponse> {
  return DriveApi.getRecentFilesPaginated(cursor, limit, driveScope);
}

export async function fetchTrashPaginated(
  cursor: string | null,
  limit = 50,
  driveScope: Nullable<DriveScope> = null,
): Promise<PaginatedFilesResponse> {
  return DriveApi.getTrashPaginated(cursor, limit, driveScope);
}

export async function fetchTransfersPaginated(cursor: string | null, limit = 50): Promise<unknown> {
  return DriveApi.getTransfersPaginated(cursor, limit);
}

export async function fetchFileById(fileId: IdLike): Promise<unknown> {
  return DriveApi.getFile(typeof fileId === "string" ? Number(fileId) : fileId);
}

export async function createFolder(
  name: string,
  parentId: Nullable<number> = null,
  driveScope: Nullable<DriveScope> = null,
): Promise<unknown> {
  return DriveApi.createFolder(name, parentId, driveScope);
}

export async function deleteFile(fileId: IdLike): Promise<unknown> {
  return DriveApi.deleteFile(fileId);
}

export async function deleteFolder(folderId: IdLike): Promise<unknown> {
  return DriveApi.deleteFolder(folderId);
}

export async function moveFile(fileId: IdLike, folderId: Nullable<number>): Promise<unknown> {
  return DriveApi.moveFile(fileId, folderId);
}

export async function moveFolder(folderId: IdLike, parentId: Nullable<number>): Promise<unknown> {
  return DriveApi.moveFolder(folderId, parentId);
}

export async function forwardFileToShared(fileId: IdLike): Promise<unknown> {
  return DriveApi.forwardFileToShared(fileId);
}

export async function purgeFile(fileId: IdLike): Promise<unknown> {
  return DriveApi.purgeFile(fileId);
}

export async function restoreFile(fileId: IdLike): Promise<unknown> {
  return DriveApi.restoreFile(fileId);
}

export async function toggleStar(id: IdLike, isFolder: boolean, starred: boolean): Promise<unknown> {
  return DriveApi.toggleStar(id, isFolder, starred);
}
