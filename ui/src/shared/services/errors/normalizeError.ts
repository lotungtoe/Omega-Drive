import { ERROR_CODES, type AppError, type ErrorRecord, type ErrorWithAppFields } from "./types";

const isObject = (val: unknown): val is Record<string, unknown> => val != null && typeof val === "object";

const applyOptionalFields = (
  base: AppError,
  fields: {
    context?: Record<string, unknown> | null | undefined;
    retryable?: boolean | undefined;
    source?: string | null | undefined;
    stack?: string | null | undefined;
  },
): AppError => {
  const next: AppError = { ...base };

  if (fields.context !== undefined) next.context = fields.context;
  if (fields.retryable !== undefined) next.retryable = fields.retryable;
  if (fields.source !== undefined) next.source = fields.source;
  if (fields.stack !== undefined) next.stack = fields.stack;

  return next;
};

const tryParseJson = (text: string): unknown | null => {
  if (typeof text !== "string") return null;
  const trimmed = text.trim();
  if (!trimmed.startsWith("{") && !trimmed.startsWith("[")) return null;
  try {
    return JSON.parse(trimmed);
  } catch {
    return null;
  }
};

export function normalizeError(err: unknown): AppError {
  if (!err) {
    return { code: ERROR_CODES.UNKNOWN, message: "Unknown error", raw: err };
  }

  if (typeof err === "string") {
    const parsed = tryParseJson(err);
    if (parsed && isObject(parsed) && typeof parsed.code === "string" && typeof parsed.message === "string") {
      const parsedError = parsed as ErrorRecord;
      return applyOptionalFields(
        {
          code: parsed.code,
          message: parsed.message,
          raw: err,
        },
        {
          context: parsedError.context,
          retryable: parsedError.retryable,
          source: parsedError.source,
        },
      );
    }
    return { code: ERROR_CODES.UNKNOWN, message: err, raw: err };
  }

  if (err instanceof Error) {
    const typedErr = err as ErrorWithAppFields;
    return applyOptionalFields(
      {
        code: typedErr.code || ERROR_CODES.UNKNOWN,
        message: typedErr.message || "Unknown error",
        raw: err,
      },
      {
        context: typedErr.context,
        retryable: typedErr.retryable,
        source: typedErr.source,
        stack: typedErr.stack ?? null,
      },
    );
  }

  if (isObject(err)) {
    const typedErr = err as ErrorRecord;
    const nestedErr = typedErr.error;
    const code = typedErr.code || nestedErr?.code || ERROR_CODES.UNKNOWN;
    const message =
      typedErr.message ||
      nestedErr?.message ||
      (typeof err.toString === "function" ? err.toString() : "Unknown error");
    return applyOptionalFields(
      {
        code,
        message,
        raw: err,
      },
      {
        context: typedErr.context || nestedErr?.context,
        retryable: typedErr.retryable ?? nestedErr?.retryable,
        source: typedErr.source || nestedErr?.source,
        stack: typedErr.stack ?? null,
      },
    );
  }

  return { code: ERROR_CODES.UNKNOWN, message: String(err), raw: err };
}
