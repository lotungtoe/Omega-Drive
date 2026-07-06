import { useMemo } from "react";
import { useDriveMutations } from "./useDriveMutations";
import { useDriveQuery } from "./useDriveQuery";
import { DRIVE_SECTION_MY } from "./driveSections";

export function useDriveController(
  toast = null,
  isLite = false,
  targetFileId = null,
  requestDeleteConfirmation = null,
  activeSection = "home",
  activeDriveRoot = DRIVE_SECTION_MY
) {
  const query = useDriveQuery(toast, isLite, targetFileId, activeSection, activeDriveRoot);
  const mutations = useDriveMutations({
    activeSection,
    toast,
    refresh: query.refresh,
    refreshInBackground: query.refreshInBackground,
    applyFilesPatch: query.applyFilesPatch,
    applyFoldersPatch: query.applyFoldersPatch,
    applyTrashPatch: query.applyTrashPatch,
    applyStatsPatch: query.applyStatsPatch,
    getCurrentState: query.getCurrentState,
    requestDeleteConfirmation,
  });

  return useMemo(
    () => ({
      files: query.files,
      folders: query.folders,
      trash: query.trash,
      stats: query.stats,
      loading: query.loading,
      currentFolderId: query.currentFolderId,
      setCurrentFolderId: query.setCurrentFolderId,
      refresh: query.refresh,
      refreshInBackground: query.refreshInBackground,
      deleteFile: mutations.deleteFile,
      createFolder: mutations.createFolder,
      moveFile: mutations.moveFile,
      moveFolder: mutations.moveFolder,
      loadMore: query.loadMore,
      loadMoreTrash: query.loadMoreTrash,
      filesHasMore: query.filesHasMore,
      trashHasMore: query.trashHasMore,
      loadingMore: query.loadingMore,
      loadingMoreTrash: query.loadingMoreTrash,
      deleteItem: mutations.deleteItem,
      restoreFile: mutations.restoreFile,
      toggleStar: mutations.toggleStar,
      forwardFileToShared: mutations.forwardFileToShared,
    }),
    [mutations, query]
  );
}
