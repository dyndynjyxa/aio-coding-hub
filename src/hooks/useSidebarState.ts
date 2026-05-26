import { useCallback, useEffect, useState } from "react";

export type SidebarState = {
  /** Whether the sidebar is currently open (visible) */
  isOpen: boolean;
  /** Toggle the sidebar open/closed state */
  toggle: () => void;
  /** Open the sidebar */
  open: () => void;
  /** Close the sidebar */
  close: () => void;
};

const SIDEBAR_STORAGE_KEY = "aio-sidebar-open";

function readSidebarOpenFromStorage(): boolean {
  if (typeof window === "undefined") return true;
  try {
    const stored = window.localStorage.getItem(SIDEBAR_STORAGE_KEY);
    return stored !== "false";
  } catch {
    return true;
  }
}

function writeSidebarOpenToStorage(isOpen: boolean) {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(SIDEBAR_STORAGE_KEY, String(isOpen));
  } catch {}
}

/**
 * Hook to manage sidebar visibility state with responsive behavior
 *
 * Behavior:
 * - Desktop GUI: sidebar open state can be toggled and is persisted
 */
export function useSidebarState(): SidebarState {
  // Desktop sidebar open state (persisted)
  const [isOpen, setIsOpen] = useState<boolean>(() => readSidebarOpenFromStorage());

  // Persist desktop sidebar state
  useEffect(() => {
    writeSidebarOpenToStorage(isOpen);
  }, [isOpen]);

  const toggle = useCallback(() => {
    setIsOpen((prev) => !prev);
  }, []);

  const open = useCallback(() => {
    setIsOpen(true);
  }, []);

  const close = useCallback(() => {
    setIsOpen(false);
  }, []);

  return {
    isOpen,
    toggle,
    open,
    close,
  };
}
