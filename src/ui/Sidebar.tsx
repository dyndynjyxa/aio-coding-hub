import type { MouseEvent as ReactMouseEvent } from "react";
import { NavLink } from "react-router-dom";
import type { LucideIcon } from "lucide-react";
import {
  Activity,
  Boxes,
  Command,
  Cpu,
  FileText,
  ImagePlus,
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
import claudeFavicon from "../assets/brand/claude-favicon.png";
import codexLogo from "../assets/brand/codex-logo.svg";
import geminiLogo from "../assets/brand/gemini-sparkle-aurora.svg";
import grokLogo from "../assets/brand/grok-logo.svg";
import { cliShortLabel, clisWith, type CliKey } from "../constants/clis";
import { AIO_REPO_URL } from "../constants/urls";
import { useDevPreviewData } from "../hooks/useDevPreviewData";
import { useGatewayStatus, openReleasesUrl } from "../hooks/useGatewayStatus";
import { useTheme } from "../hooks/useTheme";
import { updateDialogSetOpen } from "../hooks/useUpdateMeta";
import { useCliProxyControls } from "../hooks/useCliProxyControls";
import { CliProxyConflictDialog } from "../components/cli-proxy/CliProxyConflictDialog";
import { openDesktopUrl } from "../services/desktop/opener";
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
      { to: "/image-gen", label: "生图", icon: ImagePlus, theme: "pink" },
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
const CLI_PROXY_ORDER: readonly CliKey[] = ["claude", "codex", "grok", "gemini"];
const CLI_PROXY_ITEMS = [...clisWith("cliProxy")].sort(
  (left, right) => CLI_PROXY_ORDER.indexOf(left.key) - CLI_PROXY_ORDER.indexOf(right.key)
);
const CLI_PROXY_LOGOS: Record<CliKey, string> = {
  claude: claudeFavicon,
  codex: codexLogo,
  grok: grokLogo,
  gemini: geminiLogo,
};

const THEME_OPTIONS = [
  { value: "light", label: "Light", icon: Sun },
  { value: "dark", label: "Dark", icon: Moon },
  { value: "system", label: "System", icon: Monitor },
] as const;

export type SidebarProps = {
  className?: string;
};

type ThemeValue = (typeof THEME_OPTIONS)[number]["value"];
type CliProxyState = ReturnType<typeof useCliProxyControls>;

function SidebarHeader({
  repoLinkLabel,
  repoLinkTitle,
  hasUpdate,
  handleRepoClick,
}: {
  repoLinkLabel: string;
  repoLinkTitle: string;
  hasUpdate: boolean;
  handleRepoClick: (event: ReactMouseEvent<HTMLAnchorElement>) => void;
}) {
  return (
    <div data-tauri-drag-region className="px-5 pb-3.5 pt-7">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2.5">
          <div className="flex h-6 w-6 shrink-0 items-center justify-center overflow-hidden rounded-lg shadow-sm shadow-primary/10 dark:shadow-primary/30">
            <img src="/logo.jpg" alt="AIO Logo" className="h-full w-full object-cover" />
          </div>
          <div className="flex flex-col">
            <span className="text-[16px] font-extrabold tracking-tight text-sidebar-foreground">
              AIO Coding Hub
            </span>
          </div>
        </div>
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
  );
}

function SidebarNavigation() {
  return (
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
  );
}

function GatewayStatusRow({
  gatewayAriaLabel,
  gatewayAvailable,
  isGatewayRunning,
  isGatewayStopped,
  statusText,
  portText,
}: {
  gatewayAriaLabel: string;
  gatewayAvailable: string;
  isGatewayRunning: boolean;
  isGatewayStopped: boolean;
  statusText: string;
  portText: string;
}) {
  return (
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
  );
}

function CliProxyGrid({ cliProxyState }: { cliProxyState: CliProxyState }) {
  if (cliProxyState.cliProxyLoading) {
    return (
      <div className="px-1 py-1 text-muted-foreground/70 text-[10px] font-medium italic animate-pulse text-center">
        代理状态加载中…
      </div>
    );
  }

  if (cliProxyState.cliProxyAvailable === false) {
    return (
      <div className="px-1 py-1 text-muted-foreground/70 text-[10px] font-medium italic text-center">
        代理状态不可用
      </div>
    );
  }

  return (
    <div className="grid w-full grid-cols-4 gap-1" aria-label="CLI 代理控制">
      {CLI_PROXY_ITEMS.map(({ key: cliKey }) => {
        const isEnabled = !!cliProxyState.cliProxyEnabled[cliKey];
        const drifted =
          isEnabled && cliProxyState.cliProxyAppliedToCurrentGateway[cliKey] === false;
        const toggling = !!cliProxyState.cliProxyToggling[cliKey];
        const label = cliShortLabel(cliKey);

        return (
          <div
            key={cliKey}
            data-cli-key={cliKey}
            className={cn(
              "relative flex min-w-0 flex-col items-center gap-1 rounded-md border px-1 py-1.5 transition-colors",
              isEnabled
                ? "bg-emerald-500/5 border-emerald-500/10 text-emerald-600 dark:text-emerald-400"
                : "border-sidebar-control-border bg-sidebar-control-muted/30 text-sidebar-foreground/70"
            )}
            title={label}
          >
            <img
              src={CLI_PROXY_LOGOS[cliKey]}
              alt=""
              aria-hidden="true"
              className={cn(
                "h-4 w-4 object-contain",
                (cliKey === "codex" || cliKey === "grok") && "dark:invert"
              )}
            />
            <Switch
              checked={isEnabled}
              disabled={toggling}
              onCheckedChange={(next) => cliProxyState.requestCliProxyEnabledSwitch(cliKey, next)}
              size="sm"
              className="border-0"
              aria-label={`${label} 代理开关`}
            />
            {drifted ? (
              <button
                type="button"
                disabled={toggling}
                onClick={() => cliProxyState.requestCliProxyEnabledSwitch(cliKey, true)}
                className="absolute right-0.5 top-0.5 text-[8px] font-bold text-rose-500 hover:underline"
                aria-label={`修复 ${label} 代理`}
                title={`修复 ${label} 代理`}
              >
                修复
              </button>
            ) : null}
          </div>
        );
      })}
    </div>
  );
}

function ThemeSwitcher({
  theme,
  setTheme,
}: {
  theme: ThemeValue;
  setTheme: (theme: ThemeValue) => void;
}) {
  const ActiveIcon = theme === "light" ? Sun : theme === "dark" ? Moon : Monitor;

  return (
    <div className="flex items-center justify-between gap-2 px-1 py-0.5 pt-2.5 mt-1">
      <div className="flex items-center gap-2 min-w-0">
        <ActiveIcon className="h-4 w-4 text-muted-foreground/70 shrink-0" aria-hidden="true" />
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
                isActive ? activeColorClass : "text-muted-foreground hover:text-sidebar-foreground"
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
  );
}

function SidebarControlCenter({
  gatewayAriaLabel,
  gatewayAvailable,
  isGatewayRunning,
  isGatewayStopped,
  statusText,
  portText,
  cliProxyState,
  theme,
  setTheme,
}: {
  gatewayAriaLabel: string;
  gatewayAvailable: string;
  isGatewayRunning: boolean;
  isGatewayStopped: boolean;
  statusText: string;
  portText: string;
  cliProxyState: CliProxyState;
  theme: ThemeValue;
  setTheme: (theme: ThemeValue) => void;
}) {
  return (
    <div className="px-4 pb-3.5 pt-1.5 bg-transparent shrink-0">
      <div
        className={cn(
          "rounded-xl border p-4 space-y-2.5 transition-all duration-300 backdrop-blur-md",
          "border-sidebar-control-border bg-sidebar-control shadow-sidebar-control hover:shadow-sidebar-control-hover"
        )}
      >
        <div className="space-y-2.5">
          <GatewayStatusRow
            gatewayAriaLabel={gatewayAriaLabel}
            gatewayAvailable={gatewayAvailable}
            isGatewayRunning={isGatewayRunning}
            isGatewayStopped={isGatewayStopped}
            statusText={statusText}
            portText={portText}
          />
          <CliProxyGrid cliProxyState={cliProxyState} />
          <ThemeSwitcher theme={theme} setTheme={setTheme} />
        </div>
      </div>
    </div>
  );
}

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
        <SidebarHeader
          repoLinkLabel={repoLinkLabel}
          repoLinkTitle={repoLinkTitle}
          hasUpdate={hasUpdate}
          handleRepoClick={handleRepoClick}
        />

        <SidebarNavigation />

        <SidebarControlCenter
          gatewayAriaLabel={gatewayAriaLabel}
          gatewayAvailable={gatewayAvailable}
          isGatewayRunning={isGatewayRunning}
          isGatewayStopped={isGatewayStopped}
          statusText={statusText}
          portText={portText}
          cliProxyState={cliProxyState}
          theme={theme}
          setTheme={setTheme}
        />
      </div>

      <CliProxyConflictDialog
        prompt={pendingCliProxyEnablePrompt}
        onCancel={() => cliProxyState.setPendingCliProxyEnablePrompt(null)}
        onConfirm={cliProxyState.confirmPendingCliProxyEnable}
      />
    </aside>
  );
}

export { NAV, NAV_SECTIONS };
export type { NavItem, NavSection };
