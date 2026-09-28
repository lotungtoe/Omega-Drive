// Pure helpers for live upload progress. No React, no Tauri — vitest covers this file.

export type UploadPlatformProgress = {
  name: string;
  done: number;
  total: number;
};

export type UploadProgressPayload = {
  sessionId: string;
  fileName: string;
  phase: string;
  doneParts: number;
  totalParts: number;
  detail: string;
  overallProgress: number;
  platforms: UploadPlatformProgress[];
  fileId?: number | null;
};

export type UploadLiveStat = {
  percent: number;
  detail: string;
  phase: string;
  speedBps: number;
  etaSecs: number | null;
  bytesDone: number;
  bytesTotal: number;
};

const basename = (p?: string | null): string => {
  if (!p) return "";
  const parts = String(p).split(/[/\\]/);
  return parts[parts.length - 1] || String(p);
};

/** Stable map key for a progress event: file id when assigned, else file name. */
export function uploadStatKey(p: Pick<UploadProgressPayload, "fileId" | "fileName">): string | null {
  if (p.fileId != null && Number.isFinite(p.fileId)) return `id:${p.fileId}`;
  const name = basename(p.fileName);
  return name ? `name:${name}` : null;
}

/** Total bytes across provider platforms. */
export function uploadBytes(p: Pick<UploadProgressPayload, "platforms">): { done: number; total: number } {
  return (p.platforms ?? []).reduce(
    (acc, pl) => ({ done: acc.done + (Number(pl.done) || 0), total: acc.total + (Number(pl.total) || 0) }),
    { done: 0, total: 0 }
  );
}

/** Pick the live stat for a DB file row: id match first, name match as fallback. */
export function resolveUploadStat(
  file: { id: number; filename: string },
  live: Record<string, UploadLiveStat>
): UploadLiveStat | null {
  return live[`id:${file.id}`] ?? live[`name:${basename(file.filename)}`] ?? null;
}

export type SpeedSample = { bytes: number; t: number };

/** Bytes-per-second from two samples. Non-positive deltas (skew/retransmit) read as 0. */
export function nextSpeed(
  prev: SpeedSample | undefined,
  bytes: number,
  now: number
): { speed: number; sample: SpeedSample } {
  let speed = 0;
  if (prev && now > prev.t) {
    const v = (bytes - prev.bytes) / ((now - prev.t) / 1000);
    if (v > 0) speed = v;
  }
  return { speed, sample: { bytes, t: now } };
}
