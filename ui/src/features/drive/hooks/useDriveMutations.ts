import { useCallback } from "react";
import {
  createFolder as createFolderService,
  deleteFile as deleteFileService,
  deleteFolder as deleteFolderService,
  moveFile as moveFileService,
  moveFolder as moveFolderService,
  forwardFileToShared as forwardFileToSharedService,
  purgeFile as purgeFileService,
  restoreFile as restoreFileService,
  toggleStar as toggleStarService,
} from "../services/driveService";
import { toUserMessage } from "../../../shared/services/errors/toUserMessage";
import { getDriveScopeForSection } from "./driveSections";

export function useDriveMutations({
  activeSection,
  applyFilesPatch,
  applyFoldersPatch,
  applyTrashPatch,
  applyStatsPatch,
  getCurrentState,
  refresh,
  refreshInBackground,
  toast,
  requestDeleteConfirmation,
}) {
  const deleteFile = useCallback(
    async (fileId) => {
      const snapshot = getCurrentState();
      const file = snapshot?.files?.find((entry) => entry.id === fileId);

      if (!file) {
        try {
          await deleteFileService(fileId);
          await refresh();
        } catch (error) {
          const message = toUserMessage(error);
          console.error("Loi khi xoa file:", error);
          toast?.show(message.message, "error");
        }
        return;
      }

      applyFilesPatch((previous) => previous.filter((entry) => entry.id !== fileId));
      applyStatsPatch((previous) => ({
        ...previous,
        file_count: Math.max(0, (previous.file_count || 0) - 1),
      }));

      try {
        await deleteFileService(fileId);
        refreshInBackground();
      } catch (error) {
        const message = toUserMessage(error);
        console.error("Loi khi xoa file:", error);
        toast?.show(message.message, "error");
        await refresh();
      }
    },
    [applyFilesPatch, applyStatsPatch, getCurrentState, refresh, refreshInBackground, toast]
  );

  const deleteItem = useCallback(
    async (item) => {
      try {
        if (item.isFolder) {
          if (requestDeleteConfirmation) {
            const confirmed = await requestDeleteConfirmation(item);
            if (!confirmed) return;
          } else if (!globalThis.confirm(`Ban co chac chan muon xoa thu muc "${item.name}" va toan bo noi dung ben trong?`)) {
            return;
          }

          applyFoldersPatch((previous) => previous.filter((entry) => entry.id !== item.id));
          applyStatsPatch((previous) => ({
            ...previous,
            folder_count: Math.max(0, (previous.folder_count || 0) - 1),
          }));

          await deleteFolderService(item.id);
          refreshInBackground();
          return;
        }

        if (item.status === "trashed") {
          if (requestDeleteConfirmation) {
            const confirmed = await requestDeleteConfirmation(item);
            if (!confirmed) return;
          } else if (
            !globalThis.confirm(
              `Bạn có chắc chắn muốn xóa tệp "${item.filename}"? Thao tác này không thể hoàn tác.`
            )
          ) {
            return;
          }

          applyTrashPatch((previous) => previous.filter((entry) => entry.id !== item.id));
          await purgeFileService(item.id);
          refreshInBackground();
          return;
        }

        // Prompt for standard file deletion
        if (requestDeleteConfirmation) {
          const confirmed = await requestDeleteConfirmation(item);
          if (!confirmed) return;
        }
        await deleteFile(item.id);
      } catch (error) {
        const message = toUserMessage(error);
        console.error("Lỗi khi xóa item:", error);
        toast?.show(message.message, "error");
        await refresh();
      }
    },
    [applyFoldersPatch, applyStatsPatch, applyTrashPatch, deleteFile, refresh, refreshInBackground, toast, requestDeleteConfirmation]
  );

  const restoreFile = useCallback(
    async (fileId) => {
      applyTrashPatch((previous) => previous.filter((entry) => entry.id !== fileId));

      try {
        await restoreFileService(fileId);
        refreshInBackground();
      } catch (error) {
        const message = toUserMessage(error);
        console.error("Loi khi khoi phuc tep:", error);
        toast?.show(message.message, "error");
        await refresh();
      }
    },
    [applyTrashPatch, refresh, refreshInBackground, toast]
  );

  const createFolder = useCallback(
    async (name) => {
      if (!name?.trim()) {
        return;
      }

      const snapshot = getCurrentState();
      const parentId = snapshot?.currentFolderId ? Number(snapshot.currentFolderId) : null;
      const driveScope = getDriveScopeForSection(activeSection) || "my";

      try {
        await createFolderService(name, parentId, driveScope);
        await refresh();
      } catch (error) {
        const message = toUserMessage(error);
        console.error("Loi khi tao thu muc:", error);
        toast?.show(message.message, "error");
      }
    },
    [activeSection, getCurrentState, refresh, toast]
  );

  const moveFile = useCallback(
    async (fileId, folderId) => {
      const snapshot = getCurrentState();
      const currentFolderId = snapshot?.currentFolderId ?? null;

      applyFilesPatch((previous) =>
        previous
          .map((entry) => (entry.id === fileId ? { ...entry, folder_id: folderId } : entry))
          .filter((entry) => {
            if (entry.id !== fileId) {
              return true;
            }
            if (currentFolderId == null) {
              return folderId == null;
            }
            return entry.folder_id === currentFolderId;
          })
      );

      try {
        await moveFileService(fileId, folderId);
        refreshInBackground();
      } catch (error) {
        const message = toUserMessage(error);
        console.error("Loi khi di chuyen file:", error);
        toast?.show(message.message, "error");
        await refresh();
      }
    },
    [applyFilesPatch, getCurrentState, refresh, refreshInBackground, toast]
  );

  const moveFolder = useCallback(
    async (folderId, parentId) => {
      applyFoldersPatch((previous) =>
        previous.map((entry) => (entry.id === folderId ? { ...entry, parent_id: parentId } : entry))
      );

      try {
        await moveFolderService(folderId, parentId);
        refreshInBackground();
      } catch (error) {
        const message = toUserMessage(error);
        console.error("Loi khi di chuyen thu muc:", error);
        toast?.show(message.message, "error");
        await refresh();
      }
    },
    [applyFoldersPatch, refresh, refreshInBackground, toast]
  );
  
  const forwardFileToShared = useCallback(
    async (fileId) => {
      const loadingToastId = toast?.show("Đang di chuyển tệp sang Drive công cộng...", "loading");
      try {
        await forwardFileToSharedService(fileId);
        toast?.show("Đã di chuyển tệp thành công!", "success", { id: loadingToastId });
        await refresh();
      } catch (error) {
        const message = toUserMessage(error);
        console.error("Lỗi khi di chuyển sang Drive công cộng:", error);
        toast?.show(message.message || "Không thể di chuyển tệp.", "error", { id: loadingToastId });
        await refresh();
      }
    },
    [refresh, toast]
  );

  const toggleStar = useCallback(
    async (item) => {
      const nextStarred = !item.starred;

      if (item.isFolder) {
        applyFoldersPatch((previous) =>
          previous.map((entry) => (entry.id === item.id ? { ...entry, starred: nextStarred } : entry))
        );
      } else {
        applyFilesPatch((previous) =>
          previous.map((entry) => (entry.id === item.id ? { ...entry, starred: nextStarred } : entry))
        );
        applyTrashPatch((previous) =>
          previous.map((entry) => (entry.id === item.id ? { ...entry, starred: nextStarred } : entry))
        );
      }

      try {
        await toggleStarService(item.id, Boolean(item.isFolder), nextStarred);
        refreshInBackground();
      } catch (error) {
        const message = toUserMessage(error);
        console.error("Lỗi khi cập nhật star:", error);
        toast?.show(message.message, "error");
        await refresh();
      }
    },
    [applyFilesPatch, applyFoldersPatch, applyTrashPatch, refresh, refreshInBackground, toast]
  );

  return {
    createFolder,
    deleteFile,
    deleteItem,
    moveFile,
    moveFolder,
    restoreFile,
    toggleStar,
    forwardFileToShared,
  };
}
