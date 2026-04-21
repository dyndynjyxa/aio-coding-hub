import { useSyncExternalStore } from "react";
import type { AppStartupStatus } from "../services/app/startupStatus";
import {
  appStartupStatusGet,
  listenAppStartupStatusEvents,
} from "../services/app/startupStatus";

const IDLE_STARTUP_STATUS: AppStartupStatus = Object.freeze({
  running: false,
  currentStage: "idle",
  failedStage: null,
  errorMessage: null,
  canRetry: false,
});

let snapshot: AppStartupStatus = IDLE_STARTUP_STATUS;
const listeners = new Set<() => void>();

function emitSnapshot() {
  for (const listener of listeners) {
    listener();
  }
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function getAppStartupStatusSnapshot(): AppStartupStatus {
  return snapshot;
}

export function setAppStartupStatusSnapshot(next: AppStartupStatus) {
  snapshot = next;
  emitSnapshot();
}

export function resetAppStartupStatusStore() {
  snapshot = IDLE_STARTUP_STATUS;
  emitSnapshot();
}

export async function syncAppStartupStatusSnapshot(): Promise<void> {
  const next = await appStartupStatusGet();
  setAppStartupStatusSnapshot(next);
}

export async function listenAppStartupStatusSnapshot(): Promise<() => void> {
  return listenAppStartupStatusEvents(setAppStartupStatusSnapshot);
}

export function useAppStartupStatus(): AppStartupStatus {
  return useSyncExternalStore(
    subscribe,
    getAppStartupStatusSnapshot,
    getAppStartupStatusSnapshot
  );
}
