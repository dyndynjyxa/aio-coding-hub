import { appEventNames } from "../../constants/appEvents";
import { commands } from "../../generated/bindings";
import type { AppStartupStage, AppStartupStatus } from "../../generated/bindings";
import { listenDesktopEvent } from "../desktop/event";

export type { AppStartupStage, AppStartupStatus } from "../../generated/bindings";

const APP_STARTUP_STAGE_VALUES = [
  "idle",
  "initializing_db",
  "reading_settings",
  "starting_gateway",
  "syncing_cli_proxy",
  "finalizing_wsl",
  "ready",
  "failed",
] as const satisfies readonly AppStartupStage[];
const APP_STARTUP_ERROR_MESSAGE_MAX_CHARS = 4096;

function isRecord(value: unknown): value is Record<string, unknown> {
  return value != null && typeof value === "object" && !Array.isArray(value);
}

function truncateChars(value: string, maxChars: number): string {
  const chars = [...value];
  if (chars.length <= maxChars) return value;
  return chars.slice(0, maxChars).join("");
}

function normalizeStartupBoolean(value: unknown, label: string): boolean {
  if (typeof value === "boolean") return value;
  throw new Error(`IPC_INVALID_RESULT: ${label} must be a boolean`);
}

function normalizeStartupStage(value: unknown, label: string): AppStartupStage {
  if (
    typeof value === "string" &&
    (APP_STARTUP_STAGE_VALUES as readonly string[]).includes(value)
  ) {
    return value as AppStartupStage;
  }
  throw new Error(`IPC_INVALID_RESULT: invalid ${label}=${String(value)}`);
}

function normalizeOptionalStartupStage(value: unknown, label: string): AppStartupStage | null {
  if (value == null) return null;
  return normalizeStartupStage(value, label);
}

function normalizeOptionalStartupErrorMessage(value: unknown): string | null {
  if (value == null) return null;
  if (typeof value !== "string") {
    throw new Error("IPC_INVALID_RESULT: startup.errorMessage must be a string");
  }
  const normalized = value.trim();
  if (!normalized) return null;
  return truncateChars(normalized, APP_STARTUP_ERROR_MESSAGE_MAX_CHARS);
}

export function normalizeAppStartupStatus(value: unknown): AppStartupStatus {
  if (!isRecord(value)) {
    throw new Error("IPC_INVALID_RESULT: startup status must be an object");
  }

  return {
    running: normalizeStartupBoolean(value.running, "startup.running"),
    currentStage: normalizeStartupStage(value.currentStage, "startup.currentStage"),
    failedStage: normalizeOptionalStartupStage(value.failedStage, "startup.failedStage"),
    errorMessage: normalizeOptionalStartupErrorMessage(value.errorMessage),
    canRetry: normalizeStartupBoolean(value.canRetry, "startup.canRetry"),
  };
}

export async function appStartupStatusGet(): Promise<AppStartupStatus> {
  return normalizeAppStartupStatus(await commands.appStartupStatusGet());
}

export async function appStartupRetry(): Promise<AppStartupStatus> {
  return normalizeAppStartupStatus(await commands.appStartupRetry());
}

export async function listenAppStartupStatusEvents(
  onStatus: (status: AppStartupStatus) => void
): Promise<() => void> {
  return listenDesktopEvent<unknown>(appEventNames.startupStatus, (payload) => {
    if (!payload) return;
    onStatus(normalizeAppStartupStatus(payload));
  });
}
