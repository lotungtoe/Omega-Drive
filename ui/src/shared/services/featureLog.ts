import { invoke } from "@tauri-apps/api/core";

export type FeatureLogLevel = "trace" | "debug" | "info" | "warn" | "error";

const LEVELS = new Set<FeatureLogLevel>(["trace", "debug", "info", "warn", "error"]);

const isFeatureLogLevel = (level: string): level is FeatureLogLevel => LEVELS.has(level as FeatureLogLevel);

export function featureLog(
  feature: string,
  level: FeatureLogLevel | string,
  message: unknown,
  context: Record<string, unknown> = {},
): void {
  const resolvedLevel: FeatureLogLevel = isFeatureLogLevel(level) ? level : "info";
  const payload = {
    feature,
    level: resolvedLevel,
    message: String(message ?? ""),
    context: context ?? {},
  };

  invoke("log_frontend_event", payload).catch((err: unknown) => {
    console.warn("[featureLog] invoke failed:", payload, err);
  });
}
