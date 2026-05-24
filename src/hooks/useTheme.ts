import { useCallback, useSyncExternalStore } from "react";
import { setDesktopWindowTheme } from "../services/desktop/window";
import { listenThemeChanged } from "../services/desktop/themeEvent";
import { emitListenerSnapshot } from "../utils/listeners";

type Theme = "light" | "dark" | "system";

const STORAGE_KEY = "aio-theme";

function canUseWindow(): boolean {
  return typeof window !== "undefined";
}

function normalizeTheme(value: unknown): Theme {
  return value === "light" || value === "dark" || value === "system" ? value : "system";
}

// ---------------------------------------------------------------------------
// Module-level shared store — single source of truth for ALL useTheme() calls
// ---------------------------------------------------------------------------

function getSystemTheme(): "light" | "dark" {
  if (!canUseWindow() || typeof window.matchMedia !== "function") {
    return "light";
  }

  try {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  } catch {
    return "light";
  }
}

function tryReadStoredTheme(): Theme | null {
  if (!canUseWindow()) return "system";

  try {
    return normalizeTheme(window.localStorage.getItem(STORAGE_KEY));
  } catch {
    return null;
  }
}

function readStoredTheme(): Theme {
  return tryReadStoredTheme() ?? "system";
}

interface ThemeSnapshot {
  theme: Theme;
  resolvedTheme: "light" | "dark";
}

function resolve(theme: Theme): "light" | "dark" {
  return theme === "system" ? getSystemTheme() : theme;
}

let currentSnapshot: ThemeSnapshot = (() => {
  const t = readStoredTheme();
  return { theme: t, resolvedTheme: resolve(t) };
})();

type Listener = () => void;
const listeners = new Set<Listener>();
let systemThemeCleanup: (() => void) | null = null;
let storageThemeCleanup: (() => void) | null = null;
let tauriThemeCleanup: (() => void) | null = null;
let pendingTauriThemeToken: object | null = null;

function emitChange() {
  emitListenerSnapshot(listeners, (listener) => listener());
}

export function subscribeTheme(listener: Listener): () => void {
  const wasEmpty = listeners.size === 0;
  listeners.add(listener);
  if (wasEmpty && listeners.size > 0) {
    setupThemeListeners();
  }
  if (refreshSnapshotFromEnvironment()) {
    emitListenerSnapshot(new Set([listener]), (l) => l());
  }

  return () => {
    listeners.delete(listener);
    if (listeners.size === 0) {
      teardownThemeListeners();
    }
  };
}

function getSnapshot(): ThemeSnapshot {
  return currentSnapshot;
}

// SSR / test fallback — same as initial client snapshot
function getServerSnapshot(): ThemeSnapshot {
  return { theme: "system", resolvedTheme: "light" };
}

// ---------------------------------------------------------------------------
// Side-effects: DOM class + native titlebar
// ---------------------------------------------------------------------------

/** Sync native window titlebar theme with the resolved app theme. */
function syncNativeTheme(theme: Theme) {
  Promise.resolve(setDesktopWindowTheme(theme)).catch(() => {
    // Backend desktop proxy is best-effort here; DOM theme remains the source of truth.
  });
}

function applyTheme(theme: Theme) {
  const resolved = resolve(theme);
  if (typeof document !== "undefined") {
    document.documentElement.classList.toggle("dark", resolved === "dark");
  }
  syncNativeTheme(theme);
}

function refreshSnapshotFromEnvironment(): boolean {
  const theme = tryReadStoredTheme();
  if (theme == null) return false;

  const resolvedTheme = resolve(theme);
  if (currentSnapshot.theme === theme && currentSnapshot.resolvedTheme === resolvedTheme) {
    return false;
  }

  currentSnapshot = { theme, resolvedTheme };
  applyTheme(theme);
  return true;
}

// ---------------------------------------------------------------------------
// Store mutations
// ---------------------------------------------------------------------------

function setThemeInternal(next: Theme) {
  if (canUseWindow()) {
    try {
      window.localStorage.setItem(STORAGE_KEY, next);
    } catch {}
  }
  applyTheme(next);
  currentSnapshot = { theme: next, resolvedTheme: resolve(next) };
  emitChange();
}

// ---------------------------------------------------------------------------
// System theme media query listener
// ---------------------------------------------------------------------------

function handleSystemThemeChange() {
  if (currentSnapshot.theme !== "system") return;
  applyTheme("system");
  const newResolved = getSystemTheme();
  if (currentSnapshot.resolvedTheme !== newResolved) {
    currentSnapshot = { ...currentSnapshot, resolvedTheme: newResolved };
    emitChange();
  }
}

function setupSystemThemeListener() {
  if (systemThemeCleanup || !canUseWindow() || typeof window.matchMedia !== "function") {
    return;
  }

  try {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    if (typeof mq.addEventListener === "function") {
      mq.addEventListener("change", handleSystemThemeChange);
      systemThemeCleanup = () => mq.removeEventListener("change", handleSystemThemeChange);
    } else if (typeof mq.addListener === "function") {
      mq.addListener(handleSystemThemeChange);
      systemThemeCleanup = () => mq.removeListener(handleSystemThemeChange);
    }
  } catch {}
}

function handleStorageThemeChange(event: StorageEvent) {
  if (event.key !== null && event.key !== STORAGE_KEY) return;
  if (refreshSnapshotFromEnvironment()) {
    emitChange();
  }
}

function setupStorageThemeListener() {
  if (storageThemeCleanup || !canUseWindow()) return;
  window.addEventListener("storage", handleStorageThemeChange);
  storageThemeCleanup = () => window.removeEventListener("storage", handleStorageThemeChange);
}

// ---------------------------------------------------------------------------
// Tauri native theme change listener (Windows WebView2 fix)
// ---------------------------------------------------------------------------

/**
 * Listen for Tauri native theme change events. This is more reliable than
 * matchMedia on Windows (WebView2).
 */
function handleTauriThemeChange(theme: "light" | "dark") {
  if (currentSnapshot.theme !== "system") return;
  if (currentSnapshot.resolvedTheme !== theme) {
    currentSnapshot = { ...currentSnapshot, resolvedTheme: theme };
    emitChange();
    syncNativeTheme("system");
    if (typeof document !== "undefined") {
      document.documentElement.classList.toggle("dark", theme === "dark");
    }
  }
}

function setupTauriThemeListener() {
  if (!canUseWindow() || tauriThemeCleanup || pendingTauriThemeToken) return;

  const token = {};
  pendingTauriThemeToken = token;
  listenThemeChanged(handleTauriThemeChange)
    .then((unlisten) => {
      if (pendingTauriThemeToken !== token) {
        unlisten();
        return;
      }
      pendingTauriThemeToken = null;
      tauriThemeCleanup = () => {
        unlisten();
      };
    })
    .catch(() => {
      if (pendingTauriThemeToken === token) {
        pendingTauriThemeToken = null;
      }
      // Tauri event listener is best-effort; ignore failures
    });
}

function setupThemeListeners() {
  setupSystemThemeListener();
  setupStorageThemeListener();
  setupTauriThemeListener();
}

function teardownThemeListeners() {
  systemThemeCleanup?.();
  systemThemeCleanup = null;
  storageThemeCleanup?.();
  storageThemeCleanup = null;
  tauriThemeCleanup?.();
  tauriThemeCleanup = null;
  pendingTauriThemeToken = null;
}

if (canUseWindow()) {
  // Apply theme on module load to ensure DOM is in sync
  applyTheme(currentSnapshot.theme);
}

// ---------------------------------------------------------------------------
// Public hook
// ---------------------------------------------------------------------------

export function useTheme() {
  const snapshot = useSyncExternalStore(subscribeTheme, getSnapshot, getServerSnapshot);

  const setTheme = useCallback((next: Theme) => {
    setThemeInternal(next);
  }, []);

  return { theme: snapshot.theme, resolvedTheme: snapshot.resolvedTheme, setTheme } as const;
}
