import { call } from "./call";
import type {
  DriveScope,
  FoldersResponse,
  IdLike,
  JsonRecord,
  Nullable,
  PaginatedFilesResponse,
  StatsResponse,
  TenantDescriptor,
} from "./types";

const createSessionId = (): string => {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  
  // CSPRNG fallback for older webviews
  const array = new Uint8Array(16);
  globalThis.crypto.getRandomValues(array);
  
  array[6] = (array[6] & 0x0f) | 0x40; // Version 4
  array[8] = (array[8] & 0x3f) | 0x80; // Variant RFC4122
  
  const hex = Array.from(array, b => b.toString(16).padStart(2, "0")).join("");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
};

export type {
  DriveScope,
  DriveRecord,
  IdLike,
  JsonRecord,
  Nullable,
  PaginatedFilesResponse,
  FoldersResponse,
  StatsResponse,
  TenantDescriptor,
} from "./types";

export const DriveApi = {
  getFolders: (driveScope: Nullable<DriveScope> = null): Promise<FoldersResponse> =>
    call("get_folders", { driveScope }, { feature: "drive", action: "get_folders" }),
  createFolder: (
    name: string,
    parentId: Nullable<number> = null,
    driveScope: Nullable<DriveScope> = null,
  ): Promise<unknown> =>
    call("create_folder", { name, parentId, driveScope }, { feature: "drive", action: "create_folder" }),
  renameFolder: (folderId: IdLike, newName: string): Promise<unknown> =>
    call("rename_folder", { folderId, newName }, { feature: "drive", action: "rename_folder" }),
  deleteFolder: (folderId: IdLike): Promise<unknown> =>
    call("delete_folder", { folderId }, { feature: "drive", action: "delete_folder" }),

  getFiles: (
    folderId: Nullable<number> = null,
    driveScope: Nullable<DriveScope> = null,
  ): Promise<unknown> =>
    call("get_files", { folderId, driveScope }, { feature: "drive", action: "get_files" }),
  getAllFiles: (): Promise<unknown> =>
    call("get_all_files", {}, { feature: "drive", action: "get_all_files" }),
  getAllFilesPaginated: (
    cursor: string | null = null,
    limit = 50,
    driveScope: Nullable<DriveScope> = null,
  ): Promise<PaginatedFilesResponse> =>
    call(
      "get_all_files_paginated",
      { cursor, limit, driveScope },
      { feature: "drive", action: "get_all_files_paginated" },
    ),
  getRecentFilesPaginated: (
    cursor: string | null = null,
    limit = 50,
    driveScope: Nullable<DriveScope> = null,
  ): Promise<PaginatedFilesResponse> =>
    call(
      "get_recent_files_paginated",
      { cursor, limit, driveScope },
      { feature: "drive", action: "get_recent_files_paginated" },
    ),
  getTrashPaginated: (
    cursor: string | null = null,
    limit = 50,
    driveScope: Nullable<DriveScope> = null,
  ): Promise<PaginatedFilesResponse> =>
    call(
      "get_trash_paginated",
      { cursor, limit, driveScope },
      { feature: "drive", action: "get_trash_paginated" },
    ),
  getTransfersPaginated: (cursor: string | null = null, limit = 50): Promise<unknown> =>
    call("get_transfers_paginated", { cursor, limit }, { feature: "drive", action: "get_transfers_paginated" }),
  renameFile: (fileId: IdLike, newName: string): Promise<unknown> =>
    call("rename_file", { fileId, newName }, { feature: "drive", action: "rename_file" }),
  deleteFile: (fileId: IdLike): Promise<unknown> =>
    call("delete_file", { fileId }, { feature: "drive", action: "delete_file" }),
  moveFile: (fileId: IdLike, folderId: Nullable<number>): Promise<unknown> =>
    call("move_file", { fileId, folderId }, { feature: "drive", action: "move_file" }),
  moveFolder: (folderId: IdLike, parentId: Nullable<number>): Promise<unknown> =>
    call("move_folder", { folderId, parentId }, { feature: "drive", action: "move_folder" }),
  getFile: (fileId: IdLike): Promise<unknown> =>
    call("get_file", { fileId }, { feature: "drive", action: "get_file" }),
  forwardFileToShared: (fileId: IdLike): Promise<unknown> =>
    call("forward_file_to_shared", { fileId }, { feature: "drive", action: "forward_file_to_shared" }),
  toggleStar: (id: IdLike, isFolder: boolean, starred: boolean): Promise<unknown> =>
    call("toggle_star", { id, isFolder, starred }, { feature: "drive", action: "toggle_star" }),

  getConnectionStatus: (): Promise<unknown> =>
    call("get_connection_status", {}, { feature: "diagnostics", action: "get_connection_status" }),
  getVersion: (): Promise<unknown> =>
    call("get_version", {}, { feature: "diagnostics", action: "get_version" }),
  getStats: (driveScope: Nullable<DriveScope> = null): Promise<StatsResponse> =>
    call("get_stats", { driveScope }, { feature: "drive", action: "get_stats" }),
  getSettings: (): Promise<unknown> =>
    call("get_settings", {}, { feature: "settings", action: "get_settings" }),
  getBootstrapStatus: (): Promise<unknown> =>
    call("get_bootstrap_status", {}, { feature: "diagnostics", action: "get_bootstrap_status" }),
  getOnboardingState: (): Promise<unknown> =>
    call("get_onboarding_state", {}, { feature: "onboarding", action: "get_onboarding_state" }),
  loadOnboardingDestinations: (): Promise<unknown> =>
    call("load_onboarding_destinations", {}, { feature: "onboarding", action: "load_onboarding_destinations" }),
  saveDiscordToken: (token: string): Promise<unknown> =>
    call("save_discord_token", { token }, { feature: "onboarding", action: "save_discord_token" }),
  saveTelegramCredentials: (phone: string, apiId: number, apiHash: string): Promise<unknown> =>
    call(
      "save_telegram_credentials",
      { phone, apiId, apiHash },
      { feature: "onboarding", action: "save_telegram_credentials" },
    ),
  sendTelegramLoginCode: (): Promise<unknown> =>
    call("send_telegram_login_code", {}, { feature: "onboarding", action: "send_telegram_login_code" }),
  submitTelegramLoginCode: (code: string): Promise<unknown> =>
    call("submit_telegram_login_code", { code }, { feature: "onboarding", action: "submit_telegram_login_code" }),
  submitTelegramPassword: (password: string): Promise<unknown> =>
    call("submit_telegram_password", { password }, { feature: "onboarding", action: "submit_telegram_password" }),
  createOnboardingTenant: (
    scope: DriveScope,
    discordGuildId: string | null = null,
    telegramGroupId: string | null = null,
  ): Promise<unknown> =>
    call(
      "create_onboarding_tenant",
      { scope, discordGuildId, telegramGroupId },
      { feature: "onboarding", action: "create_onboarding_tenant" },
    ),
  getActiveTenant: (): Promise<unknown> =>
    call("get_active_tenant", {}, { feature: "tenant", action: "get_active_tenant" }),
  getActiveTenants: (): Promise<unknown> =>
    call("get_active_tenants", {}, { feature: "tenant", action: "get_active_tenants" }),
  listTenants: (): Promise<unknown> =>
    call("list_tenants", {}, { feature: "tenant", action: "list_tenants" }),
  renameTenantDisplayName: (tenant: TenantDescriptor, displayName: string | null): Promise<unknown> =>
    call(
      "rename_tenant_display_name",
      { tenant, displayName },
      { feature: "tenant", action: "rename_tenant_display_name" },
    ),
  switchTenant: (tenant: TenantDescriptor): Promise<unknown> =>
    call("switch_tenant", { tenant }, { feature: "tenant", action: "switch_tenant" }),
  deleteTenant: (tenant: TenantDescriptor): Promise<unknown> =>
    call("delete_tenant", { tenant }, { feature: "tenant", action: "delete_tenant" }),
  saveSettings: (config: JsonRecord): Promise<unknown> =>
    call("save_settings", { config }, { feature: "settings", action: "save_settings" }),
  openBotEnv: (): Promise<unknown> =>
    call("open_bot_env", {}, { feature: "diagnostics", action: "open_bot_env" }),
  getLogStatus: (): Promise<unknown> =>
    call("get_log_status", {}, { feature: "settings", action: "get_log_status" }),
  createFeatureLogFile: (feature: string): Promise<unknown> =>
    call("create_feature_log_file", { feature }, { feature: "settings", action: "create_feature_log_file" }),
  openLogsDir: (): Promise<unknown> =>
    call("open_logs_dir", {}, { feature: "settings", action: "open_logs_dir" }),

  getTrash: (): Promise<unknown> =>
    call("get_trash", {}, { feature: "drive", action: "get_trash" }),
  emptyTrash: (): Promise<unknown> =>
    call("empty_trash", {}, { feature: "drive", action: "empty_trash" }),
  restoreFile: (fileId: IdLike): Promise<unknown> =>
    call("restore_file", { fileId }, { feature: "drive", action: "restore_file" }),
  purgeFile: (fileId: IdLike): Promise<unknown> =>
    call("purge_file", { fileId }, { feature: "drive", action: "purge_file" }),

  uploadFile: (
    filePath: string,
    folderId: Nullable<number> = null,
    driveScope: Nullable<DriveScope> = null,
    sessionId: string | null = null,
    profileId: string | null = null,
    uploadPlan: JsonRecord | null = null,
  ): Promise<unknown> => {
    const sid = sessionId ?? createSessionId();
    return call(
      "upload_file_from_path",
      {
        filePath,
        folderId,
        driveScope,
        sessionId: sid,
        profileId,
        uploadPlan,
      },
      { feature: "upload", action: "upload_file_from_path" },
    );
  },
  uploadFilesFromPaths: (
    filePaths: string[],
    folderId: Nullable<number> = null,
    driveScope: Nullable<DriveScope> = null,
    sessionId: string | null = null,
    profileId: string | null = null,
    uploadPlan: JsonRecord | null = null,
  ): Promise<unknown> => {
    const sid = sessionId ?? createSessionId();
    return call(
      "upload_files_from_paths",
      {
        filePaths,
        folderId,
        driveScope,
        sessionId: sid,
        profileId,
        uploadPlan,
      },
      { feature: "upload", action: "upload_files_from_paths" },
    );
  },

  getUploadProfiles: (): Promise<unknown> =>
    call("get_upload_profiles", {}, { feature: "upload", action: "get_upload_profiles" }),
  saveUploadProfile: (profile: JsonRecord): Promise<unknown> =>
    call("save_upload_profile", { profile }, { feature: "upload", action: "save_upload_profile" }),
  deleteUploadProfile: (id: IdLike): Promise<unknown> =>
    call("delete_upload_profile", { id }, { feature: "upload", action: "delete_upload_profile" }),
  restoreDefaultProfiles: (): Promise<unknown> =>
    call("restore_default_profiles", {}, { feature: "upload", action: "restore_default_profiles" }),
  getUploadProfileRules: (profileId: Nullable<IdLike> = null): Promise<unknown> =>
    call("get_upload_profile_rules", { profileId }, { feature: "upload", action: "get_upload_profile_rules" }),
  saveUploadProfileRule: (rule: JsonRecord): Promise<unknown> =>
    call("save_upload_profile_rule", { rule }, { feature: "upload", action: "save_upload_profile_rule" }),
  deleteUploadProfileRule: (id: IdLike): Promise<unknown> =>
    call("delete_upload_profile_rule", { id }, { feature: "upload", action: "delete_upload_profile_rule" }),
  saveUploadProfileRulesBulk: (profileId: IdLike, orderedRuleIds: IdLike[]): Promise<unknown> =>
    call(
      "save_upload_profile_rules_bulk",
      { profileId, orderedRuleIds },
      { feature: "upload", action: "save_upload_profile_rules_bulk" },
    ),
  resolveUploadProfileForBatch: (items: JsonRecord[]): Promise<unknown> =>
    call(
      "resolve_upload_profile_for_batch",
      { items },
      { feature: "upload", action: "resolve_upload_profile_for_batch" },
    ),

  downloadToDisk: (fileId: IdLike, savePath: string, sessionId: string): Promise<unknown> =>
    call(
      "download_file_to_disk",
      { fileId, savePath, sessionId },
      { feature: "download", action: "download_file_to_disk" },
    ),
  queueDownload: (fileId: IdLike, targetPath: string): Promise<unknown> =>
    call("queue_download", { fileId, targetPath }, { feature: "download", action: "queue_download" }),
  listDownloadJobs: (): Promise<unknown> =>
    call("list_download_jobs", {}, { feature: "download", action: "list_download_jobs" }),
  pauseDownload: (jobId: IdLike): Promise<unknown> =>
    call("pause_download", { jobId }, { feature: "download", action: "pause_download" }),
  resumeDownload: (jobId: IdLike): Promise<unknown> =>
    call("resume_download", { jobId }, { feature: "download", action: "resume_download" }),
  cancelDownload: (jobId: IdLike): Promise<unknown> =>
    call("cancel_download", { jobId }, { feature: "download", action: "cancel_download" }),
  retryDownload: (jobId: IdLike): Promise<unknown> =>
    call("retry_download", { jobId }, { feature: "download", action: "retry_download" }),
  openDownloadFile: (path: string): Promise<unknown> =>
    call("open_download_file", { path }, { feature: "download", action: "open_download_file" }),
  openDownloadFolder: (path: string): Promise<unknown> =>
    call("open_download_folder", { path }, { feature: "download", action: "open_download_folder" }),

  getFilePart: (fileId: IdLike, partNum: number): Promise<unknown> =>
    call("get_file_part", { fileId, partNum }, { feature: "player", action: "get_file_part" }),
};
