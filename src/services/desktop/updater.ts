import { Channel } from "@tauri-apps/api/core";
import { commands } from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";
import { createRiskyIpcConfirm } from "../ipcConfirm";
import { invokeTauriOrNull } from "../tauriInvoke";

export const DESKTOP_UPDATER_HANDWRITTEN_COMMAND = "desktop_updater_download_and_install";
export const DESKTOP_UPDATER_HANDWRITTEN_REASON =
  "Requires a Tauri Channel callback, so this desktop updater path stays as the single handwritten desktop IPC exception.";

export type DesktopUpdaterCheck = {
  rid: number;
  version?: string;
  currentVersion?: string;
  date?: string;
  body?: string;
};

export type DesktopUpdaterDownloadEvent =
  | { event: "started"; data?: { contentLength?: number } }
  | { event: "progress"; data?: { chunkLength?: number } }
  | { event: "finished"; data?: unknown };

export function validateDesktopUpdaterRid(rid: number): number {
  if (!Number.isSafeInteger(rid) || rid < 0) {
    throw new Error(`SEC_INVALID_INPUT: invalid updater rid=${rid}`);
  }
  return rid;
}

export function normalizeDesktopUpdaterTimeoutMs(timeoutMs?: number | null): number | null {
  if (timeoutMs == null) return null;
  if (!Number.isSafeInteger(timeoutMs) || timeoutMs <= 0) {
    throw new Error(`SEC_INVALID_INPUT: invalid updater timeoutMs=${timeoutMs}`);
  }
  return timeoutMs;
}

function asOptionalString(value: unknown) {
  return typeof value === "string" ? value : undefined;
}

function asOptionalNonNegativeSafeInteger(value: unknown) {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0 ? value : undefined;
}

export function parseDesktopUpdaterCheck(value: unknown): DesktopUpdaterCheck | null {
  if (value == null || value === false || typeof value !== "object") {
    return null;
  }

  const obj = value as Record<string, unknown>;
  const rid = asOptionalNonNegativeSafeInteger(obj.rid);
  if (rid == null) {
    return null;
  }

  return {
    rid,
    version: asOptionalString(obj.version),
    currentVersion: asOptionalString(obj.currentVersion),
    date: asOptionalString(obj.date),
    body: asOptionalString(obj.body),
  };
}

function parseDesktopUpdaterDownloadEvent(value: unknown): DesktopUpdaterDownloadEvent | null {
  if (value == null || typeof value !== "object") {
    return null;
  }

  const obj = value as Record<string, unknown>;
  const event = obj.event;
  if (event !== "started" && event !== "progress" && event !== "finished") {
    return null;
  }

  const data = obj.data;
  if (event === "started") {
    const startedData =
      data && typeof data === "object"
        ? {
            contentLength: asOptionalNonNegativeSafeInteger(
              (data as Record<string, unknown>).contentLength
            ),
          }
        : undefined;
    return { event, data: startedData };
  }

  if (event === "progress") {
    const progressData =
      data && typeof data === "object"
        ? {
            chunkLength: asOptionalNonNegativeSafeInteger(
              (data as Record<string, unknown>).chunkLength
            ),
          }
        : undefined;
    return { event, data: progressData };
  }

  return { event, data };
}

export async function desktopUpdaterCheck(options?: {
  timeoutMs?: number | null;
}): Promise<DesktopUpdaterCheck | null> {
  const timeout = normalizeDesktopUpdaterTimeoutMs(options?.timeoutMs);
  const result = await invokeGeneratedIpc<unknown, null>({
    title: "检查更新失败",
    cmd: "desktop_updater_check",
    args: { timeout },
    invoke: () => commands.desktopUpdaterCheck(timeout) as Promise<GeneratedCommandResult<unknown>>,
    nullResultBehavior: "return_fallback",
    fallback: null,
  });
  return parseDesktopUpdaterCheck(result);
}

export async function desktopUpdaterDownloadAndInstall(options: {
  rid: number;
  onEvent?: (event: DesktopUpdaterDownloadEvent) => void;
  timeoutMs?: number;
}) {
  // Generated bindings cover the check path. Install stays handwritten because
  // the Rust command accepts a Channel callback and Specta cannot express it.
  const rid = validateDesktopUpdaterRid(options.rid);
  const timeout = normalizeDesktopUpdaterTimeoutMs(options.timeoutMs);
  const onEvent = typeof options.onEvent === "function" ? options.onEvent : undefined;
  const channel = new Channel<unknown>((message) => {
    const evt = parseDesktopUpdaterDownloadEvent(message);
    if (!evt) {
      return;
    }
    onEvent?.(evt);
  });
  const confirm = createRiskyIpcConfirm("desktop_updater_download_and_install", `updater:${rid}`);

  return invokeTauriOrNull<boolean>(
    DESKTOP_UPDATER_HANDWRITTEN_COMMAND,
    {
      rid,
      onEvent: channel,
      timeout,
      confirm,
    },
    { timeoutMs: null }
  );
}
