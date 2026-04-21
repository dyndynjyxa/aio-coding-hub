import { Channel } from "@tauri-apps/api/core";
import { commands } from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";
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

function asOptionalString(value: unknown) {
  return typeof value === "string" ? value : undefined;
}

function asOptionalNumber(value: unknown) {
  return typeof value === "number" && Number.isFinite(value) ? value : undefined;
}

export function parseDesktopUpdaterCheck(value: unknown): DesktopUpdaterCheck | null {
  if (value == null || value === false || typeof value !== "object") {
    return null;
  }

  const obj = value as Record<string, unknown>;
  const rid = asOptionalNumber(obj.rid);
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
        ? { contentLength: asOptionalNumber((data as Record<string, unknown>).contentLength) }
        : undefined;
    return { event, data: startedData };
  }

  if (event === "progress") {
    const progressData =
      data && typeof data === "object"
        ? { chunkLength: asOptionalNumber((data as Record<string, unknown>).chunkLength) }
        : undefined;
    return { event, data: progressData };
  }

  return { event, data };
}

export async function desktopUpdaterCheck(options?: {
  timeoutMs?: number | null;
}): Promise<DesktopUpdaterCheck | null> {
  const result = await invokeGeneratedIpc<unknown, null>({
    title: "检查更新失败",
    cmd: "desktop_updater_check",
    args: { timeout: options?.timeoutMs ?? null },
    invoke: () =>
      commands.desktopUpdaterCheck(options?.timeoutMs ?? null) as Promise<
        GeneratedCommandResult<unknown>
      >,
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
  const onEvent = typeof options.onEvent === "function" ? options.onEvent : undefined;
  const channel = new Channel<unknown>((message) => {
    const evt = parseDesktopUpdaterDownloadEvent(message);
    if (!evt) {
      return;
    }
    onEvent?.(evt);
  });

  return invokeTauriOrNull<boolean>(
    DESKTOP_UPDATER_HANDWRITTEN_COMMAND,
    {
      rid: options.rid,
      onEvent: channel,
      timeout: typeof options.timeoutMs === "number" ? options.timeoutMs : null,
    },
    { timeoutMs: null }
  );
}
