import { useCallback, useEffect, useRef, useState } from "react";
import {
  fetchAllDriveData,
  fetchFileById,
  fetchFilesPaginated,
  fetchRecentFilesPaginated,
  fetchTrashPaginated,
} from "../services/driveService";
import { toUserMessage } from "../../../shared/services/errors/toUserMessage";
import {
  DRIVE_SECTION_MY,
  DRIVE_SECTION_RECENT,
  DRIVE_SECTION_STARRED,
  getDriveScopeForSection,
  isScopedDriveSection,
} from "./driveSections";

export function useDriveQuery(
  toast = null,
  isLite = false,
  targetFileId = null,
  activeSection = "home",
  activeDriveRoot = DRIVE_SECTION_MY
) {
  const [files, setFiles] = useState([]);
  const [folders, setFolders] = useState([]);
  const [trash, setTrash] = useState([]);
  const [currentFolderId, setCurrentFolderId] = useState(null);
  const [loading, setLoading] = useState(false);
  const [stats, setStats] = useState({ total_size: 0, file_count: 0, folder_count: 0 });
  const [filesCursor, setFilesCursor] = useState(null);
  const [filesHasMore, setFilesHasMore] = useState(false);
  const [loadingMore, setLoadingMore] = useState(false);
  const [trashCursor, setTrashCursor] = useState(null);
  const [trashHasMore, setTrashHasMore] = useState(false);
  const [loadingMoreTrash, setLoadingMoreTrash] = useState(false);

  const loadMoreLock = useRef(false);
  const loadMoreTrashLock = useRef(false);
  const snapshotRef = useRef(null);
  const prevActiveSection = useRef(activeSection);

  if (
    prevActiveSection.current !== activeSection &&
    !isScopedDriveSection(activeSection)
  ) {
    setCurrentFolderId(null);
  }
  prevActiveSection.current = activeSection;

  const rootDriveScope = getDriveScopeForSection(activeDriveRoot);
  const driveScope =
    activeSection === DRIVE_SECTION_RECENT || activeSection === DRIVE_SECTION_STARRED
      ? rootDriveScope
      : getDriveScopeForSection(activeSection);

  const refresh = useCallback(async () => {
    if (isLite) {
      if (!targetFileId) {
        return;
      }

      setLoading(true);
      try {
        const result = await fetchFileById(Number(targetFileId));
        if (result?.file) {
          setFiles([result.file]);
        }
      } catch (error) {
        console.error("Lite refresh failed:", error);
      } finally {
        setLoading(false);
      }
      return;
    }

    setLoading(true);
    try {
      const listMode = activeSection === DRIVE_SECTION_RECENT ? "recent" : "all";
      const data = await fetchAllDriveData(driveScope, listMode);
      setFiles(data.files);
      setFilesCursor(data.filesCursor);
      setFilesHasMore(data.filesHasMore);
      setFolders(data.folders);
      setTrash(data.trash);
      setTrashCursor(data.trashCursor);
      setTrashHasMore(data.trashHasMore);
      setStats(data.stats);
    } catch (error) {
      const message = toUserMessage(error);
      console.error("Loi khi cap nhat du lieu Drive:", error);
      toast?.show(message.message, "error");
    } finally {
      setLoading(false);
    }
  }, [activeSection, driveScope, isLite, targetFileId, toast]);

  const refreshInBackground = useCallback(() => {
    void refresh();
  }, [refresh]);

  const loadMore = useCallback(async () => {
    if (!filesHasMore || loadMoreLock.current) {
      return;
    }

    loadMoreLock.current = true;
    setLoadingMore(true);
    try {
      const result =
        activeSection === DRIVE_SECTION_RECENT
          ? await fetchRecentFilesPaginated(filesCursor, 50, driveScope)
          : await fetchFilesPaginated(filesCursor, 50, driveScope);
      const newFiles = result.files || [];
      if (newFiles.length > 0) {
        setFiles((previous) => [...previous, ...newFiles]);
        setFilesCursor(result.next_cursor ?? null);
        setFilesHasMore(result.has_more ?? false);
      } else {
        setFilesHasMore(false);
      }
    } catch (error) {
      console.error("Loi khi tai them file:", error);
    } finally {
      setLoadingMore(false);
      loadMoreLock.current = false;
    }
  }, [activeSection, driveScope, filesCursor, filesHasMore]);

  const loadMoreTrash = useCallback(async () => {
    if (!trashHasMore || loadMoreTrashLock.current) {
      return;
    }

    loadMoreTrashLock.current = true;
    setLoadingMoreTrash(true);
    try {
      const result = await fetchTrashPaginated(trashCursor, 50, driveScope);
      const newFiles = result.files || [];
      if (newFiles.length > 0) {
        setTrash((previous) => [...previous, ...newFiles]);
        setTrashCursor(result.next_cursor ?? null);
        setTrashHasMore(result.has_more ?? false);
      } else {
        setTrashHasMore(false);
      }
    } catch (error) {
      console.error("Loi khi tai them trash:", error);
    } finally {
      setLoadingMoreTrash(false);
      loadMoreTrashLock.current = false;
    }
  }, [driveScope, trashCursor, trashHasMore]);

  const applyFilesPatch = useCallback((updater) => {
    setFiles((previous) => updater(previous));
  }, []);

  const applyFoldersPatch = useCallback((updater) => {
    setFolders((previous) => updater(previous));
  }, []);

  const applyTrashPatch = useCallback((updater) => {
    setTrash((previous) => updater(previous));
  }, []);

  const applyStatsPatch = useCallback((updater) => {
    setStats((previous) => updater(previous));
  }, []);

  const replaceSnapshot = useCallback((snapshot) => {
    if (!snapshot) {
      return;
    }

    if (Array.isArray(snapshot.files)) {
      setFiles(snapshot.files);
    }
    if (Array.isArray(snapshot.folders)) {
      setFolders(snapshot.folders);
    }
    if (Array.isArray(snapshot.trash)) {
      setTrash(snapshot.trash);
    }
    if (snapshot.stats) {
      setStats(snapshot.stats);
    }
    if (Object.prototype.hasOwnProperty.call(snapshot, "currentFolderId")) {
      setCurrentFolderId(snapshot.currentFolderId);
    }
  }, []);

  const getCurrentState = useCallback(() => snapshotRef.current, []);

  useEffect(() => {
    snapshotRef.current = {
      files,
      folders,
      trash,
      stats,
      currentFolderId,
      filesCursor,
      filesHasMore,
      trashCursor,
      trashHasMore,
    };
  }, [
    currentFolderId,
    files,
    filesCursor,
    filesHasMore,
    folders,
    stats,
    trash,
    trashCursor,
    trashHasMore,
  ]);

  useEffect(() => {
    void refresh();
  }, [refresh]);



  return {
    files,
    folders,
    trash,
    stats,
    loading,
    currentFolderId,
    setCurrentFolderId,
    refresh,
    refreshInBackground,
    loadMore,
    loadMoreTrash,
    filesHasMore,
    trashHasMore,
    loadingMore,
    loadingMoreTrash,
    applyFilesPatch,
    applyFoldersPatch,
    applyTrashPatch,
    applyStatsPatch,
    replaceSnapshot,
    getCurrentState,
  };
}
