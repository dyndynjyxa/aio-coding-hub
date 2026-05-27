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
    ],
  },
  {
    id: "tools",
    label: "TOOLS",
    items: [
      { to: "/workspaces", label: "工作区", icon: Layers },
      { to: "/prompts", label: "提示词", icon: Pencil },
      { to: "/mcp", label: "MCP", icon: Command },
      { to: "/skills", label: "Skill", icon: Cpu },
      { to: "/usage", label: "用量", icon: TrendingDown },
      { to: "/logs", label: "请求日志", icon: FileText },
      { to: "/cli-manager", label: "CLI 管理", icon: Wrench },
    ],
  },
  {
    id: "setting",
    label: "SETTING",
    items: [
      { to: "/console", label: "控制台", icon: Terminal },
      { to: "/settings", label: "设置", icon: Settings2 },
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
  const gatewayDotClass = isGatewayRunning
    ? "bg-emerald-500 shadow-[0_0_6px] shadow-emerald-500/70"
    : isGatewayStopped
      ? "bg-rose-500 shadow-[0_0_6px] shadow-rose-500/70"
      : gatewayAvailable === "checking"
        ? "bg-amber-400 shadow-[0_0_6px] shadow-amber-400/70"
        : "bg-muted-foreground/50";
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
        "sticky top-0 h-screen w-[232px] shrink-0",
        "border-r border-sidebar-border bg-sidebar",
        className
      )}
    >
      <div className="flex h-full flex-col">
        {/* macOS traffic lights safe area (titleBarStyle: overlay) + drag region */}
        <div data-tauri-drag-region className="px-5 pb-5 pt-9">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2.5">
              {/* Premium abstract AIO high-tech SVG Logo */}
              <div className="flex h-7 w-7 shrink-0 items-center justify-center overflow-hidden rounded-lg shadow-sm shadow-primary/10">
                <img src="/logo.jpg" alt="AIO Logo" className="h-full w-full object-cover" />
              </div>
              <div className="flex flex-col">
                <span className="text-[14px] font-extrabold tracking-tight text-sidebar-foreground">
                  AIO Coding Hub
                </span>
                <span className="text-[9px] font-bold tracking-wider text-muted-foreground/60 uppercase -mt-0.5">
                  Desktop Gateway
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
                            ? "border border-primary bg-primary text-primary-foreground shadow-sm shadow-primary/10"
                            : "border border-transparent text-sidebar-foreground hover:bg-sidebar-accent"
                        )
                      }
                      end={item.to === "/"}
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
                </div>
              </section>
            );
          })}
        </nav>

        <div className="space-y-2.5 px-4 py-4 border-t border-sidebar-border/80 dark:border-sidebar-border">
          <div
            className="flex items-center justify-between gap-2 rounded-xl border border-sidebar-border/80 bg-sidebar-muted/50 p-1 text-xs shadow-inner"
            aria-label="主题切换"
          >
            {THEME_OPTIONS.map((option) => (
              <button
                key={option.value}
                type="button"
                className={cn(
                  "flex min-w-0 flex-1 items-center justify-center gap-1.5 rounded-lg px-2 py-1.5 transition",
                  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-sidebar-ring/35 focus-visible:ring-offset-2 focus-visible:ring-offset-sidebar",
                  theme === option.value
                    ? "bg-sidebar-panel text-primary shadow-sm ring-1 ring-sidebar-border/30 dark:ring-sidebar-border/50"
                    : "text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-foreground"
                )}
                aria-pressed={theme === option.value}
                aria-label={`切换到 ${option.label} 主题`}
                title={`切换到 ${option.label} 主题`}
                onClick={() => setTheme(option.value)}
              >
                <option.icon className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
              </button>
            ))}
          </div>

          <div className="space-y-1.5 rounded-xl border border-sidebar-border bg-sidebar-panel p-2.5 text-xs shadow-[0_1px_3px_rgba(15,23,42,0.03)]">
            <div
              className="flex items-center gap-2 rounded-lg px-2 py-1.5 text-sidebar-foreground/75 font-semibold"
              aria-label={gatewayAriaLabel}
              title={gatewayAriaLabel}
            >
              <span className="min-w-0 truncate font-semibold">网关状态</span>
              <span
                className={cn("h-[5px] w-[5px] shrink-0 rounded-full", gatewayDotClass)}
                aria-hidden="true"
              />
              <span className="ml-auto shrink-0 text-right font-mono tabular-nums text-sidebar-foreground/80">
                {portText}
              </span>
            </div>

            {cliProxyState.cliProxyLoading ? (
              <div className="rounded-lg px-2 py-1.5 text-muted-foreground font-medium">
                代理状态加载中…
              </div>
            ) : cliProxyState.cliProxyAvailable === false ? (
              <div className="rounded-lg px-2 py-1.5 text-muted-foreground font-medium">
                代理状态不可用
              </div>
            ) : (
              CLIS.map((cli) => {
                const cliKey = cli.key;
                const drifted =
                  cliProxyState.cliProxyEnabled[cliKey] &&
                  cliProxyState.cliProxyAppliedToCurrentGateway[cliKey] === false;

                return (
                  <div
                    key={cliKey}
                    className="flex items-center gap-2 rounded-lg px-2 py-1.5 text-sidebar-foreground hover:bg-sidebar-accent transition-colors"
                  >
                    <span className="min-w-0 flex-1 truncate font-semibold text-sidebar-foreground/90">
                      {SIDEBAR_CLI_LABELS[cliKey]}
                    </span>
                    {drifted ? (
                      <Button
                        variant="danger"
                        size="sm"
                        className="h-6 px-2 py-0 text-[11px]"
                        disabled={cliProxyState.cliProxyToggling[cliKey]}
                        onClick={() => cliProxyState.requestCliProxyEnabledSwitch(cliKey, true)}
                        aria-label={`修复 ${SIDEBAR_CLI_LABELS[cliKey]} 代理`}
                      >
                        修复
                      </Button>
                    ) : null}
                    <Switch
                      checked={cliProxyState.cliProxyEnabled[cliKey]}
                      disabled={cliProxyState.cliProxyToggling[cliKey]}
                      onCheckedChange={(next) =>
                        cliProxyState.requestCliProxyEnabledSwitch(cliKey, next)
                      }
                      size="sm"
                      aria-label={`${SIDEBAR_CLI_LABELS[cliKey]} 代理开关`}
                    />
                  </div>
                );
              })
            )}
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
