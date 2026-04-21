import { appEventNames } from "../../constants/appEvents";
import { commands } from "../../generated/bindings";
import type { AppStartupStatus } from "../../generated/bindings";
import { listenDesktopEvent } from "../desktop/event";

export type { AppStartupStage, AppStartupStatus } from "../../generated/bindings";

export async function appStartupStatusGet(): Promise<AppStartupStatus> {
  return commands.appStartupStatusGet();
}

export async function appStartupRetry(): Promise<AppStartupStatus> {
  return commands.appStartupRetry();
}

export async function listenAppStartupStatusEvents(
  onStatus: (status: AppStartupStatus) => void
): Promise<() => void> {
  return listenDesktopEvent<AppStartupStatus>(appEventNames.startupStatus, (payload) => {
    if (!payload) return;
    onStatus(payload);
  });
}
