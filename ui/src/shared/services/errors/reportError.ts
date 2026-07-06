import * as Sentry from "@sentry/react";
import { featureLog } from "../featureLog";
import { normalizeError } from "./normalizeError";
import type { AppError } from "./types";

const MAX_STRING_LEN = 500;
const MAX_KEYS = 50;

const truncate = (value: unknown): unknown =>
  typeof value === "string" && value.length > MAX_STRING_LEN
    ? `${value.slice(0, MAX_STRING_LEN)}...`
    : value;

const sanitize = (input: unknown, depth = 0): unknown => {
  if (depth > 3) return "[DepthLimit]";
  if (input == null) return input;
  if (Array.isArray(input)) {
    return input.slice(0, 20).map((value) => sanitize(value, depth + 1));
  }
  if (typeof input === "object") {
    const out: Record<string, unknown> = {};
    const entries = Object.entries(input as Record<string, unknown>).slice(0, MAX_KEYS);
    for (const [key, value] of entries) {
      out[key] = sanitize(truncate(value), depth + 1);
    }
    return out;
  }
  return truncate(input);
};

export function reportError(
  feature: string,
  action: string | undefined,
  err: unknown,
  context: Record<string, unknown> = {},
): AppError {
  const appErr = normalizeError(err);
  const payload = {
    feature,
    action,
    errorCode: appErr.code,
    message: appErr.message,
    context: sanitize({
      ...(appErr.context || {}),
      ...(context || {}),
    }),
    source: appErr.source,
    stack: appErr.stack,
    retryable: appErr.retryable,
  };

  if (typeof Sentry?.captureException === "function") {
    Sentry.captureException(appErr, {
      tags: {
        feature,
        action: action || "error",
        errorCode: appErr.code,
      },
      extra: payload,
    });
  }

  featureLog(feature, "error", action || "error", payload);
  console.error("[AppError]", payload, appErr);
  return appErr;
}
