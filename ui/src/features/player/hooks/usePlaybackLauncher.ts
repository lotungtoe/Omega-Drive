import { useCallback, useMemo } from "react";
import { toUserMessage } from "../../../shared/services/errors/toUserMessage";
import {
  clearPlaybackPosition,
  getPlaybackPosition,
  loadBootstrapStatus,
  openVideoWindow,
  parseBootstrapIssues,
} from "../services/playerService";
import { formatPlaybackTime } from "../../../shared/utils/formatPlaybackTime";
import { getFileType } from "../../../shared/utils/index";

export function usePlaybackLauncher({ bootstrapStatus, setBootstrapStatus, toast, t, openPreview }) {
  const bootstrapIssues = useMemo(() => parseBootstrapIssues(bootstrapStatus), [bootstrapStatus]);

  const ensureBootstrapStatus = useCallback(async () => {
    if (bootstrapStatus != null) {
      return bootstrapStatus;
    }

    try {
      const status = await loadBootstrapStatus();
      setBootstrapStatus(status);
      return status;
    } catch (error) {
      console.error("Failed to load bootstrap status:", error);
      return null;
    }
  }, [bootstrapStatus, setBootstrapStatus]);

  const launchNativePlayer = useCallback(
    async (file) => {
      if (file.status !== "ready") {
        toast.show(t("player.videoNotReady"), "info");
        return;
      }

      await ensureBootstrapStatus();

      const playerTitle = file.filename || file.name || `Video #${file.id}`;
      let startPositionSec;

      try {
        const playback = await getPlaybackPosition(file.id);
        if (playback?.resumeEligible) {
          const shouldResume = await (async () => {
            try {
              if (typeof globalThis.confirm !== "function") return true;
              return await globalThis.confirm(
                t("player.resumePrompt", {
                  title: playerTitle,
                  time: formatPlaybackTime(playback.positionSec),
                })
              );
            } catch {
              return true;
            }
          })();

          if (shouldResume) {
            startPositionSec = playback.positionSec;
          } else {
            clearPlaybackPosition(file.id).catch(() => {});
          }
        }
      } catch (error) {
        console.warn("Khong the tai thong tin resume playback:", error);
      }

      openVideoWindow(file.id, playerTitle, startPositionSec).catch((error) => {
        const message = toUserMessage(error);
        console.error("Loi mo trinh phat native:", error);
        toast.show(message.message || t("player.openNativeFailed"), "error");
      });
    },
    [ensureBootstrapStatus, t, toast]
  );

  const handlePlay = useCallback(
    async (file) => {
      await launchNativePlayer(file);
    },
    [launchNativePlayer]
  );

  const handlePreview = useCallback(
    async (file) => {
      const displayName = file?.filename || file?.name || "";
      if (getFileType(displayName, file?.kind).group === "video") {
        await launchNativePlayer(file);
        return;
      }

      openPreview(file);
    },
    [launchNativePlayer, openPreview]
  );

  return { bootstrapIssues, handlePlay, handlePreview };
}
