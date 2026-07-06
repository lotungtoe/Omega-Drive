export interface AppError {
  code: string;
  message: string;
  context?: Record<string, unknown> | null;
  retryable?: boolean;
  source?: string | null;
  stack?: string | null;
  raw?: unknown;
}

export type ErrorWithAppFields = Error & {
  code?: string;
  context?: Record<string, unknown> | null;
  retryable?: boolean;
  source?: string | null;
};

export interface NestedErrorRecord {
  code?: string;
  message?: string;
  context?: Record<string, unknown> | null;
  retryable?: boolean;
  source?: string | null;
}

export interface ErrorRecord extends NestedErrorRecord {
  stack?: string | null;
  error?: NestedErrorRecord;
}

export const ERROR_CODES = {
  UNKNOWN: "E_UNKNOWN",
  INVALID_INPUT: "E_INVALID_INPUT",
  NOT_FOUND: "E_NOT_FOUND",
  CONFLICT: "E_CONFLICT",
  PERMISSION: "E_PERMISSION",
  DB: "E_DB",
  IO: "E_IO",
  JSON: "E_JSON",
  NETWORK: "E_NETWORK",
  TIMEOUT: "E_TIMEOUT",
  UNAVAILABLE: "E_UNAVAILABLE",
  NOT_READY: "E_NOT_READY",
  UPLOAD_FAILED: "E_UPLOAD_FAILED",
  UPLOAD_CONFLICT: "E_UPLOAD_CONFLICT",
  DOWNLOAD_FAILED: "E_DOWNLOAD_FAILED",
  PLAYER_UNSUPPORTED: "E_PLAYER_UNSUPPORTED",
  PLAYER_INIT_FAILED: "E_PLAYER_INIT_FAILED",
  SETTINGS_INVALID: "E_SETTINGS_INVALID",
} as const;
