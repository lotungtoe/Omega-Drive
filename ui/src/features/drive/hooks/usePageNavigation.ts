import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import {
  DRIVE_SECTION_HOME,
  DRIVE_SECTION_MY,
  DRIVE_SECTION_RECENT,
  DRIVE_SECTION_SHARED,
  DRIVE_SECTION_STARRED,
  DRIVE_SECTION_TRASH,
  getDriveScopeForSection,
  getItemDriveScope,
  isScopedDriveSection,
} from "./driveSections";

const RECENT_WINDOW_MS = 3 * 24 * 60 * 60 * 1000;

function recentAccessTime(file) {
  const raw = file?.last_accessed_at;
  if (typeof raw === "number") {
    return raw * 1000;
  }
  if (typeof raw === "string" && raw.trim()) {
    const parsedNumber = Number(raw);
    if (Number.isFinite(parsedNumber)) {
      return parsedNumber * 1000;
    }
    const parsedDate = Date.parse(raw);
    return Number.isFinite(parsedDate) ? parsedDate : 0;
  }
  return 0;
}

function recentFilesOnly(files) {
  const cutoff = Date.now() - RECENT_WINDOW_MS;
  return files
    .filter((file) => recentAccessTime(file) >= cutoff)
    .sort((left, right) => recentAccessTime(right) - recentAccessTime(left));
}

export function usePageNavigation(
  activeSection,
  activeDriveRoot = DRIVE_SECTION_MY,
  files,
  trash,
  currentFolderId,
  folders
) {
  const { t } = useTranslation();

  return useMemo(() => {
    let baseFiles = files;
    let pageTitle = t("sidebar.myDrive");
    const scopedDriveScope = getDriveScopeForSection(activeSection);
    const scopedFiles = scopedDriveScope
      ? files.filter((file) => getItemDriveScope(file) === scopedDriveScope)
      : files;
    const scopedFolders = scopedDriveScope
      ? folders.filter((folder) => getItemDriveScope(folder) === scopedDriveScope)
      : folders;
    const recentStarredDriveScope = getDriveScopeForSection(activeDriveRoot);
    const recentStarredFiles = recentStarredDriveScope
      ? files.filter((file) => getItemDriveScope(file) === recentStarredDriveScope)
      : files;
    const recentStarredFolders = recentStarredDriveScope
      ? folders.filter((folder) => getItemDriveScope(folder) === recentStarredDriveScope)
      : folders;

    if (activeSection === DRIVE_SECTION_HOME) {
      const starredItems = [
        ...folders.filter((folder) => folder.starred).map((folder) => ({ ...folder, isFolder: true })),
        ...files.filter((file) => file.starred),
      ].slice(0, 5);

      const recentFiles = recentFilesOnly(files).slice(0, 10);

      const combined = [...starredItems];
      recentFiles.forEach((recentFile) => {
        if (!combined.some((item) => item.id === recentFile.id && !item.isFolder)) {
          combined.push(recentFile);
        }
      });

      baseFiles = combined;
      pageTitle = t("sidebar.home");
    } else if (isScopedDriveSection(activeSection)) {
      const targetFolderId = currentFolderId ? Number(currentFolderId) : null;
      const subFiles = scopedFiles.filter(
        (file) =>
          (targetFolderId === null && !file.folder_id) ||
          (targetFolderId !== null && Number(file.folder_id) === targetFolderId)
      );
      const subFolders = scopedFolders
        .filter(
          (folder) =>
            (targetFolderId === null && !folder.parent_id) ||
            (targetFolderId !== null && Number(folder.parent_id) === targetFolderId)
        )
        .map((folder) => ({ ...folder, isFolder: true }));

      baseFiles = [...subFolders, ...subFiles];
      pageTitle =
        activeSection === DRIVE_SECTION_SHARED ? t("sidebar.sharedDrive") : t("sidebar.myDrive");

      const currentFolder = scopedFolders.find((folder) => Number(folder.id) === targetFolderId);
      if (currentFolder) {
        pageTitle = currentFolder.name;
      }
    } else if (activeSection === DRIVE_SECTION_TRASH) {
      baseFiles = trash;
      pageTitle = t("sidebar.trash");
    } else if (activeSection === DRIVE_SECTION_RECENT) {
      baseFiles = recentFilesOnly(recentStarredFiles);
      pageTitle = t("sidebar.recent");
    } else if (activeSection === DRIVE_SECTION_STARRED) {
      baseFiles = [
        ...recentStarredFolders
          .filter((folder) => folder.starred)
          .map((folder) => ({ ...folder, isFolder: true })),
        ...recentStarredFiles.filter((file) => file.starred),
      ];
      pageTitle = t("sidebar.starred");
    }

    return { baseFiles, pageTitle };
  }, [activeDriveRoot, activeSection, currentFolderId, files, folders, t, trash]);
}
