import { Suspense, lazy, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { DndContext, DragOverlay, PointerSensor, useSensor, useSensors } from "@dnd-kit/core";
import { snapCenterToCursor } from "@dnd-kit/modifiers";
import { useTranslation } from "react-i18next";
import { Upload } from "lucide-react";
import { FileIcon, FolderIcon } from "../../../shared/components/Icons";
import { ProgressOverlay } from "../../../shared/components/ProgressOverlay";
import { OverlayLoader } from "../../../shared/components/OverlayLoader";
import { ToastContainer, ToastCtx } from "../../../shared/components/Toasts";
import { FileGrid } from "../components/FileGrid";
import { Header } from "../components/Header";
import { Sidebar } from "../components/Sidebar";
import { TenantScopeDropdown } from "../components/TenantScopeDropdown";
import { DriveApi } from "../../../api/index";
import { useBreadcrumbs } from "../hooks/useBreadcrumbs";
import { useFileFiltering } from "../hooks/useFileFiltering";
import { useFileSorting } from "../hooks/useFileSorting";
import { useKeyboardActions } from "../hooks/useKeyboardActions";
import { useWindowStateTracker } from "../../../shared/hooks/useWindowStateTracker";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { usePageNavigation } from "../hooks/usePageNavigation";
import {
  DRIVE_SECTION_MY,
  DRIVE_SECTION_SHARED,
  getItemDriveScope,
  getRootDriveSection,
  isScopedDriveSection,
} from "../hooks/driveSections";

import { toUserMessage } from "../../../shared/services/errors/toUserMessage";
import { openBotEnv, openDiscordAuth } from "../../diagnostics/services/diagnosticsService";
import { BtnNewFolder } from "../../../shared/ui/atoms/BtnNewFolder";
import { BtnUpload } from "../../../shared/ui/atoms/BtnUpload";
import { BtnViewToggle } from "../../../shared/ui/atoms/BtnViewToggle";
import { BreadcrumbItem } from "../../../shared/ui/atoms/BreadcrumbItem";
import TransfersPage from "../../download/pages/TransfersPage";
import { GlobalAudioBridge } from "../../player/components/GlobalAudioBridge";
import { MiniAudioPlayer } from "../../player/components/MiniAudioPlayer";
import {
  useDriveControllerContext,
  useMainAppUiActions,
  useMainAppUiStateContext,
} from "./useMainAppContext";

const LazySettingsModal = lazy(() =>
  import("../../settings/components/SettingsModal").then((module) => ({ default: module.SettingsModal }))
);
const LazyNewFolderModal = lazy(() =>
  import("../components/NewFolderModal").then((module) => ({ default: module.NewFolderModal }))
);
const LazyPreviewModal = lazy(() =>
  import("../../preview/components/PreviewModal").then((module) => ({ default: module.PreviewModal }))
);
const LazyUploadPlanModal = lazy(() =>
  import("../../upload/components/UploadPlanModal").then((module) => ({ default: module.UploadPlanModal }))
);
const LazyDeleteConfirmModal = lazy(() =>
  import("../components/DeleteConfirmModal").then((module) => ({ default: module.DeleteConfirmModal }))
);
const LazyProviderOnboardingModal = lazy(() =>
  import("../components/ProviderOnboardingModal").then((module) => ({ default: module.ProviderOnboardingModal }))
);
const LazyTenantManagerModal = lazy(() =>
  import("../components/TenantManagerModal").then((module) => ({ default: module.TenantManagerModal }))
);
const LazyUrlImportModal = lazy(() =>
  import("../components/UrlImportModal").then((module) => ({ default: module.UrlImportModal }))
);

function MorphingDragOverlay({ activeDragData, dark }) {
  const [isPill, setIsPill] = useState(false);

  useEffect(() => {
    // Activate morph after 1 frame so CSS transition can catch start bounds
    const frame = requestAnimationFrame(() => {
      requestAnimationFrame(() => setIsPill(true));
    });
    return () => cancelAnimationFrame(frame);
  }, []);

  if (!activeDragData) return null;

  const isGrid = activeDragData.view === "grid";
  const startWidth = activeDragData.rect?.width || (isGrid ? 180 : 600);
  const startHeight = activeDragData.rect?.height || (isGrid ? 200 : 48);

  const cardStyle = {
    width: startWidth,
    height: startHeight,
    borderRadius: isGrid ? 8 : 0,
    backgroundColor: dark ? "#3c4043" : "var(--gd-blue-surface)", // Matches FileCard selected color
    border: isGrid ? "1px solid var(--gd-blue)" : "1px solid var(--gd-outline-variant)",
    boxShadow: "none",
  };

  const pillStyle = {
    width: Math.min(240, activeDragData.name.length * 8 + 60), // Auto-like width
    height: 38,
    borderRadius: 10, // Animation
    backgroundColor: "var(--gd-surface)",
    border: "1px solid var(--gd-outline-variant)",
    boxShadow: "0 8px 16px rgba(0,0,0,0.15), 0 2px 6px rgba(0,0,0,0.1)",
  };

  const targetStyle = isPill ? pillStyle : cardStyle;

  return (
    <div
      className={dark ? "dark" : ""}
      style={{
        ...targetStyle,
        position: "relative",
        overflow: "hidden",
        transition: "all 0.35s cubic-bezier(0.2, 0.8, 0.2, 1)",
        pointerEvents: "none",
        fontFamily: "'Google Sans', sans-serif",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          padding: "6px 16px 6px 12px",
          fontSize: 13,
          fontWeight: 500,
          color: "var(--gd-on-surface)",
          width: "max-content",
        }}
      >
        {activeDragData.isFolder ? <FolderIcon size={20} /> : <FileIcon filename={activeDragData.name} size={18} kind={undefined} />}
        <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
          {activeDragData.name}
        </span>
      </div>
    </div>
  );
}

export function MainAppContent() {


  const { t } = useTranslation();
  useWindowStateTracker();
  useEffect(() => { getCurrentWindow().emit("frontend-ready", {}).catch(() => {}); }, []);
  const uiState = useMainAppUiStateContext();
  const uiActions = useMainAppUiActions();
  const driveController = useDriveControllerContext();
  const dragDepthRef = useRef(0);
  const onboardingBootstrappedRef = useRef(false);
  const [activeDragData, setActiveDragData] = useState(null);
  const [tenantPickerState, setTenantPickerState] = useState({
    tenantList: [],
    activeTenants: { my: null, shared: null, current: null },
    loading: false,
  });

  const pointerSensor = useSensor(PointerSensor, {
    activationConstraint: { distance: 5 },
  });
  const sensors = useSensors(pointerSensor);

  const handleDndEnd = useCallback(
    (event) => {
      setActiveDragData(null);
      const { active, over } = event;
      if (!active || !over) return;

      const dragData = active.data.current;
      const dropData = over.data.current;
      if (!dragData || !dropData) return;

      const sourceScope = getItemDriveScope(dragData);
      const targetScope = dropData.targetScope ?? null;
      if (sourceScope && targetScope && sourceScope !== targetScope) {
        return;
      }

      let targetFolderId = null;
      if (dropData.type === "sidebar") {
        targetFolderId = dropData.targetFolderId;
      } else if (dropData.type === "folder") {
        targetFolderId = dropData.id;
      } else {
        return;
      }

      if (dragData.type === "folder" && dragData.id === targetFolderId) {
        return;
      }

      if (dragData.type === "file") {
        driveController.moveFile(dragData.id, targetFolderId);
      } else if (dragData.type === "folder") {
        driveController.moveFolder(dragData.id, targetFolderId);
      }
    },
    [driveController]
  );

  const handleDndStart = useCallback((event) => {
    setActiveDragData({
      ...(event.active.data.current ?? {}),
      rect: event.active.rect.current.initial, // Capture initial rect for true morphing
    });
  }, []);

  const handleDndCancel = useCallback(() => {
    setActiveDragData(null);
  }, []);

  useKeyboardActions(
    uiState.search,
    uiActions.setSearch,
    driveController.refresh,
    uiActions.setShowSettings,
    uiActions.setShowNewFolder,
    uiActions.setPreviewFile
  );

  const breadcrumbs = useBreadcrumbs(driveController.currentFolderId, driveController.folders);
  const { baseFiles, pageTitle } = usePageNavigation(
    uiState.activeSection,
    uiState.activeDriveRoot,
    driveController.files,
    driveController.trash,
    driveController.currentFolderId,
    driveController.folders
  );
  const filtered = useFileFiltering(baseFiles, uiState.search);
  const sorted = useFileSorting(filtered, uiState.sort);
  const isTransfers = uiState.activeSection === "transfers";
  const isTrash = uiState.activeSection === "trash";
  const isScopedSection = isScopedDriveSection(uiState.activeSection);
  const rootDriveSection = getRootDriveSection(uiState.activeDriveRoot);
  const rootDriveLabel =
    rootDriveSection === DRIVE_SECTION_SHARED ? t("sidebar.sharedDrive") : t("sidebar.myDrive");
  const currentTenantScope = rootDriveSection === DRIVE_SECTION_SHARED ? "shared" : "my";
  const toastContextValue = useMemo(() => ({ show: uiActions.toast.show }), [uiActions.toast.show]);
  const isFileDragEvent = useCallback((event) => {
    const types = Array.from(event.dataTransfer?.types ?? []);
    return types.includes("Files");
  }, []);

  const extractDroppedPaths = useCallback((event) => {
    const files: any = Array.from(event.dataTransfer?.files ?? []);
    return files
      .map((file: any) => (typeof file.path === "string" && file.path.length > 0 ? file.path : null))
      .filter(Boolean);
  }, []);

  const handleDragEnter = useCallback(
    (event) => {
      if (!isFileDragEvent(event)) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      dragDepthRef.current += 1;
      if (event.dataTransfer) {
        event.dataTransfer.dropEffect = "copy";
      }
      uiActions.setIsDragOver(true);
    },
    [isFileDragEvent, uiActions]
  );

  const handleDragOver = useCallback(
    (event) => {
      if (!isFileDragEvent(event)) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      if (event.dataTransfer) {
        event.dataTransfer.dropEffect = "copy";
      }
      if (!uiState.isDragOver) {
        uiActions.setIsDragOver(true);
      }
    },
    [isFileDragEvent, uiActions, uiState.isDragOver]
  );

  const handleDragLeave = useCallback(
    (event) => {
      if (!isFileDragEvent(event)) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      dragDepthRef.current = Math.max(dragDepthRef.current - 1, 0);
      if (dragDepthRef.current === 0) {
        uiActions.setIsDragOver(false);
      }
    },
    [isFileDragEvent, uiActions]
  );

  const handleDrop = useCallback(
    (event) => {
      if (!isFileDragEvent(event)) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      dragDepthRef.current = 0;
      uiActions.setIsDragOver(false);

      const droppedPaths = extractDroppedPaths(event);
      if (droppedPaths.length > 0) {
        uiActions.uploadPaths(droppedPaths);
      }
    },
    [extractDroppedPaths, isFileDragEvent, uiActions]
  );

  const listHasMore = uiState.activeSection === "trash" ? driveController.trashHasMore : driveController.filesHasMore;
  const loadMore = uiState.activeSection === "trash" ? driveController.loadMoreTrash : driveController.loadMore;
  const loadingMore =
    uiState.activeSection === "trash" ? driveController.loadingMoreTrash : driveController.loadingMore;

  const applyOnboardingState = useCallback(
    (nextState, preferredScope = null, forceVisible = true) => {
      const scope = preferredScope ?? uiState.onboardingPreferredScope ?? null
      const scopeNeedsSelection = scope ? Boolean(nextState?.tenants?.[scope]?.needsSelection) : false
      uiActions.setOnboardingState(nextState)
      if (forceVisible || nextState?.requiresOnboarding || scopeNeedsSelection) {
        uiActions.showOnboarding(scope)
        return
      }
      uiActions.hideOnboarding()
    },
    [uiActions, uiState.onboardingPreferredScope]
  )

  const refreshTenantPickerState = useCallback(async () => {
    setTenantPickerState((current) => ({ ...current, loading: true }));
    try {
      const [tenantList, activeTenants] = await Promise.all([
        DriveApi.listTenants(),
        DriveApi.getActiveTenants(),
      ]);
      const nextState = {
        tenantList: Array.isArray(tenantList) ? tenantList : [],
        activeTenants: (activeTenants as any) || { my: null, shared: null, current: null },
        loading: false,
      };
      setTenantPickerState(nextState as any);
      return nextState;
    } catch (error) {
      setTenantPickerState((current) => ({ ...current, loading: false }));
      throw error;
    }
  }, []);

  const renameTenantDisplayName = useCallback(
    async (tenant, displayName) => {
      try {
        await DriveApi.renameTenantDisplayName(tenant, displayName)
        await refreshTenantPickerState()
      } catch (error) {
        const message = toUserMessage(error)
        console.error("[MainApp] Failed to rename tenant display name:", error)
        uiActions.toast.show(
          message.message || t("tenantManager.renameFailed", "Doi ten tenant that bai."),
          "error"
        )
        throw error
      }
    },
    [refreshTenantPickerState, t, uiActions.toast]
  )

  useEffect(() => {
    if (onboardingBootstrappedRef.current) {
      return undefined;
    }
    onboardingBootstrappedRef.current = true;

    let cancelled = false
    void DriveApi.getOnboardingState()
      .then((nextState) => {
        if (cancelled) return
        uiActions.setOnboardingState(nextState)
        if ((nextState as any)?.requiresOnboarding && !uiState.onboardingDismissed) {
          uiActions.showOnboarding(null)
        }
      })
      .catch((error) => {
        if (!cancelled) {
          console.error('[MainApp] Failed to load onboarding state:', error)
        }
      })

    return () => {
      cancelled = true
    }
  }, [uiActions, uiState.onboardingDismissed])

  useEffect(() => {
    if (uiState.onboardingVisible) {
      return undefined;
    }
    let cancelled = false;
    void Promise.all([DriveApi.listTenants(), DriveApi.getActiveTenants()])
      .then(([tenantList, activeTenants]) => {
        if (cancelled) return;
        setTenantPickerState({
          tenantList: Array.isArray(tenantList) ? tenantList : [],
          activeTenants: (activeTenants as any) || { my: null, shared: null, current: null },
          loading: false,
        });
      })
      .catch((error) => {
        if (cancelled) return;
        console.error("[MainApp] Failed to load tenant picker state:", error);
        setTenantPickerState((current) => ({ ...current, loading: false }));
      });

    return () => {
      cancelled = true;
    };
  }, [uiState.activeSection, uiState.onboardingVisible]);

  const isDefaultTenant = useCallback((tenant) => {
    if (!tenant) return false;
    return (tenant.discordGuildId || "0") === "0" && (tenant.telegramGroupId || "0") === "0";
  }, []);

  const scopeTenantList = useMemo(
    () =>
      tenantPickerState.tenantList.filter(
        (tenant) => tenant.scope === currentTenantScope && !isDefaultTenant(tenant)
      ),
    [currentTenantScope, isDefaultTenant, tenantPickerState.tenantList]
  );

  const activeScopeTenant = useMemo(() => {
    const tenant = tenantPickerState.activeTenants?.[currentTenantScope] || null;
    return isDefaultTenant(tenant) ? null : tenant;
  }, [currentTenantScope, isDefaultTenant, tenantPickerState.activeTenants]);

  const openTenantSetup = useCallback(
    (scope) => {
      driveController.setCurrentFolderId(null);
      uiActions.setActiveSection(scope === "shared" ? DRIVE_SECTION_SHARED : DRIVE_SECTION_MY);
      uiActions.resetOnboardingDismissal();
      uiActions.showOnboarding(scope);
    },
    [driveController, uiActions]
  );

  const handleCreateTenantDone = useCallback(
    async (scope) => {
      await refreshTenantPickerState();
      uiActions.toast.show(
        t("onboarding.createSuccess", "Tao co so du lieu thanh cong!"),
        "success"
      );
    },
    [refreshTenantPickerState, t, uiActions.toast]
  );

  const handleTenantSwitch = useCallback(
    async (tenant) => {
      try {
        await DriveApi.switchTenant(tenant);
        driveController.setCurrentFolderId(null);
        uiActions.setActiveSection(tenant.scope === "shared" ? DRIVE_SECTION_SHARED : DRIVE_SECTION_MY);
        const nextState = await DriveApi.getOnboardingState();
        applyOnboardingState(nextState, tenant.scope, false);
        await refreshTenantPickerState();
      } catch (error) {
        const message = toUserMessage(error);
        console.error("[MainApp] Failed to switch tenant from dropdown:", error);
        uiActions.toast.show(
          message.message || t("onboarding.actionFailed", "Thao tac that bai."),
          "error"
        );
      }
    },
    [applyOnboardingState, driveController, refreshTenantPickerState, t, uiActions]
  );

  return (
    <ToastCtx.Provider value={toastContextValue}>
      <DndContext
        sensors={sensors}
        onDragStart={handleDndStart}
        onDragEnd={handleDndEnd}
        onDragCancel={handleDndCancel}
      >
      <div
        onDragEnter={handleDragEnter}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
        role="application"
        aria-label="Omega Drive"
        className={`gd-app ${uiState.dark ? "dark" : ""}`}
        style={{
          display: "flex",
          height: "100vh",
          width: "100%",
          overflow: "hidden",
          backgroundColor: "var(--gd-bg)",
          color: "var(--gd-on-surface)",
        }}
      >
        {uiState.showSidebar && <Sidebar />}

        <main style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
          <Header />

          <div
            id="main-scroll-container"
            onScroll={(event) => {
              globalThis.__GD_SCROLL_TOP = event.currentTarget.scrollTop;
            }}
            style={{ flex: 1, overflowY: "auto" }}
          >
            <div style={{ padding: "16px 24px" }}>
              {!uiState.onboardingVisible && uiState.bootstrapIssues.length > 0 && (
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    gap: 16,
                    marginBottom: 16,
                    padding: "14px 16px",
                    borderRadius: 14,
                    border: "1px solid rgba(251, 146, 60, 0.35)",
                    background: "linear-gradient(135deg, rgba(255,247,237,0.98), rgba(255,237,213,0.98))",
                    color: "#9a3412",
                  }}
                >
                  <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                    <strong style={{ fontSize: 14 }}>{t("diagnostics.bootstrapTitle")}</strong>
                    <span style={{ fontSize: 13, lineHeight: 1.4 }}>
                      {uiState.bootstrapIssues.join(", ")}.
                      {uiState.bootstrapStatus?.botEnvPath ? ` bot.env: ${uiState.bootstrapStatus.botEnvPath}` : ""}
                    </span>
                  </div>
                  <div style={{ display: "flex", alignItems: "center", gap: 8, flexShrink: 0 }}>
                    <button type="button"
                      onClick={() =>
                        openBotEnv().catch((error) => {
                          const message = toUserMessage(error);
                          console.error("Failed to open bot.env:", error);
                          uiActions.toast.show(message.message || t("diagnostics.openBotEnvFailed"), "error");
                        })
                      }
                      className="gd-btn gd-btn-secondary"
                      style={{ padding: "10px 12px" }}
                    >
                      {t("diagnostics.openBotEnv")}
                    </button>
                    <button type="button"
                      onClick={() =>
                        openDiscordAuth().catch((error) => {
                          const message = toUserMessage(error);
                          console.error("Failed to open Discord auth page:", error);
                          uiActions.toast.show(message.message || t("diagnostics.openDiscordPortalFailed"), "error");
                        })
                      }
                      className="gd-btn"
                      style={{ padding: "10px 12px" }}
                    >
                      {t("diagnostics.openDiscordPortal")}
                    </button>
                  </div>
                </div>
              )}

              {!isTransfers && (
                <>
                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "space-between",
                      marginBottom: 16,
                    }}
                  >
                    <div className="gd-breadcrumb" style={{ display: "flex", alignItems: "center", gap: 4 }}>
                      <BreadcrumbItem
                        label={driveController.currentFolderId ? rootDriveLabel : pageTitle}
                        isLast={!driveController.currentFolderId}
                        active={true}
                        onClick={
                          isScopedSection
                            ? () => {
                                driveController.setCurrentFolderId(null);
                                uiActions.setActiveSection(rootDriveSection);
                              }
                            : undefined
                        }
                      />
                      {breadcrumbs.map((breadcrumb: any, index) => (
                        <BreadcrumbItem
                          key={breadcrumb.id}
                          label={breadcrumb.name}
                          isLast={index === breadcrumbs.length - 1}
                          active={true}
                          onClick={() => driveController.setCurrentFolderId(Number(breadcrumb.id))}
                        />
                      ))}
                    </div>
                    {!isTrash && isScopedSection && (
                      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                        <BtnViewToggle view={uiState.view} setView={uiActions.setView} />
                        <BtnNewFolder onClick={uiActions.openNewFolder} />
                        <BtnUpload onClick={() => uiActions.uploadPaths()} />
                      </div>
                    )}
                  </div>

                  {!isTrash && isScopedSection && (
                    <TenantScopeDropdown
                      scope={currentTenantScope}
                      tenants={scopeTenantList}
                      activeTenant={activeScopeTenant}
                      loading={tenantPickerState.loading}
                      onSelectTenant={handleTenantSwitch}
                      onOpenManager={uiActions.showTenantManager}
                      onOpenSetup={openTenantSetup}
                    />
                  )}
                </>
              )}

              {isTransfers ? (
                <TransfersPage toast={uiActions.toast} />
              ) : (
                <FileGrid {...({ files: sorted, hasMore: listHasMore, loadMore, loadingMore } as any)} />
              )}
            </div>

            <script
              dangerouslySetInnerHTML={{
                __html: `
                  setTimeout(() => {
                    const el = document.getElementById("main-scroll-container");
                    if (el && globalThis.__GD_SCROLL_TOP) {
                      el.scrollTop = globalThis.__GD_SCROLL_TOP;
                    }
                  }, 50);
                `,
              }}
            />
          </div>
        </main>

        <ProgressOverlay {...{ progressMap: uiState.progressMap, onClose: uiActions.clearProgressMap, removeSession: uiActions.removeSession } as any} />
        {uiState.showSettings && (
          <Suspense fallback={<OverlayLoader message={t("modal.loadingSettings")} />}>
            <LazySettingsModal
              onClose={uiActions.closeSettings}
              dark={uiState.dark}
              toggleDark={uiActions.toggleDark}
              toast={uiActions.toast}
            />
          </Suspense>
        )}
        {uiState.showNewFolder && (
          <Suspense fallback={<OverlayLoader message={t("modal.loadingDialog")} />}>
            <LazyNewFolderModal
              onClose={uiActions.closeNewFolder}
              onCreate={driveController.createFolder}
            />
          </Suspense>
        )}
        {uiState.previewFile && (
          <Suspense fallback={<OverlayLoader message={t("modal.loadingPreview")} />}>
            <LazyPreviewModal
              file={uiState.previewFile}
              onClose={uiActions.closePreview}
              onDownload={() => uiActions.handleDownload(uiState.previewFile)}
              dark={uiState.dark}
            />
          </Suspense>
        )}
        {uiState.uploadPlanModal && (
          <Suspense fallback={<OverlayLoader message={t("modal.loadingUploadPlan")} />}>
            <LazyUploadPlanModal
              entries={uiState.uploadPlanModal.entries}
              toast={uiActions.toast}
              onClose={uiActions.closeUploadPlanModal}
              onProceed={uiActions.proceedUploadPlanModal}
            />
          </Suspense>
        )}
        {uiState.deleteConfirmModal && (
          <Suspense fallback={<OverlayLoader message={t("modal.loadingDialog")} />}>
            <LazyDeleteConfirmModal
              isOpen={true}
              item={uiState.deleteConfirmModal.item}
              onClose={uiActions.closeDeleteConfirmModal}
              onConfirm={uiActions.proceedDeleteConfirmModal}
            />
          </Suspense>
        )}
        {uiState.onboardingVisible && (
          <Suspense fallback={<OverlayLoader message={t("onboarding.loading", "Loading onboarding...")} />}>
            <LazyProviderOnboardingModal
              state={uiState.onboardingState}
              preferredScope={uiState.onboardingPreferredScope}
              onStateChange={applyOnboardingState}
              onSkip={uiActions.dismissOnboarding}
              toast={uiActions.toast}
            />
          </Suspense>
        )}
        {uiState.tenantManagerVisible && (
          <Suspense fallback={<OverlayLoader message={t("tenantManager.loading", "Dang tai tenant...")} />}>
            <LazyTenantManagerModal
              scope={uiState.tenantManagerScope || currentTenantScope}
              tenants={tenantPickerState.tenantList.filter(
                (tenant) =>
                  tenant.scope === (uiState.tenantManagerScope || currentTenantScope) &&
                  !isDefaultTenant(tenant)
              )}
              activeTenant={
                isDefaultTenant(tenantPickerState.activeTenants?.[uiState.tenantManagerScope || currentTenantScope])
                  ? null
                  : tenantPickerState.activeTenants?.[uiState.tenantManagerScope || currentTenantScope] || null
              }
              loading={tenantPickerState.loading}
              onClose={uiActions.hideTenantManager}
              onSwitchTenant={async (tenant) => {
                await handleTenantSwitch(tenant)
                uiActions.hideTenantManager()
              }}
              onRenameTenant={renameTenantDisplayName}
              onOpenSetup={(scope) => {
                uiActions.hideTenantManager()
                openTenantSetup(scope)
              }}
            />
          </Suspense>
        )}
        {uiState.showUrlImport && (
          <Suspense fallback={<OverlayLoader message={t("modal.loadingDialog")} />}>
            <LazyUrlImportModal
              dark={uiState.dark}
              onClose={uiActions.closeUrlImport}
              onImportStarted={uiActions.importUrl}
            />
          </Suspense>
        )}

        <ToastContainer toasts={uiActions.toast.toasts} remove={uiActions.toast.remove} />

        {uiState.isDragOver && (
          <section
            style={{
              position: "fixed",
              inset: 0,
              zIndex: 9999,
              pointerEvents: "none",
              backgroundColor: "rgba(26, 115, 232, 0.1)",
              border: "3px dashed var(--gd-blue)",
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
              gap: 20,
            }}
          >
            <div
              style={{
                backgroundColor: "var(--gd-surface)",
                padding: "24px 48px",
                borderRadius: "var(--gd-radius-lg)",
                display: "flex",
                flexDirection: "column",
                alignItems: "center",
                gap: 12,
              }}
            >
              <Upload size={48} color="var(--gd-blue)" strokeWidth={1.5} />
              <span style={{ fontSize: 18, fontWeight: 500 }}>{t("upload.dropToUpload")}</span>
            </div>
          </section>
        )}

        <DragOverlay dropAnimation={null} modifiers={[snapCenterToCursor]}>
          {activeDragData ? <MorphingDragOverlay activeDragData={activeDragData} dark={uiState.dark} /> : null}
        </DragOverlay>

        <GlobalAudioBridge />
        <MiniAudioPlayer />
      </div>
      </DndContext>
    </ToastCtx.Provider>
  );
}
