import { call } from "../../../shared/api/call";
import type { JsonRecord } from "../../../shared/api/types";

export async function getSettings(): Promise<unknown> {
  return call("get_settings", {}, { feature: "settings", action: "get_settings" });
}

export async function saveSettings(config: JsonRecord): Promise<unknown> {
  return call("save_settings", { config }, { feature: "settings", action: "save_settings" });
}

export async function applySettings(config: JsonRecord): Promise<unknown> {
  return call("apply_settings", { config }, { feature: "settings", action: "apply_settings" });
}

export async function getLogStatus(): Promise<unknown> {
  return call("get_log_status", {}, { feature: "settings", action: "get_log_status" });
}

export async function createFeatureLogFile(feature: string): Promise<unknown> {
  return call(
    "create_feature_log_file",
    { feature },
    { feature: "settings", action: "create_feature_log_file" },
  );
}

export async function openLogsDir(): Promise<unknown> {
  return call("open_logs_dir", {}, { feature: "settings", action: "open_logs_dir" });
}

export async function triggerBackup(): Promise<unknown> {
  return call("trigger_backup_snapshot", {}, { feature: "backup", action: "trigger_snapshot" });
}

export async function getGPUAdapters(): Promise<string[]> {
  return call("get_gpu_adapters", {}, { feature: "settings", action: "get_gpu_adapters" }) as Promise<string[]>;
}
