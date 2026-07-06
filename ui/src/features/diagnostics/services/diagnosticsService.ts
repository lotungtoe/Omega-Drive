import { call } from "../../../shared/api/call";
import type { BootstrapStatus, UiVisibilityPayload } from "../../../shared/api/types";

export async function getBootstrapStatus(): Promise<BootstrapStatus> {
  return call(
    "get_bootstrap_status",
    {},
    { feature: "diagnostics", action: "get_bootstrap_status" },
  );
}

export async function openBotEnv(): Promise<unknown> {
  return call("open_bot_env", {}, { feature: "diagnostics", action: "open_bot_env" });
}

export async function openDiscordAuth(): Promise<unknown> {
  return call(
    "open_discord_auth",
    {},
    { feature: "diagnostics", action: "open_discord_auth" },
  );
}

let inFlightConnectionStatus: Promise<unknown> | null = null;

export async function getConnectionStatus(): Promise<unknown> {
  if (inFlightConnectionStatus) {
    return inFlightConnectionStatus;
  }
  inFlightConnectionStatus = call(
    "get_connection_status",
    {},
    { feature: "diagnostics", action: "get_connection_status" },
  ).finally(() => {
    inFlightConnectionStatus = null;
  });
  return inFlightConnectionStatus;
}

export async function reportUiVisibility({
  windowLabel,
  visible,
  focused,
}: UiVisibilityPayload): Promise<unknown> {
  return call(
    "report_ui_visibility",
    { windowLabel, visible, focused },
    { feature: "diagnostics", action: "report_ui_visibility" },
  );
}
