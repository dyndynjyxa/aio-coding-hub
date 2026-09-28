import { emitListenerSnapshot } from "../../utils/listeners";
import {
  USAGE_DEFAULT_FULL_IDLE_GAP_MINUTES,
  USAGE_DEFAULT_SESSION_BREAK_GAP_MINUTES,
  USAGE_FULL_IDLE_GAP_MINUTES_MAX,
  USAGE_FULL_IDLE_GAP_MINUTES_MIN,
  USAGE_SESSION_BREAK_GAP_MINUTES_MAX,
  USAGE_SESSION_BREAK_GAP_MINUTES_MIN,
} from "../../constants/usageDevelopmentTime";

export const HOME_USAGE_DEVELOPMENT_TIME_STORAGE_KEY = "homeUsageDevelopmentTimeThresholds";
export const HOME_USAGE_DEFAULT_FULL_IDLE_GAP_MINUTES = USAGE_DEFAULT_FULL_IDLE_GAP_MINUTES;
export const HOME_USAGE_DEFAULT_SESSION_BREAK_GAP_MINUTES = USAGE_DEFAULT_SESSION_BREAK_GAP_MINUTES;
export const HOME_USAGE_FULL_IDLE_GAP_MINUTES_OPTIONS = Array.from(
  { length: USAGE_FULL_IDLE_GAP_MINUTES_MAX - USAGE_FULL_IDLE_GAP_MINUTES_MIN + 1 },
  (_, index) => index + USAGE_FULL_IDLE_GAP_MINUTES_MIN
);
export const HOME_USAGE_SESSION_BREAK_GAP_MINUTES_OPTIONS = Array.from(
  { length: USAGE_SESSION_BREAK_GAP_MINUTES_MAX - USAGE_SESSION_BREAK_GAP_MINUTES_MIN + 1 },
  (_, index) => index + USAGE_SESSION_BREAK_GAP_MINUTES_MIN
);

export type HomeUsageDevelopmentTimeThresholds = {
  fullIdleGapMinutes: number;
  sessionBreakGapMinutes: number;
};

type Listener = () => void;

const listeners = new Set<Listener>();

function emit() {
  emitListenerSnapshot(listeners, (listener) => listener());
}

function isLocalStorageEvent(event: StorageEvent) {
  if (typeof window === "undefined" || event.storageArea == null) {
    return true;
  }

  try {
    return event.storageArea === window.localStorage;
  } catch {
    return false;
  }
}

function handleStorageEvent(event: StorageEvent) {
  if (!isLocalStorageEvent(event)) return;
  if (event.key === HOME_USAGE_DEVELOPMENT_TIME_STORAGE_KEY || event.key === null) {
    emit();
  }
}

function normalizeFullIdleGapMinutes(value: unknown) {
  return Number.isSafeInteger(value) &&
    Number(value) >= USAGE_FULL_IDLE_GAP_MINUTES_MIN &&
    Number(value) <= USAGE_FULL_IDLE_GAP_MINUTES_MAX
    ? Number(value)
    : HOME_USAGE_DEFAULT_FULL_IDLE_GAP_MINUTES;
}

function normalizeSessionBreakGapMinutes(value: unknown) {
  return Number.isSafeInteger(value) &&
    Number(value) >= USAGE_SESSION_BREAK_GAP_MINUTES_MIN &&
    Number(value) <= USAGE_SESSION_BREAK_GAP_MINUTES_MAX
    ? Number(value)
    : HOME_USAGE_DEFAULT_SESSION_BREAK_GAP_MINUTES;
}

export function normalizeHomeUsageDevelopmentTimeThresholds(
  value: Partial<HomeUsageDevelopmentTimeThresholds> | null | undefined
): HomeUsageDevelopmentTimeThresholds {
  const fullIdleGapMinutes = normalizeFullIdleGapMinutes(value?.fullIdleGapMinutes);
  const sessionBreakGapMinutes = normalizeSessionBreakGapMinutes(value?.sessionBreakGapMinutes);
  if (fullIdleGapMinutes >= sessionBreakGapMinutes) {
    return {
      fullIdleGapMinutes: HOME_USAGE_DEFAULT_FULL_IDLE_GAP_MINUTES,
      sessionBreakGapMinutes: HOME_USAGE_DEFAULT_SESSION_BREAK_GAP_MINUTES,
    };
  }
  return { fullIdleGapMinutes, sessionBreakGapMinutes };
}

export function readHomeUsageDevelopmentTimeThresholdsFromStorage() {
  if (typeof window === "undefined") {
    return normalizeHomeUsageDevelopmentTimeThresholds(null);
  }

  try {
    const raw = window.localStorage.getItem(HOME_USAGE_DEVELOPMENT_TIME_STORAGE_KEY);
    if (raw == null) return normalizeHomeUsageDevelopmentTimeThresholds(null);
    return normalizeHomeUsageDevelopmentTimeThresholds(JSON.parse(raw));
  } catch {
    return normalizeHomeUsageDevelopmentTimeThresholds(null);
  }
}

export function readHomeUsageFullIdleGapMinutesFromStorage() {
  return readHomeUsageDevelopmentTimeThresholdsFromStorage().fullIdleGapMinutes;
}

export function readHomeUsageSessionBreakGapMinutesFromStorage() {
  return readHomeUsageDevelopmentTimeThresholdsFromStorage().sessionBreakGapMinutes;
}

function writeThresholdsToStorage(thresholds: HomeUsageDevelopmentTimeThresholds) {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(
      HOME_USAGE_DEVELOPMENT_TIME_STORAGE_KEY,
      JSON.stringify(thresholds)
    );
  } catch {}
  emit();
}

export function writeHomeUsageFullIdleGapMinutesToStorage(value: number) {
  const current = readHomeUsageDevelopmentTimeThresholdsFromStorage();
  const fullIdleGapMinutes = normalizeFullIdleGapMinutes(value);
  const sessionBreakGapMinutes =
    fullIdleGapMinutes >= current.sessionBreakGapMinutes
      ? fullIdleGapMinutes + 1
      : current.sessionBreakGapMinutes;
  writeThresholdsToStorage({ fullIdleGapMinutes, sessionBreakGapMinutes });
}

export function writeHomeUsageSessionBreakGapMinutesToStorage(value: number) {
  const current = readHomeUsageDevelopmentTimeThresholdsFromStorage();
  const sessionBreakGapMinutes = normalizeSessionBreakGapMinutes(value);
  const fullIdleGapMinutes =
    sessionBreakGapMinutes <= current.fullIdleGapMinutes
      ? sessionBreakGapMinutes - 1
      : current.fullIdleGapMinutes;
  writeThresholdsToStorage({ fullIdleGapMinutes, sessionBreakGapMinutes });
}

export function subscribeHomeUsageDevelopmentTimeThresholds(listener: Listener) {
  if (listeners.size === 0 && typeof window !== "undefined") {
    window.addEventListener("storage", handleStorageEvent);
  }
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
    if (listeners.size === 0 && typeof window !== "undefined") {
      window.removeEventListener("storage", handleStorageEvent);
    }
  };
}
