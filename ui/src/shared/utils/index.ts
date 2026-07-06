type ClassNameValue = string | false | null | undefined;
type FileGroup = "doc" | "image" | "video" | "audio" | "archive" | "code" | "sheet" | "other";
type FileKind = "image" | "video" | "audio" | "document" | "archive" | "code" | "sheet" | "unknown" | "other";

type FileTypeInfo = {
  labelKey: string;
  group: FileGroup;
  ext: string;
};

export const cn = (...args: ClassNameValue[]): string => args.filter(Boolean).join(" ");

export const formatSize = (bytes: number | null | undefined): string => {
  if (!bytes || bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const unitIndex = Math.floor(Math.log(bytes) / Math.log(1024));
  const safeIndex = Math.min(unitIndex, units.length - 1);
  return `${(bytes / Math.pow(1024, safeIndex)).toFixed(1)} ${units[safeIndex]}`;
};

export const formatDateSafe = (
  value: string | number | Date | null | undefined,
  locale = "vi-VN",
  options?: Intl.DateTimeFormatOptions,
): string => {
  if (!value) return "-";
  const date = value instanceof Date ? value : new Date(value);
  if (Number.isNaN(date.getTime())) return "-";
  const hasTime =
    options &&
    (Object.hasOwn(options, "hour") ||
      Object.hasOwn(options, "minute") ||
      Object.hasOwn(options, "second"));
  if (hasTime) return date.toLocaleString(locale, options);
  return date.toLocaleDateString(locale, options);
};

export const getExt = (filename?: string | null): string => filename?.split(".").pop()?.toLowerCase() || "";

const KIND_MAP: Record<FileKind, { labelKey: string; group: FileGroup }> = {
  image: { labelKey: "fileType.image", group: "image" },
  video: { labelKey: "fileType.video", group: "video" },
  audio: { labelKey: "fileType.audio", group: "audio" },
  document: { labelKey: "fileType.document", group: "doc" },
  archive: { labelKey: "fileType.archive", group: "archive" },
  code: { labelKey: "fileType.code", group: "code" },
  sheet: { labelKey: "fileType.sheet", group: "sheet" },
  unknown: { labelKey: "fileType.unknown", group: "other" },
  other: { labelKey: "fileType.unknown", group: "other" },
};

const FILE_TYPE_MAP: Record<string, { labelKey: string; group: FileGroup }> = {
  pdf: { labelKey: "fileType.pdf", group: "doc" },
  doc: { labelKey: "fileType.word", group: "doc" },
  docx: { labelKey: "fileType.word", group: "doc" },
  txt: { labelKey: "fileType.text", group: "doc" },
  jpg: { labelKey: "fileType.image", group: "image" },
  jpeg: { labelKey: "fileType.image", group: "image" },
  png: { labelKey: "fileType.image", group: "image" },
  gif: { labelKey: "fileType.gif", group: "image" },
  webp: { labelKey: "fileType.image", group: "image" },
  svg: { labelKey: "fileType.svg", group: "image" },
  avif: { labelKey: "fileType.image", group: "image" },
  ico: { labelKey: "fileType.image", group: "image" },
  heic: { labelKey: "fileType.image", group: "image" },
  heif: { labelKey: "fileType.image", group: "image" },
  mp4: { labelKey: "fileType.video", group: "video" },
  mov: { labelKey: "fileType.video", group: "video" },
  avi: { labelKey: "fileType.video", group: "video" },
  mkv: { labelKey: "fileType.video", group: "video" },
  webm: { labelKey: "fileType.video", group: "video" },
  m4v: { labelKey: "fileType.video", group: "video" },
  mp3: { labelKey: "fileType.audio", group: "audio" },
  wav: { labelKey: "fileType.audio", group: "audio" },
  flac: { labelKey: "fileType.audio", group: "audio" },
  opus: { labelKey: "fileType.audio", group: "audio" },
  m4a: { labelKey: "fileType.audio", group: "audio" },
  ogg: { labelKey: "fileType.audio", group: "audio" },
  zip: { labelKey: "fileType.archive", group: "archive" },
  rar: { labelKey: "fileType.archive", group: "archive" },
  "7z": { labelKey: "fileType.archive", group: "archive" },
  tar: { labelKey: "fileType.archive", group: "archive" },
  js: { labelKey: "fileType.js", group: "code" },
  ts: { labelKey: "fileType.ts", group: "code" },
  jsx: { labelKey: "fileType.jsx", group: "code" },
  tsx: { labelKey: "fileType.tsx", group: "code" },
  py: { labelKey: "fileType.python", group: "code" },
  rs: { labelKey: "fileType.rust", group: "code" },
  xls: { labelKey: "fileType.excel", group: "sheet" },
  xlsx: { labelKey: "fileType.excel", group: "sheet" },
  csv: { labelKey: "fileType.csv", group: "sheet" },
};

export const getFileType = (filename?: string | null, kind?: FileKind): FileTypeInfo => {
  const ext = getExt(filename);
  if (kind && kind !== "other") {
    const kindInfo = KIND_MAP[kind];
    if (kindInfo) return { ...kindInfo, ext };
  }
  if (!filename) return { labelKey: "fileType.file", group: "other", ext: "" };
  const mapped = FILE_TYPE_MAP[ext];
  if (mapped) return { ...mapped, ext };
  if (!ext) return { labelKey: "fileType.unknown", group: "other", ext: "" };
  return { labelKey: "fileType.custom", group: "other", ext };
};

export const FILE_COLORS: Record<FileGroup, string> = {
  doc: "#3b82f6",
  image: "#10b981",
  video: "#f43f5e",
  audio: "#a855f7",
  archive: "#f97316",
  code: "#eab308",
  sheet: "#16a34a",
  other: "#6366f1",
};

export const getColor = (filename?: string | null, kind?: FileKind): string => {
  const { group } = getFileType(filename, kind);
  return FILE_COLORS[group] || "#6366f1";
};

export const genSid = (prefix: string): string =>
  `${prefix}-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
