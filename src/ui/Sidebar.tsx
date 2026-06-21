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
  Monitor,
  Moon,
  MessageSquare,
  Pencil,
  Puzzle,
  Settings2,
  Sun,
  Terminal,
  TrendingDown,
  Wrench,
} from "lucide-react";
import { CLIS } from "../constants/clis";
import { AIO_REPO_URL } from "../constants/urls";
import { useDevPreviewData } from "../hooks/useDevPreviewData";
import { useGatewayStatus, openReleasesUrl } from "../hooks/useGatewayStatus";
import { useTheme } from "../hooks/useTheme";
import { updateDialogSetOpen } from "../hooks/useUpdateMeta";
import { useCliProxyControls } from "../hooks/useCliProxyControls";
import { openDesktopUrl } from "../services/desktop/opener";
import type { CliKey } from "../services/providers/providers";
import { Button } from "./Button";
import { Dialog } from "./Dialog";
import { Switch } from "./Switch";
import { cn } from "../utils/cn";

type NavItem = {
  to: string;
  label: string;
  icon: LucideIcon;
  theme: string;
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
      { to: "/", label: "首页", icon: Activity, theme: "blue" },
      { to: "/providers", label: "供应商", icon: Boxes, theme: "cyan" },
      { to: "/sessions", label: "Session 会话", icon: MessageSquare, theme: "violet" },
    ],
  },
  {
    id: "tools",
    label: "TOOLS",
    items: [
      { to: "/workspaces", label: "工作区", icon: Layers, theme: "emerald" },
      { to: "/prompts", label: "提示词", icon: Pencil, theme: "amber" },
      { to: "/mcp", label: "MCP", icon: Command, theme: "indigo" },
      { to: "/skills", label: "Skill", icon: Cpu, theme: "pink" },
      { to: "/plugins", label: "插件", icon: Puzzle, theme: "emerald" },
      { to: "/usage", label: "用量", icon: TrendingDown, theme: "orange" },
      { to: "/logs", label: "请求日志", icon: FileText, theme: "slate" },
      { to: "/cli-manager", label: "CLI 管理", icon: Wrench, theme: "sky" },
    ],
  },
  {
    id: "setting",
    label: "SETTING",
    items: [
      { to: "/console", label: "控制台", icon: Terminal, theme: "rose" },
      { to: "/settings", label: "设置", icon: Settings2, theme: "slate" },
    ],
  },
];

const NAV: NavItem[] = NAV_SECTIONS.flatMap((section) => section.items);

const THEME_OPTIONS = [
  { value: "light", label: "Light", icon: Sun },
  { value: "dark", label: "Dark", icon: Moon },
  { value: "system", label: "System", icon: Monitor },
] as const;

const SIDEBAR_CLI_LABELS: Record<CliKey, string> = {
  claude: "Claude",
  codex: "Codex",
  gemini: "Gemini",
};

export type SidebarProps = {
  className?: string;
};

export function Sidebar({ className }: SidebarProps) {
  const {
    gatewayAvailable,
    statusText,
    portText,
    isGatewayRunning,
    isGatewayStopped,
    hasUpdate,
    isPortable,
  } = useGatewayStatus();
  const { theme, setTheme } = useTheme();
  const devPreview = useDevPreviewData();
  const cliProxyState = useCliProxyControls();
  const { pendingCliProxyEnablePrompt } = cliProxyState;
  const gatewayAriaLabel = `网关状态：${statusText}，端口 ${portText}`;
  const repoLinkLabel = hasUpdate
    ? isPortable && !devPreview.enabled
      ? "AIO Coding Hub GitHub：发现新版本，打开下载页"
      : "AIO Coding Hub GitHub：发现新版本，打开更新对话框"
    : "AIO Coding Hub GitHub 仓库";
  const repoLinkTitle = hasUpdate
    ? isPortable && !devPreview.enabled
      ? "发现新版本（portable：打开下载页）"
      : "发现新版本（点击更新）"
    : "AIO Coding Hub GitHub 仓库";

  function handleRepoClick(event: ReactMouseEvent<HTMLAnchorElement>) {
    event.preventDefault();
    event.stopPropagation();
    if (hasUpdate) {
      if (isPortable && !devPreview.enabled) {
        openReleasesUrl().catch(() => {});
        return;
      }
      updateDialogSetOpen(true);
      return;
    }
    openDesktopUrl(AIO_REPO_URL).catch(() => {});
  }

  return (
    <aside
      className={cn(
        "sticky top-0 h-screen w-[248px] shrink-0",
        "border-r border-sidebar-border bg-sidebar",
        className
      )}
    >
      <div className="flex h-full flex-col">
        {/* macOS traffic lights safe area (titleBarStyle: overlay) + drag region */}
        <div data-tauri-drag-region className="px-5 pb-3.5 pt-7">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2.5">
              {/* Premium abstract AIO high-tech SVG Logo */}
              <div className="flex h-6 w-6 shrink-0 items-center justify-center overflow-hidden rounded-lg shadow-sm shadow-primary/10 dark:shadow-primary/30">
                <img src="/logo.jpg" alt="AIO Logo" className="h-full w-full object-cover" />
              </div>
              <div className="flex flex-col">
                <span className="text-[16px] font-extrabold tracking-tight text-sidebar-foreground">
                  AIO Coding Hub
                </span>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <a
                href={AIO_REPO_URL}
                target="_blank"
                rel="noopener noreferrer"
                aria-label={repoLinkLabel}
                title={repoLinkTitle}
                onClick={handleRepoClick}
                className={cn(
                  "relative inline-flex h-6 w-6 items-center justify-center transition",
                  hasUpdate
                    ? "text-success hover:text-success"
                    : "text-muted-foreground/40 hover:text-muted-foreground"
                )}
              >
                {hasUpdate ? (
                  <span
                    aria-hidden="true"
                    className="absolute -top-2 left-1/2 -translate-x-1/2 rounded-full bg-success/15 px-1 text-[7px] font-extrabold leading-normal tracking-wider text-success ring-1 ring-success/30"
                  >
                    NEW
                  </span>
                ) : null}
                <svg className="h-4 w-4" fill="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                  <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0024 12c0-6.63-5.37-12-12-12z" />
                </svg>
              </a>
            </div>
          </div>
        </div>

        <nav
          aria-label="Main navigation"
          className="flex-1 overflow-y-auto min-h-0 space-y-3 px-3 scrollbar-thin scrollbar-overlay"
        >
          {NAV_SECTIONS.map((section) => {
            const headingId = `sidebar-section-${section.id}`;

            return (
              <section key={section.id} aria-labelledby={headingId} className="space-y-1">
                <h2
                  id={headingId}
                  className="px-3 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground/70"
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
                          "group nav-link-item relative flex items-center gap-3 rounded-lg px-3 py-2 font-display text-[13px] font-semibold border border-transparent",
                          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sidebar-ring/35 focus-visible:ring-offset-2 focus-visible:ring-offset-sidebar",
                          isActive
                            ? "sidebar-active-item"
                            : "text-sidebar-foreground hover:bg-sidebar-accent"
                        )
                      }
                      end={item.to === "/"}
                    >
                      {({ isActive }) => (
                        <>
                          <item.icon
                            className={cn(
                              "h-4 w-4 shrink-0 transition-opacity",
                              isActive
                                ? "opacity-100 text-primary-foreground"
                                : "opacity-70 group-hover:opacity-100"
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

        <div className="px-4 pb-3.5 pt-1.5 bg-transparent shrink-0">
          {/* Flat Unified Premium Control Center Card */}
          <div
            className={cn(
              "rounded-xl border p-4 space-y-2.5 transition-all duration-300 backdrop-blur-md",
              "border-sidebar-control-border bg-sidebar-control shadow-sidebar-control hover:shadow-sidebar-control-hover"
            )}
          >
            {/* Flat unified vertical control list */}
            <div className="space-y-2.5">
              {/* Row 1: Gateway Status */}
              <div
                className="flex items-center justify-between gap-2 px-1 py-0.5"
                aria-label={gatewayAriaLabel}
                title={gatewayAriaLabel}
              >
                <div className="flex items-center gap-2 min-w-0">
                  <span
                    className={cn(
                      "h-1.5 w-1.5 shrink-0 rounded-full",
                      isGatewayRunning
                        ? "bg-emerald-500 shadow-status-dot shadow-emerald-500/80"
                        : isGatewayStopped
                          ? "bg-rose-500 shadow-status-dot shadow-rose-500/80"
                          : gatewayAvailable === "checking"
                            ? "bg-amber-400 shadow-status-dot shadow-amber-400/80"
                            : "bg-muted-foreground/50"
                    )}
                  />
                  <span className="font-semibold text-[11px] text-sidebar-foreground/90 truncate tracking-wide">
                    {isGatewayRunning
                      ? "网关已开启"
                      : isGatewayStopped
                        ? "网关已关闭"
                        : gatewayAvailable === "checking"
                          ? "网关检查中"
                          : gatewayAvailable === "unavailable"
                            ? "网关不可用"
                            : `网关${statusText}`}
                  </span>
                </div>
                <span className="font-mono text-[9px] tabular-nums tracking-wider text-muted-foreground/80 bg-sidebar-control-muted px-2 py-0.5 rounded-full border border-sidebar-control-border">
                  Port: {portText}
                </span>
              </div>

              {/* CLI Proxy List - Compact Horizontal Dashboard Cards */}
              {cliProxyState.cliProxyLoading ? (
                <div className="px-1 py-1 text-muted-foreground/70 text-[10px] font-medium italic animate-pulse text-center">
                  代理状态加载中…
                </div>
              ) : cliProxyState.cliProxyAvailable === false ? (
                <div className="px-1 py-1 text-muted-foreground/70 text-[10px] font-medium italic text-center">
                  代理状态不可用
                </div>
              ) : (
                <div className="grid grid-cols-3 gap-2">
                  {CLIS.map((cli) => {
                    const cliKey = cli.key;
                    const isEnabled = cliProxyState.cliProxyEnabled[cliKey];
                    const drifted =
                      isEnabled && cliProxyState.cliProxyAppliedToCurrentGateway[cliKey] === false;

                    return (
                      <div
                        key={cliKey}
                        className={cn(
                          "flex flex-col items-center justify-between rounded-lg p-1.5 border transition-all duration-200",
                          isEnabled
                            ? "bg-emerald-500/5 border-emerald-500/10 text-emerald-600 dark:text-emerald-400"
                            : "bg-sidebar-control-muted/30 border-slate-300/20 text-sidebar-foreground/70 dark:border-slate-500/15"
                        )}
                      >
                        <span className="text-[10px] font-bold tracking-tight truncate max-w-full">
                          {SIDEBAR_CLI_LABELS[cliKey]}
                        </span>
                        <div className="flex items-center gap-1 mt-1">
                          {drifted ? (
                            <button
                              type="button"
                              disabled={cliProxyState.cliProxyToggling[cliKey]}
                              onClick={() =>
                                cliProxyState.requestCliProxyEnabledSwitch(cliKey, true)
                              }
                              className="text-[9px] text-rose-500 font-bold hover:underline"
                              aria-label={`修复 ${SIDEBAR_CLI_LABELS[cliKey]} 代理`}
                              title={`修复 ${SIDEBAR_CLI_LABELS[cliKey]} 代理`}
                            >
                              修复
                            </button>
                          ) : (
                            <span
                              className={cn(
                                "h-1 w-1 shrink-0 rounded-full transition-all duration-300",
                                isEnabled
                                  ? "bg-emerald-500 shadow-status-dot shadow-emerald-500/70"
                                  : "bg-muted-foreground/20"
                              )}
                            />
                          )}
                          <Switch
                            checked={isEnabled}
                            disabled={cliProxyState.cliProxyToggling[cliKey]}
                            onCheckedChange={(next) =>
                              cliProxyState.requestCliProxyEnabledSwitch(cliKey, next)
                            }
                            size="sm"
                            className="border-0"
                            aria-label={`${SIDEBAR_CLI_LABELS[cliKey]} 代理开关`}
                          />
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}

              {/* Row 5: Seamless Integrated Theme Switcher */}
              <div className="flex items-center justify-between gap-2 px-1 py-0.5 pt-2.5 mt-1">
                <div className="flex items-center gap-2 min-w-0">
                  {(() => {
                    const ActiveIcon = theme === "light" ? Sun : theme === "dark" ? Moon : Monitor;
                    return (
                      <ActiveIcon
                        className="h-4 w-4 text-muted-foreground/70 shrink-0"
                        aria-hidden="true"
                      />
                    );
                  })()}
                  <span className="font-semibold text-[11px] text-sidebar-foreground/80 truncate tracking-wide">
                    主题
                  </span>
                </div>

                <div
                  className="flex items-center gap-0.5 rounded-lg bg-sidebar-control-inset p-0.5 border border-sidebar-control-border"
                  aria-label="主题切换"
                >
                  {THEME_OPTIONS.map((option) => {
                    const isActive = theme === option.value;
                    const activeColorClass =
                      option.value === "light"
                        ? "bg-sidebar-option-active text-theme-option-light shadow-sidebar-option border border-sidebar-control-border"
                        : option.value === "dark"
                          ? "bg-sidebar-option-active text-theme-option-dark shadow-sidebar-option border border-sidebar-control-border"
                          : "bg-sidebar-option-active text-theme-option-system shadow-sidebar-option border border-sidebar-control-border";

                    return (
                      <button
                        key={option.value}
                        type="button"
                        className={cn(
                          "p-1 rounded-md transition-all duration-200 active:scale-95",
                          "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-primary/20",
                          isActive
                            ? activeColorClass
                            : "text-muted-foreground hover:text-sidebar-foreground"
                        )}
                        aria-pressed={isActive}
                        aria-label={`切换到 ${option.label} 主题`}
                        title={`切换到 ${option.label} 主题`}
                        onClick={() => setTheme(option.value)}
                      >
                        <option.icon className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
                      </button>
                    );
                  })}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <Dialog
        open={pendingCliProxyEnablePrompt != null}
        onOpenChange={(open) => {
          if (!open) cliProxyState.setPendingCliProxyEnablePrompt(null);
        }}
        title={
          pendingCliProxyEnablePrompt
            ? `检测到 ${SIDEBAR_CLI_LABELS[pendingCliProxyEnablePrompt.cliKey]} 代理相关环境变量冲突`
            : "检测到环境变量冲突"
        }
        description="继续启用可能会被这些环境变量覆盖（不会显示变量值）。是否继续？"
        className="max-w-lg"
      >
        {pendingCliProxyEnablePrompt ? (
          <div className="space-y-4">
            <ul className="space-y-2">
              {pendingCliProxyEnablePrompt.conflicts.map((row) => (
                <li
                  key={`${row.var_name}:${row.source_type}:${row.source_path}`}
                  className="rounded-lg border border-border bg-secondary px-3 py-2"
                >
                  <div className="font-mono text-xs text-foreground">{row.var_name}</div>
                  <div className="mt-1 text-xs text-muted-foreground">{row.source_path}</div>
                </li>
              ))}
            </ul>

            <div className="flex items-center justify-end gap-2">
              <Button
                variant="secondary"
                size="md"
                onClick={() => cliProxyState.setPendingCliProxyEnablePrompt(null)}
              >
                取消
              </Button>
              <Button
                variant="primary"
                size="md"
                onClick={cliProxyState.confirmPendingCliProxyEnable}
              >
                继续启用
              </Button>
            </div>
          </div>
        ) : null}
      </Dialog>
    </aside>
  );
}

export { NAV, NAV_SECTIONS };
export type { NavItem, NavSection };
