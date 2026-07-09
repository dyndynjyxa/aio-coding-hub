import { emitListenerSnapshot } from "../../utils/listeners";

export const HOME_USAGE_DAY_START_HOUR_STORAGE_KEY = "homeUsageDayStartHour";
export const HOME_USAGE_DEFAULT_DAY_START_HOUR = 0;
export const HOME_USAGE_DAY_START_HOUR_OPTIONS = Array.from({ length: 10 }, (_, hour) => hour);
export const HOME_USAGE_DAY_WINDOW_MS = 24 * 60 * 60 * 1000;

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

  if (event.key === HOME_USAGE_DAY_START_HOUR_STORAGE_KEY || event.key === null) {
    emit();
  }
}

function normalizedHourOfDay(hour: number) {
  if (!Number.isSafeInteger(hour)) return 0;
  return ((hour % 24) + 24) % 24;
}

function formatHour(hour: number) {
  return String(normalizedHourOfDay(hour)).padStart(2, "0");
}

function localDateKeyFromDate(date: Date) {
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(
    date.getDate()
  ).padStart(2, "0")}`;
}

export function normalizeHomeUsageDayStartHour(value: number | null | undefined) {
  if (value == null || !Number.isSafeInteger(value)) return HOME_USAGE_DEFAULT_DAY_START_HOUR;
  if (value < 0 || value > 9) return HOME_USAGE_DEFAULT_DAY_START_HOUR;
  return value;
}

export function dayStartHourLabel(hour: number) {
  return `${formatHour(normalizeHomeUsageDayStartHour(hour))}:00`;
}

export function readHomeUsageDayStartHourFromStorage() {
  if (typeof window === "undefined") return HOME_USAGE_DEFAULT_DAY_START_HOUR;

  try {
    const raw = window.localStorage.getItem(HOME_USAGE_DAY_START_HOUR_STORAGE_KEY);
    if (raw == null) return HOME_USAGE_DEFAULT_DAY_START_HOUR;
    return normalizeHomeUsageDayStartHour(Number(raw));
  } catch {
    return HOME_USAGE_DEFAULT_DAY_START_HOUR;
  }
}

export function writeHomeUsageDayStartHourToStorage(hour: number) {
  if (typeof window === "undefined") return;

  const normalized = normalizeHomeUsageDayStartHour(hour);
  try {
    window.localStorage.setItem(HOME_USAGE_DAY_START_HOUR_STORAGE_KEY, String(normalized));
  } catch {}

  emit();
}

export function subscribeHomeUsageDayStartHour(listener: Listener) {
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

export function startOfLocalUsageDay(date: Date, dayStartHour: number) {
  const normalizedDayStartHour = normalizeHomeUsageDayStartHour(dayStartHour);
  const start = new Date(
    date.getFullYear(),
    date.getMonth(),
    date.getDate(),
    normalizedDayStartHour,
    0,
    0,
    0
  );
  if (date.getTime() < start.getTime()) {
    return new Date(
      date.getFullYear(),
      date.getMonth(),
      date.getDate() - 1,
      normalizedDayStartHour,
      0,
      0,
      0
    );
  }
  return start;
}

export function addLocalDays(date: Date, days: number) {
  return new Date(
    date.getFullYear(),
    date.getMonth(),
    date.getDate() + days,
    date.getHours(),
    date.getMinutes(),
    date.getSeconds(),
    date.getMilliseconds()
  );
}

export function localDateHour(dateValue: string, hour: number, dayOffset = 0) {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(dateValue);
  if (!match) return null;
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const date = new Date(
    year,
    month - 1,
    day + dayOffset,
    normalizeHomeUsageDayStartHour(hour),
    0,
    0,
    0
  );
  if (Number.isNaN(date.getTime())) return null;
  return date;
}

export function orderedUsageDayHours(dayStartHour: number) {
  const normalizedDayStartHour = normalizeHomeUsageDayStartHour(dayStartHour);
  return Array.from({ length: 24 }, (_, index) => (normalizedDayStartHour + index) % 24);
}

export function formatUsageDayHourLabel(hour: number, dayStartHour: number) {
  const normalizedDayStartHour = normalizeHomeUsageDayStartHour(dayStartHour);
  const normalizedHour = normalizedHourOfDay(hour);
  const prefix = normalizedHour < normalizedDayStartHour ? "次日" : "";
  return `${prefix}${formatHour(normalizedHour)}:00`;
}

export function formatUsageDayHourTickLabel(hour: number, dayStartHour: number) {
  const normalizedDayStartHour = normalizeHomeUsageDayStartHour(dayStartHour);
  const normalizedHour = normalizedHourOfDay(hour);
  const prefix = normalizedHour < normalizedDayStartHour ? "次日" : "";
  return `${prefix}${formatHour(normalizedHour)}`;
}

function formatLocalHourMinuteFromMs(value: number) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return null;
  return `${formatHour(date.getHours())}:${String(date.getMinutes()).padStart(2, "0")}`;
}

export function formatUsageDayHourMinuteFromMs(
  value: number,
  dayKey: string,
  dayStartHour = HOME_USAGE_DEFAULT_DAY_START_HOUR
) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return null;
  const timeText = formatLocalHourMinuteFromMs(value);
  if (!timeText) return null;

  const usageDayStart = localDateHour(dayKey, dayStartHour);
  if (!usageDayStart) return timeText;
  const usageDayEnd = addLocalDays(usageDayStart, 1);
  const insideUsageDay =
    date.getTime() >= usageDayStart.getTime() && date.getTime() < usageDayEnd.getTime();
  if (insideUsageDay && localDateKeyFromDate(date) !== localDateKeyFromDate(usageDayStart)) {
    return `次日${timeText}`;
  }

  return timeText;
}
