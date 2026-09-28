import type { UploadProgressPayload } from "./uploadProgress";

// Browser-only fake progress so the Transfers UI can be previewed with
// `npm run dev` (no Tauri events there). Never imported on desktop paths
// except through the isTauriRuntime() gate in useTransfersList.

/** Advance toward 95 and hold — real completion still comes from the backend. */
export function nextMockProgress(prevPercent: number, step = 7): number {
  if (prevPercent >= 95) return 95;
  return Math.min(prevPercent + step, 95);
}

export function mockPayload(
  file: { id?: number; filename?: string; size?: number },
  percent: number
): UploadProgressPayload {
  const total = file.size && file.size > 0 ? file.size : 1000;
  const done = Math.round((percent / 100) * total);
  return {
    sessionId: `mock-${file.id ?? "unknown"}`,
    fileName: file.filename ?? "unknown",
    phase: "uploading",
    doneParts: 0,
    totalParts: 0,
    detail: "",
    overallProgress: percent,
    platforms: [{ name: "Mock", done, total }],
    fileId: file.id ?? null,
  };
}
