import {
  desktopUpdaterCheck,
  desktopUpdaterDownloadAndInstall,
  parseDesktopUpdaterCheck,
  type DesktopUpdaterDownloadEvent,
} from "../desktop/updater";

export type UpdaterCheckUpdate = {
  rid: number;
  version?: string;
  currentVersion?: string;
  date?: string;
  body?: string;
};

export type UpdaterCheckResult = UpdaterCheckUpdate | null;

export function parseUpdaterCheckResult(value: unknown): UpdaterCheckResult {
  return parseDesktopUpdaterCheck(value);
}

export async function updaterCheck(): Promise<UpdaterCheckResult> {
  return desktopUpdaterCheck();
}

export async function updaterDownloadAndInstall(options: {
  rid: number;
  onEvent?: (event: DesktopUpdaterDownloadEvent) => void;
  timeoutMs?: number;
}): Promise<boolean | null> {
  return desktopUpdaterDownloadAndInstall(options);
}

export type UpdaterDownloadEvent = DesktopUpdaterDownloadEvent;
