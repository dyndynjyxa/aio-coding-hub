import { NavLink } from "react-router-dom";
import type { LucideIcon } from "lucide-react";
import {
  Activity,
  Boxes,
  Command,
  Cpu,
  FileText,
  Layers,
  MessageSquare,
  Pencil,
  Settings2,
  Terminal,
  TrendingDown,
  Wrench,
} from "lucide-react";
import { AIO_REPO_URL } from "../constants/urls";
import { useDevPreviewData } from "../hooks/useDevPreviewData";
import { useGatewayStatus, openReleasesUrl } from "../hooks/useGatewayStatus";
import { updateDialogSetOpen } from "../hooks/useUpdateMeta";
import { cn } from "../utils/cn";

type NavItem = {
  to: string;
  label: string;
  icon: LucideIcon;
};

const NAV: NavItem[] = [
  { to: "/", label: "首页", icon: Activity },
  { to: "/providers", label: "供应商", icon: Boxes },
  { to: "/sessions", label: "Session 会话", icon: MessageSquare },
  { to: "/workspaces", label: "工作区", icon: Layers },
  { to: "/prompts", label: "提示词", icon: Pencil },
  { to: "/mcp", label: "MCP", icon: Command },
  { to: "/skills", label: "Skill", icon: Cpu },
  { to: "/usage", label: "用量", icon: TrendingDown },
  { to: "/console", label: "控制台", icon: Terminal },
  { to: "/logs", label: "代理记录", icon: FileText },
  { to: "/cli-manager", label: "CLI 管理", icon: Wrench },
  { to: "/settings", label: "设置", icon: Settings2 },
];

export type SidebarProps = {
  /** Whether the sidebar is visible (for responsive control) */
  isOpen?: boolean;
  /** Callback when navigation item is clicked (for mobile drawer close) */
  onNavClick?: () => void;
  /** Additional className for the sidebar container */
  className?: string;
};

export function Sidebar({ isOpen = true, onNavClick, className }: SidebarProps) {
  const { statusText, statusTone, portTone, portText, hasUpdate, isPortable } = useGatewayStatus();
  const devPreview = useDevPreviewData();

  function handleNavClick() {
    onNavClick?.();
  }

  return (
    <aside
      className={cn(
        "sticky top-0 h-screen shrink-0",
        "border-r border-slate-200 bg-white/70 backdrop-blur",
        "dark:border-slate-700 dark:bg-slate-900/70",
        // Responsive width: hidden on mobile, full width on desktop
        "w-64",
        // Transition for smooth open/close
        "transition-transform duration-200 ease-in-out",
        // On desktop (lg+), always show
        "lg:translate-x-0",
        // On smaller screens, control via isOpen prop
        !isOpen && "max-lg:-translate-x-full max-lg:absolute max-lg:z-40",
        className
      )}
    >
      <div className="flex h-full flex-col">
        {/* macOS traffic lights safe area (titleBarStyle: overlay) + drag region */}
        <div data-tauri-drag-region className="px-4 pb-5 pt-9">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <a
                href={AIO_REPO_URL}
                target="_blank"
                rel="noopener noreferrer"
                aria-label="AIO Coding Hub GitHub 仓库"
                className="text-slate-500 transition hover:text-slate-900 dark:text-slate-400 dark:hover:text-slate-100"
              >
                <svg className="h-5 w-5" fill="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z" />
                </svg>
              </a>
              <div className="text-sm font-semibold dark:text-slate-100">AIO Coding Hub</div>
            </div>
            {hasUpdate ? (
              <button
                type="button"
                className={cn(
                  "flex items-center gap-1 rounded-lg px-2 py-1 transition",
                  "bg-emerald-50 text-emerald-700 ring-1 ring-emerald-200 hover:bg-emerald-100",
                  "dark:bg-emerald-900/30 dark:text-emerald-400 dark:ring-emerald-700 dark:hover:bg-emerald-900/50"
                )}
                title={
                  isPortable && !devPreview.enabled
                    ? "发现新版本（portable：打开下载页）"
                    : "发现新版本（点击更新）"
                }
                onClick={() => {
                  if (isPortable && !devPreview.enabled) {
                    openReleasesUrl().catch(() => {});
                    return;
                  }
                  updateDialogSetOpen(true);
                }}
              >
                <span className="text-[10px] font-bold leading-none tracking-wide">NEW</span>
              </button>
            ) : null}
          </div>
        </div>

        <nav aria-label="Main navigation" className="flex-1 space-y-1 px-3">
          {NAV.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              className={({ isActive }) =>
                cn(
                  "group flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition",
                  isActive
                    ? "bg-slate-900 text-white shadow-sm dark:bg-slate-100 dark:text-slate-900"
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
                      "h-4 w-4 shrink-0 transition-opacity",
                      isActive ? "opacity-100" : "opacity-70 group-hover:opacity-100"
                    )}
                  />
                  <span className="truncate">{item.label}</span>
                </>
              )}
            </NavLink>
          ))}
        </nav>

        <div className="px-4 py-3 text-xs text-slate-500 dark:text-slate-400">
          <div className="rounded-xl bg-slate-100/90 px-3 py-2.5 dark:bg-slate-800">
            <div className="flex flex-wrap items-center gap-2">
              <span className="text-sm font-medium text-slate-600 dark:text-slate-300">网关</span>
              <span className={cn("rounded-full px-2 py-0.5 text-[12px] font-medium", statusTone)}>
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
        </div>
      </div>
    </aside>
  );
}

// Export NAV items for use in MobileNav
export { NAV };
export type { NavItem };
