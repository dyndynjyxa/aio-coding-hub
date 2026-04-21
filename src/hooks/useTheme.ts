import { useCallback, useSyncExternalStore } from "react";
import { setDesktopWindowTheme } from "../services/desktop/window";
import { listenThemeChanged } from "../services/desktop/themeEvent";

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

function readStoredTheme(): Theme {
  if (!canUseWindow()) return "system";

  try {
    return normalizeTheme(window.localStorage.getItem(STORAGE_KEY));
  } catch {
    return "system";
  }
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

function emitChange() {
  for (const l of listeners) l();
}

function subscribe(listener: Listener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
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
// System theme media query listener (singleton, always active)
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

if (canUseWindow() && typeof window.matchMedia === "function") {
  try {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    if (typeof mq.addEventListener === "function") {
      mq.addEventListener("change", handleSystemThemeChange);
    } else if (typeof mq.addListener === "function") {
      mq.addListener(handleSystemThemeChange);
    }
  } catch {}
}

// ---------------------------------------------------------------------------
// Tauri native theme change listener (Windows WebView2 fix)
// ---------------------------------------------------------------------------

/**
 * Listen for Tauri native theme change events.
 * This is more reliable than matchMedia on Windows (WebView2).
 */
function setupTauriThemeListener() {
  listenThemeChanged((theme) => {
    if (currentSnapshot.theme !== "system") return;
    if (currentSnapshot.resolvedTheme !== theme) {
      currentSnapshot = { ...currentSnapshot, resolvedTheme: theme };
      emitChange();
      // Sync DOM class for the new resolved theme
      if (typeof document !== "undefined") {
        document.documentElement.classList.toggle("dark", theme === "dark");
      }
    }
  }).catch(() => {
    // Tauri event listener is best-effort; ignore failures
  });
}

if (canUseWindow()) {
  setupTauriThemeListener();
  // Apply theme on module load to ensure DOM is in sync
  applyTheme(currentSnapshot.theme);
}

// ---------------------------------------------------------------------------
// Public hook
// ---------------------------------------------------------------------------

export function useTheme() {
  const snapshot = useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);

  const setTheme = useCallback((next: Theme) => {
    setThemeInternal(next);
  }, []);

  return { theme: snapshot.theme, resolvedTheme: snapshot.resolvedTheme, setTheme } as const;
}
