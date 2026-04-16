export const HOME_OVERVIEW_LOGS_PRIMARY_LAYOUT_STORAGE_KEY =
  "aio-home-overview-logs-primary-layout";

type Listener = () => void;

const listeners = new Set<Listener>();

function emit() {
  for (const listener of listeners) listener();
}

function handleStorageEvent(event: StorageEvent) {
  if (event.key === HOME_OVERVIEW_LOGS_PRIMARY_LAYOUT_STORAGE_KEY) {
    emit();
  }
}

export function readHomeOverviewLogsPrimaryLayoutFromStorage(): boolean {
  if (typeof window === "undefined") return false;

  try {
    const raw = window.localStorage.getItem(HOME_OVERVIEW_LOGS_PRIMARY_LAYOUT_STORAGE_KEY);
    if (!raw) return false;
    return raw === "true";
  } catch {
    return false;
  }
}

export function writeHomeOverviewLogsPrimaryLayoutToStorage(enabled: boolean) {
  if (typeof window === "undefined") return;

  try {
    window.localStorage.setItem(
      HOME_OVERVIEW_LOGS_PRIMARY_LAYOUT_STORAGE_KEY,
      String(Boolean(enabled))
    );
  } catch {}

  emit();
}

export function subscribeHomeOverviewLogsPrimaryLayout(listener: Listener) {
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
