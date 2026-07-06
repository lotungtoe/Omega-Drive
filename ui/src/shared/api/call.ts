import { invoke } from "@tauri-apps/api/core";
import { type CallFn, type JsonRecord } from "./types";

export type { CallOptions } from "./types";
import { reportError } from "../services/errors/reportError";
import { handleMockCall } from "./mocks";










function sanitizeArgs(args: unknown = {}) {
  if (!args || typeof args !== "object") return args;
  const out = {} as JsonRecord;
  for (const [key, value] of Object.entries(args)) {
    if (typeof value === "string" && value.length > 300) {
      out[key] = `${value.slice(0, 300)}...`;
    } else {
      out[key] = value;
    }
  }
  return out;
}

export const call = (async (cmd, args = {}, options = {}) => {

  const { feature = "frontend", action = cmd, context = {} } = options;
  const sanitizedArgs = sanitizeArgs(args);

  const isMpvCommand = cmd.startsWith("mpv_");
  const traceMpvSuccess = isMpvCommand && cmd !== "mpv_get_status";

  if (globalThis.window !== undefined && !Reflect.has(globalThis, "__TAURI_INTERNALS__")) {
    return handleMockCall(cmd, args);
  }

  try {
    if (traceMpvSuccess) {
      console.info("[TauriCall] invoke:start", { cmd, args: sanitizedArgs });
    }

    const result = await invoke(cmd, args);

    if (traceMpvSuccess) {
      console.info("[TauriCall] invoke:success", { cmd, result });
    }

    return result;
  } catch (err) {
    if (isMpvCommand) {
      console.error("[TauriCall] invoke:error", { cmd, args: sanitizedArgs, err });
    }

    const appErr = reportError(feature, action, err, {
      cmd,
      args: sanitizedArgs ?? {},
      ...context,
    });

    throw appErr;
  }
}) as CallFn;
