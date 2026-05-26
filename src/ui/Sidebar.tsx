import type { MouseEvent as ReactMouseEvent } from "react";
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
import { openDesktopUrl } from "../services/desktop/opener";
import { cn } from "../utils/cn";

type NavItem = {
  to: string;
  label: string;
  icon: LucideIcon;
};

type NavSection = {
  id: string;
  label: string;
  items: NavItem[];
};

const NAV_SECTIONS: NavSection[] = [
  {
    id: "main",
    label: "MAIN",
    items: [
      { to: "/", label: "首页", icon: Activity },
      { to: "/providers", label: "供应商", icon: Boxes },
      { to: "/sessions", label: "Session 会话", icon: MessageSquare },
      { to: "/workspaces", label: "工作区", icon: Layers },
      { to: "/prompts", label: "提示词", icon: Pencil },
    ],
  },
  {
    id: "tools",
    label: "TOOLS",
    items: [
      { to: "/mcp", label: "MCP", icon: Command },
      { to: "/skills", label: "Skill", icon: Cpu },
      { to: "/usage", label: "用量", icon: TrendingDown },
      { to: "/logs", label: "请求日志", icon: FileText },
      { to: "/cli-manager", label: "CLI 管理", icon: Wrench },
      { to: "/console", label: "控制台", icon: Terminal },
      { to: "/settings", label: "设置", icon: Settings2 },
    ],
  },
];

const NAV: NavItem[] = NAV_SECTIONS.flatMap((section) => section.items);

export type SidebarProps = {
  className?: string;
};

export function Sidebar({ className }: SidebarProps) {
  const { statusText, portText, hasUpdate, isPortable } = useGatewayStatus();
  const devPreview = useDevPreviewData();

  function handleRepoClick(event: ReactMouseEvent<HTMLAnchorElement>) {
    event.preventDefault();
    event.stopPropagation();
    openDesktopUrl(AIO_REPO_URL).catch(() => {});
  }

  return (
    <aside
      className={cn(
        "sticky top-0 h-screen w-[232px] shrink-0",
        "border-r border-sidebar-border bg-sidebar",
        className
      )}
    >
      <div className="flex h-full flex-col">
        {/* macOS traffic lights safe area (titleBarStyle: overlay) + drag region */}
        <div data-tauri-drag-region className="px-3 pb-4 pt-9">
          <div className="rounded-xl border border-sidebar-border bg-sidebar-panel px-3 py-2.5">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                <a
                  href={AIO_REPO_URL}
                  target="_blank"
                  rel="noopener noreferrer"
                  aria-label="AIO Coding Hub GitHub 仓库"
                  onClick={handleRepoClick}
                  className="text-muted-foreground transition hover:text-foreground"
                >
                  <svg
                    className="h-5 w-5"
                    fill="currentColor"
                    viewBox="0 0 24 24"
                    aria-hidden="true"
                  >
                    <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z" />
                  </svg>
                </a>
                <div className="text-sm font-semibold text-sidebar-foreground">AIO Coding Hub</div>
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
        </div>

        <nav aria-label="Main navigation" className="flex-1 space-y-5 px-3">
          {NAV_SECTIONS.map((section) => {
            const headingId = `sidebar-section-${section.id}`;

            return (
              <section key={section.id} aria-labelledby={headingId} className="space-y-2">
                <h2
                  id={headingId}
                  className="px-3 text-[10px] font-semibold uppercase tracking-[0.18em] text-muted-foreground/70"
                >
                  {section.label}
                </h2>
                <div className="space-y-1 rounded-xl p-1">
                  {section.items.map((item) => (
                    <NavLink
                      key={item.to}
                      to={item.to}
                      className={({ isActive }) =>
                        cn(
                          "group relative flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors",
                          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sidebar-ring/35 focus-visible:ring-offset-2 focus-visible:ring-offset-sidebar",
                          isActive
                            ? "border border-state-selected-border bg-state-selected text-state-selected-foreground"
                            : "border border-transparent text-sidebar-foreground hover:bg-sidebar-accent"
                        )
                      }
                      end={item.to === "/"}
                    >
                      {({ isActive }) => (
                        <>
                          {isActive ? (
                            <span className="absolute left-1 top-1/2 h-4 -translate-y-1/2 rounded-full border-l-2 border-state-selected-foreground" />
                          ) : null}
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
                </div>
              </section>
            );
          })}
        </nav>

        <div className="px-4 py-3">
          <div className="flex items-center gap-2 rounded-xl border border-sidebar-border bg-sidebar-muted px-3 py-2 text-xs text-muted-foreground">
            <span className="h-[5px] w-[5px] rounded-full bg-green shadow-[0_0_6px] shadow-green" />
            <span>
              {statusText} · {portText}
            </span>
          </div>
        </div>
      </div>
    </aside>
  );
}

export { NAV, NAV_SECTIONS };
export type { NavItem, NavSection };
