export type JsonRecord = Record<string, unknown>;
export type Nullable<T> = T | null;
export type IdLike = number | string;
export type DriveScope = "my" | "shared";

export interface DriveRecord extends JsonRecord {
  id?: IdLike;
  filename?: string;
  folder_id?: number | null;
}

export interface PaginatedFilesResponse extends JsonRecord {
  files?: DriveRecord[];
  next_cursor?: string | null;
  has_more?: boolean;
}

export interface FoldersResponse extends JsonRecord {
  folders?: DriveRecord[];
}

export interface StatsResponse extends JsonRecord {
  total_size?: number;
  total_files?: number;
  total_folders?: number;
  trash_count?: number;
}

export interface TenantDescriptor extends JsonRecord {
  scope: DriveScope;
  discordGuildId?: string | null;
  telegramGroupId?: string | null;
  displayName?: string | null;
  dbFileName?: string | null;
}

export interface UploadSelection {
  profileId?: string | null;
  uploadPlan?: JsonRecord | null;
}



export interface BootstrapStatus extends JsonRecord {
  discordConfigured?: boolean;
  ffmpegReady?: boolean;
  ffprobeReady?: boolean;
  nativePlayerReady?: boolean;
}

export interface UiVisibilityPayload {
  windowLabel: string;
  visible: boolean;
  focused: boolean;
}

export interface CallOptions {
  feature?: string;
  action?: string;
  context?: JsonRecord;
}

export type CallFn = <T>(cmd: string, args?: JsonRecord, options?: CallOptions) => Promise<T>;
