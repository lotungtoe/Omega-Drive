import { getCurrentWindow } from "@tauri-apps/api/window";

export function readWindowMode() {
  const params = new URLSearchParams(globalThis.location.search);
  const queryFileId = params.get("file_id");
  const queryTitle = params.get("title") || "";

  try {
    const label = getCurrentWindow().label;
    if (label.startsWith("video-player-")) {
      return {
        isVideoWindow: true,
        videoWindowFileId: queryFileId || label.replace("video-player-", ""),
        videoTitle: queryTitle,
      };
    }
  } catch {
    // Fall back to URL-based detection.
  }

  const res = {
    isVideoWindow: !!queryFileId,
    videoWindowFileId: queryFileId,
    videoTitle: queryTitle,
  };
  console.info("[Diagnostic] readWindowMode result:", res);
  return res;
}
