import { useEffect } from "react";
import { createPortal } from "react-dom";
import { NavLink } from "react-router-dom";
import { AIO_REPO_URL } from "../constants/urls";
import { useDevPreviewData } from "../hooks/useDevPreviewData";
import { useGatewayStatus, openReleasesUrl } from "../hooks/useGatewayStatus";
import { updateDialogSetOpen } from "../hooks/useUpdateMeta";
import { cn } from "../utils/cn";
import { NAV } from "./Sidebar";

export type MobileNavProps = {
  /** Whether the drawer is open */
  isOpen: boolean;
  /** Callback to close the drawer */
  onClose: () => void;
};

/**
 * Mobile navigation drawer component
 * Slides in from the left on small screens
 */
export function MobileNav({ isOpen, onClose }: MobileNavProps) {
  const { statusText, statusTone, portTone, portText, hasUpdate, isPortable } = useGatewayStatus();
  const devPreview = useDevPreviewData();

  // Close on escape key
  useEffect(() => {
    if (!isOpen) return;

    function onKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [isOpen, onClose]);

  // Prevent body scroll when drawer is open
  useEffect(() => {
    if (isOpen) {
      document.body.style.overflow = "hidden";
    } else {
      document.body.style.overflow = "";
    }
    return () => {
      document.body.style.overflow = "";
    };
  }, [isOpen]);

  function handleNavClick() {
    onClose();
  }

  if (!isOpen) return null;

  return createPortal(
    <div className="fixed inset-0 z-50 lg:hidden">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/30 backdrop-blur-sm"
        onClick={onClose}
        aria-hidden="true"
      />

      {/* Drawer */}
      <aside
        className={cn(
          "absolute left-0 top-0 h-full w-72 max-w-[85vw]",
          "bg-white shadow-xl dark:bg-slate-900 dark:shadow-slate-950/50",
          "transform transition-transform duration-300 ease-out",
          isOpen ? "translate-x-0" : "-translate-x-full"
        )}
      >
        <div className="flex h-full flex-col">
          {/* Header */}
          <div className="flex items-center justify-between border-b border-slate-200 px-4 pb-4 pt-9 dark:border-slate-700">
            <div className="flex items-center gap-3">
              <div className="text-sm font-semibold dark:text-slate-100">AIO Coding Hub</div>
              {hasUpdate && (
                <button
                  type="button"
                  className={cn(
                    "flex items-center gap-1 rounded-lg px-2 py-1 transition",
                    "bg-emerald-50 text-emerald-700 ring-1 ring-emerald-200 hover:bg-emerald-100 dark:bg-emerald-900/30 dark:text-emerald-400 dark:ring-emerald-700 dark:hover:bg-emerald-900/50"
                  )}
                  title={
                    isPortable && !devPreview.enabled
                      ? "发现新版本（打开下载页）"
                      : "发现新版本（点击更新）"
                  }
                  onClick={() => {
                    if (isPortable && !devPreview.enabled) {
                      openReleasesUrl().catch(() => {});
                      return;
                    }
                    updateDialogSetOpen(true);
                    onClose();
                  }}
                >
                  <span className="text-[10px] font-bold leading-none tracking-wide">NEW</span>
                </button>
              )}
            </div>

            {/* Close button */}
            <button
              type="button"
              className="rounded-lg p-2 text-slate-500 hover:bg-slate-100 hover:text-slate-900 dark:text-slate-400 dark:hover:bg-slate-800 dark:hover:text-slate-100"
              onClick={onClose}
              aria-label="关闭菜单"
            >
              <svg className="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </button>
          </div>

          {/* Navigation */}
          <nav className="flex-1 overflow-y-auto p-3">
            <div className="space-y-1">
              {NAV.map((item) => (
                <NavLink
                  key={item.to}
                  to={item.to}
                  className={({ isActive }) =>
                    cn(
                      "group flex items-center gap-3 rounded-lg px-3 py-3 text-sm transition",
                      isActive
                        ? "bg-slate-900 text-white shadow-sm dark:bg-slate-700"
                        : "text-slate-700 hover:bg-slate-100 dark:text-slate-300 dark:hover:bg-slate-800"
                    )
                  }
                  end={item.to === "/"}
                  onClick={handleNavClick}
                >
                  {({ isActive }) => (
                    <>
                      <item.icon
                        className={cn(
                          "h-4.5 w-4.5 shrink-0 transition-opacity",
                          isActive ? "opacity-100" : "opacity-70 group-hover:opacity-100"
                        )}
                      />
                      <span>{item.label}</span>
                    </>
                  )}
                </NavLink>
              ))}
            </div>
          </nav>

          {/* Footer - Gateway status */}
          <div className="border-t border-slate-200 px-4 py-4 text-xs text-slate-500 dark:border-slate-700 dark:text-slate-400">
            <div className="rounded-xl bg-slate-100/90 px-3 py-2.5 dark:bg-slate-800">
              <div className="flex flex-wrap items-center gap-2">
                <span className="text-sm font-medium text-slate-600 dark:text-slate-300">网关</span>
                <span
                  className={cn("rounded-full px-2 py-0.5 text-[12px] font-medium", statusTone)}
                >
                  {statusText}
                </span>
                <span
                  className={cn(
                    "rounded-full px-2 py-0.5 font-mono text-[12px] font-medium",
                    portTone
                  )}
                >
                  {portText}
                </span>
              </div>
            </div>

            {/* GitHub link */}
            <div className="mt-4 flex justify-center">
              <a
                href={AIO_REPO_URL}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-2 text-slate-500 transition hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100"
              >
                <svg className="h-5 w-5" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z" />
                </svg>
                <span>GitHub</span>
              </a>
            </div>
          </div>
        </div>
      </aside>
    </div>,
    document.body
  );
}

/**
 * Mobile header with hamburger menu button
 * Shows on screens smaller than lg breakpoint
 */
export type MobileHeaderProps = {
  /** Callback when hamburger button is clicked */
  onMenuClick: () => void;
};

export function MobileHeader({ onMenuClick }: MobileHeaderProps) {
  const { isGatewayRunning } = useGatewayStatus();

  return (
    <header className="sticky top-0 z-30 flex items-center justify-between border-b border-slate-200 bg-white/80 px-4 pb-3 pt-9 backdrop-blur dark:border-slate-700 dark:bg-slate-900/80 lg:hidden">
      {/* Hamburger menu button */}
      <button
        type="button"
        className="rounded-lg p-2 text-slate-600 hover:bg-slate-100 hover:text-slate-900 dark:text-slate-400 dark:hover:bg-slate-800 dark:hover:text-slate-100"
        onClick={onMenuClick}
        aria-label="打开菜单"
      >
        <svg className="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M4 6h16M4 12h16M4 18h16"
          />
        </svg>
      </button>

      {/* Title */}
      <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">AIO Coding Hub</div>

      {/* Gateway status indicator */}
      <div
        className={cn(
          "h-2.5 w-2.5 rounded-full",
          isGatewayRunning ? "bg-emerald-500" : "bg-slate-300 dark:bg-slate-600"
        )}
        title={isGatewayRunning ? "网关运行中" : "网关未运行"}
      />
    </header>
  );
}
