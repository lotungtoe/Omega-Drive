import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { DRIVE_SECTION_MY, isScopedDriveSection } from "./driveSections";

function readStorage(key, fallback, parser = (value) => value) {
  try {
    const value = localStorage.getItem(key);
    if (value == null) {
      return fallback;
    }
    return parser(value);
  } catch {
    return fallback;
  }
}

function writeStorage(key, value, label) {
  try {
    localStorage.setItem(key, value);
  } catch (error) {
    console.warn(`Failed to save ${label}:`, error);
  }
}

export function useMainAppUiState() {
  const [dark, setDark] = useState(() => readStorage("gd-dark-mode", false, (value) => value === "true"));
  const [view, setView] = useState(() => readStorage("gd-view-mode", "grid"));
  const [showSidebar, setShowSidebar] = useState(true);
  const [search, setSearch] = useState("");
  const [previewFile, setPreviewFile] = useState(null);
  const [activeSection, setActiveSectionState] = useState("home");
  const [activeDriveRoot, setActiveDriveRootState] = useState(DRIVE_SECTION_MY);
  const [showSettings, setShowSettings] = useState(false);
  const [showNewFolder, setShowNewFolder] = useState(false);
  const [isDragOver, setIsDragOver] = useState(false);
  const [sort, setSort] = useState({ field: "name", dir: "asc" });
  const [progressMap, setProgressMap] = useState({});
  const [isAnyVideoPlaying, setIsAnyVideoPlaying] = useState(false);
  const [bootstrapStatus, setBootstrapStatus] = useState(null);
  const [onboardingState, setOnboardingState] = useState(null);
  const [onboardingVisible, setOnboardingVisible] = useState(false);
  const [onboardingDismissed, setOnboardingDismissed] = useState(false);
  const [onboardingPreferredScope, setOnboardingPreferredScope] = useState(null);
  const [tenantManagerScope, setTenantManagerScope] = useState(null);
  const [tenantManagerVisible, setTenantManagerVisible] = useState(false);
  const [uploadPlanModal, setUploadPlanModal] = useState(null);
  const [deleteConfirmModal, setDeleteConfirmModal] = useState(null);
  const [showUrlImport, setShowUrlImport] = useState(false);

  // Audio Player State
  const [activeAudioFile, setActiveAudioFile] = useState(null);
  const [isAudioMinimized, setIsAudioMinimized] = useState(false);
  const [audioPlayback, setAudioPlayback] = useState({
    playing: false,
    position: 0,
    duration: 0,
    loading: false,
  });

  useEffect(() => {
    document.body.classList.toggle("dark", dark);
    writeStorage("gd-dark-mode", String(dark), "dark mode");
  }, [dark]);

  useEffect(() => {
    writeStorage("gd-view-mode", view, "view mode");
  }, [view]);

  const openUploadPlanModal = useCallback(
    (entries) =>
      new Promise((resolve) => {
        setUploadPlanModal({ entries, resolve });
      }),
    []
  );

  const closeUploadPlanModal = useCallback(() => {
    setUploadPlanModal((current) => {
      if (current?.resolve) {
        current.resolve(null);
      }
      return null;
    });
  }, []);

  const proceedUploadPlanModal = useCallback((result) => {
    setUploadPlanModal((current) => {
      if (current?.resolve) {
        current.resolve(result);
      }
      return null;
    });
  }, []);

  const openDeleteConfirmModal = useCallback(
    (item) =>
      new Promise((resolve) => {
        setDeleteConfirmModal({ item, resolve });
      }),
    []
  );

  const closeDeleteConfirmModal = useCallback(() => {
    setDeleteConfirmModal((current) => {
      if (current?.resolve) {
        current.resolve(false);
      }
      return null;
    });
  }, []);

  const proceedDeleteConfirmModal = useCallback(() => {
    setDeleteConfirmModal((current) => {
      if (current?.resolve) {
        current.resolve(true);
      }
      return null;
    });
  }, []);

  const setActiveSection = useCallback((section) => {
    setActiveSectionState(section);
    if (isScopedDriveSection(section)) {
      setActiveDriveRootState(section);
    }
  }, []);

  const setActiveDriveRoot = useCallback((section) => {
    if (isScopedDriveSection(section)) {
      setActiveDriveRootState(section);
    }
  }, []);

  const uiState = useMemo(
    () => ({
      dark,
      view,
      showSidebar,
      search,
      previewFile,
      activeSection,
      activeDriveRoot,
      showSettings,
      showNewFolder,
      isDragOver,
      sort,
      progressMap,
      isAnyVideoPlaying,
      bootstrapStatus,
      onboardingState,
      onboardingVisible,
      onboardingDismissed,
      onboardingPreferredScope,
      tenantManagerScope,
      tenantManagerVisible,
      uploadPlanModal,
      deleteConfirmModal,
      showUrlImport,
      activeAudioFile,
      isAudioMinimized,
      audioPlayback,
    }),
    [
      activeSection,
      activeDriveRoot,
      bootstrapStatus,
      onboardingDismissed,
      onboardingPreferredScope,
      tenantManagerScope,
      tenantManagerVisible,
      onboardingState,
      onboardingVisible,
      dark,
      isAnyVideoPlaying,
      isDragOver,
      previewFile,
      progressMap,
      search,
      showNewFolder,
      showSettings,
      showSidebar,
      sort,
      uploadPlanModal,
      deleteConfirmModal,
      showUrlImport,
      view,
      activeAudioFile,
      isAudioMinimized,
      audioPlayback,
    ]
  );

  const uiActions = useMemo(
    () => ({
      setDark,
      toggleDark: () => setDark((value) => !value),
      setView,
      toggleSidebar: () => setShowSidebar((value) => !value),
      setShowSidebar,
      setSearch,
      openPreview: (file) => {
        setPreviewFile(file);
        // Sync with background audio player if it's an audio file
        const isAudio = file?.kind?.startsWith('audio') || file?.filename?.match(/\.(mp3|wav|ogg|flac|m4a)$/i);
        if (isAudio) {
          setActiveAudioFile(file);
          setAudioPlayback(p => ({ ...p, playing: true }));
          setIsAudioMinimized(false);
        } else if (activeAudioFile) {
          setIsAudioMinimized(true);
        }
      },
      closePreview: () => {
        setPreviewFile(null);
        if (activeAudioFile) {
          setIsAudioMinimized(true);
        }
      },
      setPreviewFile,
      setActiveSection,
      setActiveDriveRoot,
      openSettings: () => setShowSettings(true),
      closeSettings: () => setShowSettings(false),
      setShowSettings,
      openNewFolder: () => setShowNewFolder(true),
      closeNewFolder: () => setShowNewFolder(false),
      setShowNewFolder,
      openUrlImport: () => setShowUrlImport(true),
      closeUrlImport: () => setShowUrlImport(false),
      setIsDragOver,
      setSort,
      setProgressMap,
      clearProgressMap: () => setProgressMap({}),
      setIsAnyVideoPlaying,
      setBootstrapStatus,
      setOnboardingState,
      showOnboarding: (preferredScope = null) => {
        setOnboardingPreferredScope(preferredScope);
        setOnboardingVisible(true);
      },
      hideOnboarding: () => setOnboardingVisible(false),
      dismissOnboarding: () => {
        setOnboardingDismissed(true);
        setOnboardingVisible(false);
      },
      resetOnboardingDismissal: () => setOnboardingDismissed(false),
      setOnboardingPreferredScope,
      showTenantManager: (scope) => {
        setTenantManagerScope(scope);
        setTenantManagerVisible(true);
      },
      hideTenantManager: () => setTenantManagerVisible(false),
      setTenantManagerScope,
      openUploadPlanModal,
      closeUploadPlanModal,
      proceedUploadPlanModal,
      openDeleteConfirmModal,
      closeDeleteConfirmModal,
      proceedDeleteConfirmModal,
      setActiveAudioFile,
      setIsAudioMinimized,
      setAudioPlayback,
      minimizeAudio: () => {
        setIsAudioMinimized(true);
        setPreviewFile(null);
      },
      closeAudio: async () => {
        setActiveAudioFile(null);
        setIsAudioMinimized(false);
        setAudioPlayback({ playing: false, position: 0, duration: 0, loading: false });
        try {
          const port = await invoke('get_bridge_port');
          await fetch(`http://127.0.0.1:${port}/player/shutdown?type=audio`, { method: 'POST' });
        } catch (e) {
          console.error("Failed to shutdown audio player:", e);
        }
      },
      expandAudio: () => {
        setIsAudioMinimized(false);
        if (activeAudioFile) {
          setPreviewFile(activeAudioFile);
        }
      },
    }),
    [
      closeUploadPlanModal,
      openUploadPlanModal,
      proceedUploadPlanModal,
      openDeleteConfirmModal,
      closeDeleteConfirmModal,
      proceedDeleteConfirmModal,
      activeAudioFile,
      setActiveSection,
      setActiveDriveRoot,
    ]
  );

  return { uiState, uiActions };
}
