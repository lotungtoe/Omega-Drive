import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { onOpenUrl } from "@tauri-apps/plugin-deep-link";

export function useDeepLink(handlePlay, driveFiles) {
  useEffect(() => {
    let unlisten = null;
    let isMounted = true;

    const processUrl = (url) => {
      console.info("Deep link received:", url);
      if (url.startsWith("omegadrive://play/")) {
        const idStr = url.replace("omegadrive://play/", "");
        const fileId = Number.parseInt(idStr, 10);

        if (!Number.isNaN(fileId)) {
          const existingFile = driveFiles?.find?.((f) => f.id === fileId);

          if (existingFile) {
            handlePlay(existingFile);
          } else {
            handlePlay({ id: fileId, status: "ready" });
          }
        }
      }
    };

    const initDeepLink = async () => {
      try {
        unlisten = await onOpenUrl((urls) => {
          if (!isMounted) return;
          urls.forEach(processUrl);
        });

        // Lắng nghe thêm event thủ công từ single-instance để đảm bảo không miss
        const unlistenManual = await listen("omegadrive-deep-link", (event) => {
          if (!isMounted) return;
          if (event.payload) {
            processUrl(event.payload);
          }
        });

        const oldUnlisten = unlisten;
        unlisten = async () => {
          if (typeof oldUnlisten === "function") oldUnlisten();
          else if (oldUnlisten?.then) await oldUnlisten;
          unlistenManual();
        };

      } catch (err) {
        console.warn("Could not setup deep link:", err);
      }
    };
    
    initDeepLink();
    
    return () => {
      isMounted = false;
      if (typeof unlisten === "function") unlisten();
      else if (unlisten?.then) {
         unlisten.then(fn => typeof fn === "function" && fn());
      }
    };
  }, [handlePlay, driveFiles]);
}
