import { useMemo } from "react";
import { buildBreadcrumbs } from "../services/driveUtils";

/**
 * useBreadcrumbs Hook: Tạo danh sách các thư mục cha để hiển thị đường dẫn.
 * Ví dụ: My Drive > Work > 2024
 */
export function useBreadcrumbs(currentFolderId, folders) {
  return useMemo(
    () => buildBreadcrumbs(currentFolderId, folders),
    [currentFolderId, folders]
  );
}
