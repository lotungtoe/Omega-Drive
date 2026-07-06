import { useDriveController } from "./useDriveController";

export function useDrive(toast = null, isLite = false, targetFileId = null, requestDeleteConfirmation = null) {
  return useDriveController(toast, isLite, targetFileId, requestDeleteConfirmation);
}
