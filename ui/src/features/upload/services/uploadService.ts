import { ask, open } from "@tauri-apps/plugin-dialog";
import { DriveApi } from "../../../api/index";
import { call } from "../../../shared/api/call";
import type { DriveScope, JsonRecord, UploadSelection } from "../../../shared/api/types";
import { genSid } from "../../../shared/utils/index";

type UploadEntry = {
  path: string;
  filename: string;
  collidingFile?: {
    id: number;
    filename: string;
    folder_id?: number | null;
  } | null;
};

type UploadGroup = {
  entries: UploadEntry[];
  profileId: string | null;
  uploadPlan: JsonRecord | null;
};

type UploadCandidate = {
  filename?: string;
  folder_id?: number | null;
  id?: number;
};

type CollidingUploadEntry = UploadEntry & {
  collidingFile: NonNullable<UploadEntry["collidingFile"]>;
};

type UploadSummary = {
  started: number;
  skipped: number;
  failedToStart: string[];
};

type ToastLike = {
  show: (message: string, level: "success" | "info" | "error") => void;
};

const getFilename = (p: string): string => p.split(/[\\/]/).pop() ?? p;

const toCollidingFile = (
  candidate?: UploadCandidate,
): NonNullable<UploadEntry["collidingFile"]> | undefined => {
  if (!candidate || typeof candidate.id !== "number" || typeof candidate.filename !== "string") {
    return undefined;
  }

  return {
    id: candidate.id,
    filename: candidate.filename,
    folder_id: candidate.folder_id ?? null,
  };
};

const selectionKey = (selection: UploadSelection = {}): string =>
  JSON.stringify({
    profileId: selection.profileId ?? null,
    uploadPlan: selection.uploadPlan ?? null,
  });

function buildUploadGroups(
  entries: UploadEntry[],
  planByPath: Map<string, UploadSelection> = new Map<string, UploadSelection>(),
): UploadGroup[] {
  const groups = new Map<string, UploadGroup>();

  for (const entry of entries) {
    const selection = planByPath.get(entry.path) ?? {};
    const key = selectionKey(selection);
    const current = groups.get(key) ?? {
      entries: [],
      profileId: selection.profileId ?? null,
      uploadPlan: selection.uploadPlan ?? null,
    };
    current.entries.push(entry);
    groups.set(key, current);
  }

  return Array.from(groups.values());
}
// Helper to filter entries before upload
function filterPendingEntries(
  entries: UploadEntry[],
  overwriteConfirmed: boolean,
  blockedByPurge: Set<string>,
): { pending: UploadEntry[]; skipped: number; failedToStart: string[] } {
  const pending: UploadEntry[] = [];
  let skipped = 0;
  const failedToStart: string[] = [];
  for (const entry of entries) {
    if (entry.collidingFile && !overwriteConfirmed) {
      skipped++;
      continue;
    }
    if (blockedByPurge.has(entry.filename)) {
      failedToStart.push(entry.filename);
      continue;
    }
    pending.push(entry);
  }
  return { pending, skipped, failedToStart };
}

// Helper to process upload groups
async function processUploadGroups(
  pendingEntries: UploadEntry[],
  fid: number | null,
  driveScope: DriveScope | null,
  planByPath: Map<string, UploadSelection>,
): Promise<{ started: number; failedToStart: string[] }> {
  let started = 0;
  const failedToStart: string[] = [];
  for (const group of buildUploadGroups(pendingEntries, planByPath)) {
    if (group.entries.length > 1) {
      try {
        await DriveApi.uploadFilesFromPaths(
          group.entries.map((e) => e.path),
          fid,
          driveScope,
          genSid("upb"),
          group.profileId,
          group.uploadPlan,
        );
        started += group.entries.length;
        continue;
      } catch {
        // fallback to individual uploads
      }
    }
    for (const entry of group.entries) {
      try {
        await DriveApi.uploadFile(
          entry.path,
          fid,
          driveScope,
          genSid("up"),
          group.profileId,
          group.uploadPlan,
        );
        started++;
      } catch {
        failedToStart.push(entry.filename);
      }
    }
  }
  return { started, failedToStart };
}


export function buildUploadEntries(
  paths: string[],
  files: UploadCandidate[],
  trash: UploadCandidate[],
  currentFolderId: number | null,
): UploadEntry[] {
  return paths.map((path) => {
    const filename = getFilename(path);
    const existing = files.find((f) => f.filename === filename);
    const inTrash = trash.find(
      (f) =>
        f.filename === filename &&
        (Number(f.folder_id) === Number(currentFolderId) || (!f.folder_id && !currentFolderId)),
    );
    const collidingFile = toCollidingFile(existing ?? inTrash);
    return collidingFile ? { path, filename, collidingFile } : { path, filename };
  });
}

export async function selectFiles(): Promise<string[] | null> {
  const selected = await open({
    multiple: true,
    directory: false,
    title: "Select files to upload",
  });
  if (!selected) return null;
  return Array.isArray(selected) ? selected : [selected];
}

export async function confirmOverwrite(collisions: UploadEntry[]): Promise<boolean> {
  const sample = collisions.slice(0, 5).map((c) => `"${c.filename}"`).join(", ");
  const suffix = collisions.length > 5 ? ", ..." : "";
  return ask(
    `Detected ${collisions.length} duplicate files: ${sample}${suffix}. Overwrite all duplicates?`,
    { title: "Overwrite duplicate files", kind: "warning" },
  );
}

export async function processPurge(collisions: CollidingUploadEntry[]): Promise<Set<string>> {
  const blockedByPurge = new Set<string>();
  const unique = Array.from(
    new Map<number, CollidingUploadEntry>(collisions.map((c) => [c.collidingFile.id, c])).values(),
  );
  const results = await Promise.allSettled(
    unique.map((entry) => DriveApi.purgeFile(entry.collidingFile.id)),
  );
  results.forEach((r, i) => {
    if (r.status === 'rejected') blockedByPurge.add(unique[i].filename);
  });
  return blockedByPurge;
}

export async function executeUploadLoop(
  entries: UploadEntry[],
  overwriteConfirmed: boolean,
  blockedByPurge: Set<string>,
  folderId: number | null,
  driveScope: DriveScope | null,
  planByPath: Map<string, UploadSelection> = new Map<string, UploadSelection>(),
): Promise<UploadSummary> {
  const fid = folderId ? Number(folderId) : null;

  const { pending, skipped, failedToStart: initialFailed } = filterPendingEntries(
    entries,
    overwriteConfirmed,
    blockedByPurge,
  );

  const { started, failedToStart: groupFailed } = await processUploadGroups(
    pending,
    fid,
    driveScope,
    planByPath,
  );

  const failedToStart = [...initialFailed, ...groupFailed];

  return { started, skipped, failedToStart };
}

export function showUploadSummary(
  toast: ToastLike,
  { started, skipped, failedToStart }: UploadSummary,
): void {
  if (started > 0) toast.show(`Started uploading ${started} file(s).`, "success");
  if (skipped > 0) toast.show(`Skipped ${skipped} duplicate file(s).`, "info");
  if (failedToStart.length > 0) {
    const sample = failedToStart.slice(0, 3).map((name) => `"${name}"`).join(", ");
    const suffix = failedToStart.length > 3 ? "..." : "";
    toast.show(`Could not upload ${failedToStart.length} file(s): ${sample}${suffix}`, "error");
  }
}

export async function resumeUploadByPath(fileObj: {
  local_path?: string | null;
  folder_id?: number | null;
  drive_scope?: DriveScope | null;
}): Promise<unknown> {
  if (!fileObj?.local_path) {
    throw new Error("Missing local_path");
  }
  return DriveApi.uploadFile(
    fileObj.local_path,
    fileObj.folder_id ?? null,
    fileObj.drive_scope || "my",
    genSid("resume"),
    null,
    null,
  );
}

export async function resumeUploadTask(
  sessionId: string,
  fileId: number,
  filePath: string,
  folderId: number | null,
  driveScope: DriveScope | null = null,
): Promise<unknown> {
  return call(
    "resume_upload",
    { sessionId, fileId, filePath, folderId, driveScope },
    { feature: "upload", action: "resume_upload" },
  );
}

export async function cancelTransfer(sessionId: string): Promise<unknown> {
  return call("cancel_transfer", { sessionId }, { feature: "upload", action: "cancel_transfer" });
}
