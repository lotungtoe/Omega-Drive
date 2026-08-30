import { useEffect, useMemo, useRef } from "react";
import { useTranslation } from "react-i18next";
import { useToastState } from "../../../shared/components/Toasts";
import { useDownloadFile } from "../../download/hooks/useDownloadFile";
import { useDriveController } from "../hooks/useDriveController";
import { useDriveEventSubscriptions } from "../hooks/useDriveEventSubscriptions";
import { useMainAppUiState } from "../hooks/useMainAppUiState";
import { usePlaybackLauncher } from "../../player/hooks/usePlaybackLauncher";
import { useStatusMonitor } from "../../download/hooks/useStatusMonitor";
import { useUploadFile } from "../../upload/hooks/useUploadFile";
import {
  DriveControllerContext,
  MainAppUiActionsContext,
  MainAppUiStateContext,
} from "./useMainAppContext";
import { getBootstrapStatus } from "../../diagnostics/services/diagnosticsService";
import { useDeepLink } from "../hooks/useDeepLink";

export function MainAppProvider({ children }) {
  const toast = useToastState();
  const toastApi = useMemo(() => ({ show: toast.show }), [toast.show]);
  const { t } = useTranslation();
  const { uiState, uiActions } = useMainAppUiState();
  const driveController = useDriveController(
    toastApi,
    uiState.isAnyVideoPlaying,
    null,
    uiActions.openDeleteConfirmModal,
    uiState.activeSection,
    uiState.activeDriveRoot
  );
  const { discordOnline, telegramOnline } = useStatusMonitor(uiState.isAnyVideoPlaying);
  const { bootstrapIssues, handlePlay, handlePreview } = usePlaybackLauncher({
    bootstrapStatus: uiState.bootstrapStatus,
    setBootstrapStatus: uiActions.setBootstrapStatus,
    toast: toastApi,
    t,
    openPreview: uiActions.openPreview,
  });
  const { uploadPaths, resumeUpload, importUrl } = useUploadFile(
    driveController.currentFolderId,
    uiState.activeSection,
    driveController.files,
    driveController.trash,
    driveController.refresh,
    toastApi,
    uiActions.openUploadPlanModal
  );
  const { handleDownload } = useDownloadFile(toastApi);
  const uploadPathsRef = useRef(uploadPaths);
  
  useDeepLink(handlePlay, driveController.files);

  useEffect(() => {
    uploadPathsRef.current = uploadPaths;
  }, [uploadPaths]);

  const { removeSession } = useDriveEventSubscriptions({
    isInternalDragging: false,
    setIsDragOver: uiActions.setIsDragOver,
    setIsAnyVideoPlaying: uiActions.setIsAnyVideoPlaying,
    setProgressMap: uiActions.setProgressMap,
    refreshInBackground: driveController.refreshInBackground,
    toast: toastApi,
    t,
    uploadPathsRef,
  });

  const uiStateValue = useMemo(
    () => ({
      ...uiState,
      bootstrapIssues,
      discordOnline,
      telegramOnline,
    }),
    [bootstrapIssues, discordOnline, telegramOnline, uiState]
  );

  const uiActionsValue = useMemo(
    () => ({
      ...uiActions,
      handleDownload,
      handlePlay,
      handlePreview,
      resumeUpload,
      importUrl,
      removeSession,
      toast,
      uploadPaths,
    }),
    [handleDownload, handlePlay, handlePreview, resumeUpload, importUrl, removeSession, toast, uiActions, uploadPaths]
  );

  return (
    <MainAppUiStateContext.Provider value={uiStateValue}>
      <MainAppUiActionsContext.Provider value={uiActionsValue}>
        <DriveControllerContext.Provider value={driveController}>{children}</DriveControllerContext.Provider>
      </MainAppUiActionsContext.Provider>
    </MainAppUiStateContext.Provider>
  );
}
