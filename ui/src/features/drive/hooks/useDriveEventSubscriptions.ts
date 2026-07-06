import { useCallback, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { openDownloadFile, openDownloadFolder } from "../../download/services/downloadService";
import { DriveApi } from "../../../api";

export function useDriveEventSubscriptions({
  isInternalDragging,
  setIsDragOver,
  setIsAnyVideoPlaying,
  setProgressMap,
  refreshInBackground,
  toast,
  t,
  uploadPathsRef,
}) {
  const progressQueueRef = useRef({});
  const cancelledRef = useRef(new Set());
  const dbRefreshTimeoutRef = useRef(null);

  useEffect(() => {
    const handleProgress = (event) => {
      const payload = event.payload;
      const sessionId = payload.sessionId;
      if (!sessionId) {
        return;
      }
      if (cancelledRef.current.has(sessionId)) {
        return;
      }
      if (payload.phase === "failed") {
        const detail =
          payload.detail ||
          t("upload.failed", { defaultValue: "Upload failed. Vui long kiem tra cau hinh provider." });
        toast?.show?.(detail, "error", 8000);
      }

      let percentage = 0;
      if (payload.totalParts > 0) {
        percentage = Math.round((payload.doneParts / payload.totalParts) * 100);
      } else if (payload.platforms?.length > 0) {
        const totalDone = payload.platforms.reduce((sum, p) => sum + p.done, 0)
        const totalAll = payload.platforms.reduce((sum, p) => sum + p.total, 0)
        if (totalAll > 0) {
          percentage = Math.round((totalDone / totalAll) * 100);
        }
      }
      if (payload.phase === "done") {
        percentage = 100;
        if (refreshInBackground) {
          refreshInBackground();
        }
      }

      progressQueueRef.current[sessionId] = {
        ...payload,
        percentage,
        lastUpdate: Date.now(),
      };
    };

    const unlistenUpload = listen("upload-progress", handleProgress);
    const unlistenDownload = listen("download-progress", handleProgress);
    const unlistenPlayback = listen("playback-state-changed", (event) => {
      setIsAnyVideoPlaying(Boolean(event.payload));
    });

    const batchInterval = setInterval(() => {
      const queue = progressQueueRef.current;
      
      setProgressMap((previous) => {
        let changed = Object.keys(queue).length > 0;
        let next = changed ? { ...previous, ...queue } : previous;
        
        const now = Date.now();
        for (const sessionId of Object.keys(next)) {
          const entry = next[sessionId];
          const staleSecs = now - entry.lastUpdate;
          // Xóa ngay nếu đã hoàn thành và cũ hơn 2 giây
          const isDone = entry.percentage >= 100 && staleSecs > 2000;
          // Xóa nếu phase là failed (không có % = 100) và cũ hơn 3 giây
          const isFailed = entry.phase === 'failed' && staleSecs > 3000;
          // Xóa nếu session bị stuck (không cập nhật hơn 8 giây) - trường hợp bị cancel mà không có event
          const isStuck = staleSecs > 8000;
          if (isDone || isFailed || isStuck) {
            if (!changed) {
              next = { ...previous };
              changed = true;
            }
            delete next[sessionId];
          }
        }
        return changed ? next : previous;
      });
      progressQueueRef.current = {};
    }, 1000);

    return () => {
      unlistenUpload.then((callback) => callback());
      unlistenDownload.then((callback) => callback());
      unlistenPlayback.then((callback) => callback());
      clearInterval(batchInterval);
    };
  }, [setIsAnyVideoPlaying, setProgressMap, refreshInBackground, t, toast]);

  useEffect(() => {
    let unlistenComplete;
    let unlistenFailed;

    const setup = async () => {
      try {
        [unlistenComplete, unlistenFailed] = await Promise.all([
          listen("download-complete", (event) => {
            const payload = event.payload || {};
            const filename = payload.filename || t("downloads.unnamedFile");
            const path = payload.path;
            const fileId = payload.fileId;

            if (fileId) {
              setProgressMap((previous) => {
                const next = { ...previous };
                delete next[`dl-${fileId}`];
                return next;
              });
            }

            toast.show(t("downloads.completed", { filename }), "success", 8000, [
              path && { label: t("downloads.openFile"), onClick: () => openDownloadFile(path).catch(() => {}) },
              path && { label: t("downloads.openFolder"), onClick: () => openDownloadFolder(path).catch(() => {}) },
            ].filter(Boolean));
          }),
          listen("download-failed", (event) => {
            const payload = event.payload || {};
            const errorMessage = payload.error || t("downloads.failed");
            const fileId = payload.fileId;

            if (fileId) {
              setProgressMap((previous) => {
                const next = { ...previous };
                delete next[`dl-${fileId}`];
                return next;
              });
            }

            toast.show(errorMessage, "error", 8000);
          }),
        ]);
      } catch (error) {
        console.error("Failed to setup download notifications:", error);
      }
    };

    setup();

    return () => {
      if (unlistenComplete) {
        unlistenComplete();
      }
      if (unlistenFailed) {
        unlistenFailed();
      }
    };
  }, [setProgressMap, t, toast]);

  useEffect(() => {
    let lastTime = performance.now();
    const interval = setInterval(() => {
      const now = performance.now();
      const delta = now - lastTime;
      if (delta > 2000) {
        console.warn(`[Forensic] Major UI Freeze detected! Lag: ${delta - 1000}ms`);
      }
      lastTime = now;
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    let unlisten;

    const setupDropListener = async () => {
      try {
        const appWebview = getCurrentWebview();
        unlisten = await appWebview.onDragDropEvent((event) => {
          if (isInternalDragging) {
            return;
          }

          if (event.payload.type === "enter" || event.payload.type === "over") {
            setIsDragOver(true);
          } else if (event.payload.type === "leave") {
            setIsDragOver(false);
          } else if (event.payload.type === "drop") {
            setIsDragOver(false);
            if (event.payload.paths?.length > 0) {
              uploadPathsRef.current(event.payload.paths);
            }
          }
        });
      } catch (error) {
        console.error("Failed to setup drop listener:", error);
      }
    };

    setupDropListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [isInternalDragging, setIsDragOver, uploadPathsRef]);

  useEffect(() => {
    let unlistenFn;

    const setupDatabaseListener = async () => {
      try {
        unlistenFn = await listen("omega-event", (event) => {
          const payload = event.payload;
          // Print JSON string to know exact structure
          console.info("[AutoRefresh] omega-event payload:", payload);
          
          // Check both string and Object cases with type field
          const isFilesChanged = 
            payload === "FilesTableChanged" || 
            payload?.type === "FilesTableChanged";
          
          if (isFilesChanged) {
            console.info("[AutoRefresh] FilesTableChanged detected.");
            if (dbRefreshTimeoutRef.current) {
              clearTimeout(dbRefreshTimeoutRef.current);
            }
            
            dbRefreshTimeoutRef.current = setTimeout(() => {
              if (typeof refreshInBackground === "function") {
                console.info("[AutoRefresh] Calling refreshInBackground().");
                refreshInBackground();
              } else {
                console.warn("â ï¸ [AutoRefresh] refreshInBackground is not a function!", refreshInBackground);
              }
              dbRefreshTimeoutRef.current = null;
            }, 500);
          }
        });
        console.info("[AutoRefresh] Database listener is active.");
      } catch (error) {
        console.error("Failed to setup database auto-update listener:", error);
      }
    };

    setupDatabaseListener();

    return () => {
      if (unlistenFn) {
        unlistenFn();
      }
    };
  }, [refreshInBackground]);

  const removeSession = useCallback((sessionId, sessionData) => {
    cancelledRef.current.add(sessionId);
    delete progressQueueRef.current[sessionId];
    setProgressMap((prev) => {
      const next = { ...prev };
      delete next[sessionId];
      return next;
    });
    if (sessionData?.fileId != null && !sessionId.startsWith('dl-')) {
      DriveApi.purgeFile(sessionData.fileId).catch(() => {});
    }
    setTimeout(() => {
      cancelledRef.current.delete(sessionId);
    }, 15_000);
  }, [setProgressMap]);

  return { removeSession };
}
