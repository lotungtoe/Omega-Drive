import { ERROR_CODES } from "./types";
import type { AppError } from "./types";

const FRIENDLY: Record<string, string> = {
  [ERROR_CODES.INVALID_INPUT]: "Invalid input data.",
  [ERROR_CODES.NOT_FOUND]: "Requested data not found.",
  [ERROR_CODES.CONFLICT]: "Data conflict, please try again.",
  [ERROR_CODES.PERMISSION]: "You do not have permission to perform this action.",
  [ERROR_CODES.DB]: "Storage system encountered an error.",
  [ERROR_CODES.IO]: "Cannot read/write data on this machine.",
  [ERROR_CODES.JSON]: "Configuration data has a format error.",
  [ERROR_CODES.NETWORK]: "Cannot connect to service.",
  [ERROR_CODES.TIMEOUT]: "Request timed out, please try again.",
  [ERROR_CODES.UNAVAILABLE]: "Service is temporarily unavailable.",
  [ERROR_CODES.NOT_READY]: "Resource not ready yet.",
  [ERROR_CODES.UPLOAD_FAILED]: "Upload failed.",
  [ERROR_CODES.UPLOAD_CONFLICT]: "Duplicate file, needs overwrite confirmation.",
  [ERROR_CODES.DOWNLOAD_FAILED]: "Download failed.",
  [ERROR_CODES.PLAYER_UNSUPPORTED]: "Video does not support playback on WebView.",
  [ERROR_CODES.PLAYER_INIT_FAILED]: "Cannot initialize player.",
  [ERROR_CODES.SETTINGS_INVALID]: "Invalid configuration.",
};

type UserMessage = {
  title: string;
  message: string;
  details: Record<string, unknown>;
};

export function toUserMessage(appError: AppError | string | null | undefined): UserMessage {
  if (typeof appError === "string") {
    return {
      title: "An error occurred",
      message: appError,
      details: {},
    };
  }

  const code = appError?.code || ERROR_CODES.UNKNOWN;
  const message = FRIENDLY[code] || appError?.message || "An unknown error has occurred.";
  const details = {
    code,
    message: appError?.message,
    source: appError?.source,
    context: appError?.context,
    stack: appError?.stack,
    retryable: appError?.retryable,
  };

  return {
    title: "An error occurred",
    message,
    details,
  };
}
