import { useMemo } from "react";
import { buildBreadcrumbs } from "../services/driveUtils";

/**
 * useBreadcrumbs Hook: Build list of parent folders for breadcrumb navigation.
 * E.g.: My Drive > Work > 2024
 */
export function useBreadcrumbs(currentFolderId, folders) {
  return useMemo(
    () => buildBreadcrumbs(currentFolderId, folders),
    [currentFolderId, folders]
  );
}
