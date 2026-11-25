import { useEffect, useReducer, useRef, useState } from "react";
import { toast } from "sonner";
import type { AppSettings, WslHostAddressMode } from "../../services/settings/settings";
import { logToConsole } from "../../services/consoleLog";
import { validateWslCustomHostAddress } from "../../services/settings/settingsValidation";
import type { WslConfigureReport } from "../../services/app/wsl";
import { listenDesktopEvent } from "../../services/desktop/event";
import { useAppAboutQuery } from "../../query/appAbout";
import { useSettingsPatchMutation } from "../../query/settings";
import { useWslConfigureClientsMutation, useWslOverviewQuery } from "../../query/wsl";
import { Card } from "../../ui/Card";
import { ConfirmDialog } from "../../ui/ConfirmDialog";
import { Input } from "../../ui/Input";
import { SettingsRow } from "../../ui/SettingsRow";
import { Switch } from "../../ui/Switch";
import { Button } from "../../ui/Button";
import { cn } from "../../utils/cn";
import { Boxes, RefreshCw, Info } from "lucide-react";
import { buildConfigTomlPath } from "../../utils/codexPaths";

export type WslSettingsCardProps = {
  available: boolean;
  saving: boolean;
  settings: AppSettings;
};

type WslAddressDraftState = {
  sourceKey: string;
  hostAddressMode: WslHostAddressMode;
  customHostAddress: string;
};

type WslAddressDraftAction =
  | { type: "resetFromSettings"; state: WslAddressDraftState }
  | { type: "setHostAddressMode"; hostAddressMode: WslHostAddressMode }
  | { type: "setCustomHostAddress"; customHostAddress: string };

type WslStatusRow = NonNullable<
  NonNullable<ReturnType<typeof useWslOverviewQuery>["data"]>["statusRows"]
>[number];

type WslSettingsCardController = {
  aboutOs: string | null;
  checkedOnce: boolean;
  codexHomeMode: AppSettings["codex_home_mode"];
  codexHostConfigPath: string;
  codexWslSyncEnabled: boolean;
  configuring: boolean;
  customHostAddress: string;
  detectionPresent: boolean;
  distros: string[];
  effectiveHost: string;
  hostAddressMode: WslHostAddressMode;
  hostIp: string | null;
  lastReport: WslConfigureReport | null;
  listenModeIsLocalhost: boolean;
  loading: boolean;
  settingsMutating: boolean;
  showListenModeDialog: boolean;
  statusRows: WslStatusRow[] | null;
  switchingListenMode: boolean;
  wslDetected: boolean;
  wslSupported: boolean;
  commitCustomHostAddress: () => Promise<void>;
  commitHostAddressMode: (next: WslHostAddressMode) => Promise<void>;
  commitWslAutoConfig: (value: boolean) => Promise<void>;
  commitWslGemini: (value: boolean) => Promise<void>;
  configureNow: () => Promise<void>;
  confirmSwitchListenMode: () => Promise<void>;
  refreshAll: () => Promise<void>;
  setCustomHostAddress: (customHostAddress: string) => void;
  setShowListenModeDialog: (show: boolean) => void;
};

function createWslAddressDraftState(settings: AppSettings): WslAddressDraftState {
  return {
    sourceKey: `${settings.wsl_host_address_mode}:${settings.wsl_custom_host_address}`,
    hostAddressMode: settings.wsl_host_address_mode,
    customHostAddress: settings.wsl_custom_host_address,
  };
}

function wslAddressDraftReducer(
  state: WslAddressDraftState,
  action: WslAddressDraftAction
): WslAddressDraftState {
  if (action.type === "resetFromSettings") {
    return action.state;
  }
  if (action.type === "setHostAddressMode") {
    return { ...state, hostAddressMode: action.hostAddressMode };
  }
  return { ...state, customHostAddress: action.customHostAddress };
}

function WslSettingsHeader({
  available,
  loading,
  onRefresh,
}: {
  available: boolean;
  loading: boolean;
  onRefresh: () => void;
}) {
  return (
    <div className="mb-4 border-b border-border pb-4 flex items-start justify-between gap-3">
      <div>
        <div className="flex items-center gap-2 text-sm font-semibold text-foreground">
          <Boxes className="h-5 w-5 text-blue-500" />
          WSL 配置
        </div>
      </div>
      <Button
        variant="secondary"
        size="sm"
        onClick={onRefresh}
        disabled={!available || loading}
        className="gap-2"
      >
        <RefreshCw className={cn("h-3.5 w-3.5", loading && "animate-spin")} />
        刷新
      </Button>
    </div>
  );
}

function WslUnavailableMessage({ children }: { children: string }) {
  return (
    <div className="text-sm font-medium text-secondary-foreground dark:text-foreground bg-secondary p-4 rounded-lg">
      {children}
    </div>
  );
}

function WslDetectionSummary({
  checkedOnce,
  detectionPresent,
  distros,
  loading,
  wslDetected,
}: {
  checkedOnce: boolean;
  detectionPresent: boolean;
  distros: string[];
  loading: boolean;
  wslDetected: boolean;
}) {
  return (
    <>
      <SettingsRow label="WSL 状态">
        <div className="flex items-center gap-2">
          <span
            className={cn(
              "inline-block h-2.5 w-2.5 rounded-full",
              wslDetected ? "bg-emerald-500" : checkedOnce ? "bg-muted" : "bg-muted"
            )}
          />
          <span className="text-sm text-secondary-foreground">
            {!checkedOnce
              ? loading
                ? "检测中..."
                : "等待检测"
              : wslDetected
                ? "已检测到 WSL"
                : "未检测到 WSL"}
          </span>
          {checkedOnce && detectionPresent ? (
            <span className="text-xs text-muted-foreground">({distros.length} 个发行版)</span>
          ) : null}
        </div>
      </SettingsRow>

      {wslDetected && distros.length > 0 ? (
        <SettingsRow label="发行版">
          <div className="flex flex-wrap gap-2">
            {distros.map((d) => (
              <span
                key={d}
                className="rounded-full bg-secondary px-2.5 py-1 text-xs text-secondary-foreground border border-border/60 dark:border-border"
              >
                {d}
              </span>
            ))}
          </div>
        </SettingsRow>
      ) : null}
    </>
  );
}

function WslStatusCell({
  auth,
  cliKey,
  mcp,
  prompt,
}: {
  auth: boolean;
  cliKey: string;
  mcp: boolean;
  prompt: boolean;
}) {
  return (
    <td
      className="px-3 py-2"
      aria-label={`${cliKey} Auth ${auth ? "可用" : "不可用"}，MCP ${
        mcp ? "可用" : "不可用"
      }，Prompt ${prompt ? "可用" : "不可用"}`}
    >
      <div
        className="flex items-center justify-center gap-1.5"
        title={`Auth: ${auth ? "yes" : "no"}, MCP: ${mcp ? "yes" : "no"}, Prompt: ${
          prompt ? "yes" : "no"
        }`}
      >
        <span
          aria-hidden="true"
          className={cn(
            "inline-block h-2 w-2 rounded-full",
            auth ? "bg-emerald-500" : "bg-muted dark:bg-secondary"
          )}
        />
        <span
          aria-hidden="true"
          className={cn(
            "inline-block h-2 w-2 rounded-full",
            mcp ? "bg-blue-500" : "bg-muted dark:bg-secondary"
          )}
        />
        <span
          aria-hidden="true"
          className={cn(
            "inline-block h-2 w-2 rounded-full",
            prompt ? "bg-violet-500" : "bg-muted dark:bg-secondary"
          )}
        />
      </div>
    </td>
  );
}

function WslStatusTable({ statusRows }: { statusRows: WslStatusRow[] | null }) {
  if (!statusRows || statusRows.length === 0) return null;

  return (
    <div className="mt-3">
      <div className="mb-2 text-sm font-semibold text-foreground">配置状态</div>
      <div className="rounded-lg border border-border overflow-hidden">
        <table className="w-full text-sm">
          <thead>
            <tr className="bg-secondary text-muted-foreground">
              <th className="text-left px-3 py-2 font-medium">发行版</th>
              <th className="text-center px-3 py-2 font-medium">Claude Code</th>
              <th className="text-center px-3 py-2 font-medium">Codex</th>
              <th className="text-center px-3 py-2 font-medium">Gemini</th>
            </tr>
          </thead>
          <tbody>
            {statusRows.map((row) => (
              <tr key={row.distro} className="border-t border-border">
                <td className="px-3 py-2 text-secondary-foreground font-mono text-xs">
                  {row.distro}
                </td>
                <WslStatusCell
                  cliKey="claude"
                  auth={row.claude}
                  mcp={row.claude_mcp ?? false}
                  prompt={row.claude_prompt ?? false}
                />
                <WslStatusCell
                  cliKey="codex"
                  auth={row.codex}
                  mcp={row.codex_mcp ?? false}
                  prompt={row.codex_prompt ?? false}
                />
                <WslStatusCell
                  cliKey="gemini"
                  auth={row.gemini}
                  mcp={row.gemini_mcp ?? false}
                  prompt={row.gemini_prompt ?? false}
                />
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className="mt-1.5 flex items-center gap-3 text-[11px] text-muted-foreground px-1">
        <span className="flex items-center gap-1">
          <span className="inline-block h-2 w-2 rounded-full bg-emerald-500" /> Auth
        </span>
        <span className="flex items-center gap-1">
          <span className="inline-block h-2 w-2 rounded-full bg-blue-500" /> MCP
        </span>
        <span className="flex items-center gap-1">
          <span className="inline-block h-2 w-2 rounded-full bg-violet-500" /> Prompt
        </span>
        <span className="flex items-center gap-1">
          <span className="inline-block h-2 w-2 rounded-full bg-muted dark:bg-secondary" /> 未配置
        </span>
      </div>
    </div>
  );
}

function WslAutoConfigSection({
  disabled,
  listenModeIsLocalhost,
  settings,
  onCommitWslAutoConfig,
}: {
  disabled: boolean;
  listenModeIsLocalhost: boolean;
  settings: AppSettings;
  onCommitWslAutoConfig: (value: boolean) => void;
}) {
  return (
    <>
      <div className="mt-3">
        <SettingsRow label="自动同步配置">
          <Switch
            checked={settings.wsl_auto_config}
            onCheckedChange={onCommitWslAutoConfig}
            disabled={disabled}
          />
        </SettingsRow>
      </div>

      <div className="mt-2 space-y-2">
        <div className="flex items-start gap-2 text-xs text-muted-foreground">
          <Info className="h-3.5 w-3.5 mt-0.5 shrink-0" />
          <span>
            {settings.wsl_auto_config
              ? "已启用：应用启动时自动检测并配置 WSL 环境，修改相关设置时自动同步。"
              : '未启用：WSL 不会在启动时自动配置，可使用下方"立即配置"按钮手动执行。'}
          </span>
        </div>
        {listenModeIsLocalhost && settings.wsl_auto_config ? (
          <div className="flex items-start gap-2 text-xs text-amber-600 dark:text-amber-400">
            <Info className="h-3.5 w-3.5 mt-0.5 shrink-0" />
            <span>当前监听模式为"仅本地"，WSL 无法访问网关。启动时会提示切换监听模式。</span>
          </div>
        ) : null}
      </div>
    </>
  );
}

function WslCodexSyncTarget({
  codexHomeMode,
  codexHostConfigPath,
  codexWslSyncEnabled,
}: {
  codexHomeMode: AppSettings["codex_home_mode"];
  codexHostConfigPath: string;
  codexWslSyncEnabled: boolean;
}) {
  return (
    <div className="mt-3 rounded-lg border border-border/70 bg-secondary/70 p-3 dark:border-border dark:bg-secondary/40">
      <div className="text-sm font-medium text-secondary-foreground">WSL 中的 Codex 同步目标</div>
      <div className="mt-2 font-mono text-xs text-secondary-foreground break-all">
        $CODEX_HOME/config.toml
      </div>
      <div className="mt-1 text-[11px] leading-relaxed text-muted-foreground">
        {codexWslSyncEnabled
          ? "已纳入 WSL 自动同步。同步时，每个 distro 都会在自己的环境里独立解析 $CODEX_HOME/config.toml；如果没有设置，则回退到 ~/.codex/config.toml。"
          : "当前未启用 Codex 的 WSL 自动同步。若后续启用，目标仍然是每个 distro 内独立解析出的 $CODEX_HOME/config.toml（未设置时回退到 ~/.codex/config.toml）。"}
      </div>
      <div className="mt-1 text-[11px] leading-relaxed text-muted-foreground">
        仅 Windows 本机当前
        {codexHomeMode === "custom"
          ? "使用自定义位置"
          : codexHomeMode === "follow_codex_home"
            ? "跟随 Windows 侧 $CODEX_HOME"
            : "固定使用 Windows 当前用户目录"}
        ：<span className="ml-1 font-mono break-all">{codexHostConfigPath}</span>
        。这只影响 Windows 本机，不会覆盖 WSL 内的目标路径。
      </div>
    </div>
  );
}

function WslGeminiTarget({
  disabled,
  enabled,
  onCommit,
}: {
  disabled: boolean;
  enabled: boolean;
  onCommit: (value: boolean) => void;
}) {
  return (
    <div className="mt-3">
      <SettingsRow label="WSL 中的 Gemini（Antigravity CLI）">
        <Switch
          aria-label="WSL 中的 Gemini（Antigravity CLI）"
          checked={enabled}
          onCheckedChange={onCommit}
          disabled={disabled}
        />
      </SettingsRow>
      <div className="mt-2 flex items-start gap-2 text-xs text-muted-foreground">
        <Info className="h-3.5 w-3.5 mt-0.5 shrink-0" />
        <span>
          {enabled
            ? "已开启：配置 WSL 时，会把 Gemini 的网关配置（~/.gemini/config/config.json 中的 gateway 字段）以及 MCP、提示词和 Skills 同步到每个 distro 的 Antigravity CLI（agy）。修改前请退出 WSL 中正在运行的 agy，新开的会话才生效；开启期间无法用 Google 账号登录，关闭后恢复原有配置。还需在本机开启 Gemini 代理，网关才会接收这些请求。"
            : "未开启：不会修改 WSL 内的 Antigravity 配置，也不同步 Gemini 的 MCP、提示词和 Skills。从旧版升级后需要在这里重新开启。"}
        </span>
      </div>
    </div>
  );
}

function WslConfigureActions({
  configuring,
  saving,
  statusRows,
  onConfigureNow,
}: {
  configuring: boolean;
  saving: boolean;
  statusRows: WslStatusRow[] | null;
  onConfigureNow: () => void;
}) {
  return (
    <>
      <div className="mt-3 flex items-center justify-between gap-3">
        <div className="text-xs text-muted-foreground">
          {statusRows ? (
            <span>
              已检测到至少一个 CLI 已配置：
              {statusRows.filter((r) => r.claude || r.codex || r.gemini).length}/{statusRows.length}{" "}
              个 distro
            </span>
          ) : null}
        </div>
        <Button onClick={onConfigureNow} disabled={configuring || saving} className="gap-2">
          <RefreshCw className={cn("h-4 w-4", configuring && "animate-spin")} />
          立即配置
        </Button>
      </div>

      <div className="mt-2 flex items-start gap-2 text-xs text-muted-foreground">
        <Info className="h-3.5 w-3.5 mt-0.5 shrink-0" />
        <span>
          同步时会自动将 MCP 服务器配置和提示词模板同步到 WSL。stdio 类型 MCP
          的命令路径会自动尝试转换（去除 .cmd/.bat 扩展名，Windows 绝对路径取文件名），但不保证 100%
          正确。
        </span>
      </div>
    </>
  );
}

function WslAdvancedAddressOptions({
  customHostAddress,
  disabled,
  effectiveHost,
  hostAddressMode,
  hostIp,
  onCommitCustomHostAddress,
  onCommitHostAddressMode,
  onCustomHostAddressChange,
}: {
  customHostAddress: string;
  disabled: boolean;
  effectiveHost: string;
  hostAddressMode: WslHostAddressMode;
  hostIp: string | null;
  onCommitCustomHostAddress: () => void;
  onCommitHostAddressMode: (next: WslHostAddressMode) => void;
  onCustomHostAddressChange: (value: string) => void;
}) {
  return (
    <details className="mt-3 rounded-lg border border-border bg-secondary/60 dark:bg-secondary/40">
      <summary className="cursor-pointer select-none px-3 py-2 text-sm font-medium text-secondary-foreground">
        高级选项（地址兜底）
      </summary>
      <div className="px-3 pb-3 space-y-2">
        <div className="text-xs text-muted-foreground">
          当自动检测到的宿主机地址不可用（WSL 无法访问网关）时，可手动指定一个可用的
          host/IP；修改后通常需要重启应用/网关后生效。
        </div>

        <SettingsRow label="生效宿主机地址">
          <div className="font-mono text-xs text-secondary-foreground bg-white/60 dark:bg-card/20 px-2 py-1 rounded border border-border/60 dark:border-border break-all">
            {effectiveHost}
          </div>
        </SettingsRow>

        <SettingsRow label="自动检测地址">
          <div className="font-mono text-xs text-secondary-foreground bg-white/60 dark:bg-card/20 px-2 py-1 rounded border border-border/60 dark:border-border break-all">
            {hostIp ?? "（未检测到）"}
          </div>
        </SettingsRow>

        <SettingsRow label="使用自定义地址">
          <Switch
            checked={hostAddressMode === "custom"}
            onCheckedChange={(checked) => onCommitHostAddressMode(checked ? "custom" : "auto")}
            disabled={disabled}
          />
        </SettingsRow>

        {hostAddressMode === "custom" ? (
          <SettingsRow label="自定义地址">
            <Input
              value={customHostAddress}
              placeholder={hostIp ?? "172.20.0.1"}
              onChange={(e) => onCustomHostAddressChange(e.currentTarget.value)}
              onBlur={onCommitCustomHostAddress}
              disabled={disabled}
              className="font-mono"
            />
          </SettingsRow>
        ) : null}
      </div>
    </details>
  );
}

function WslConfigureReportBanner({ report }: { report: WslConfigureReport | null }) {
  if (!report) return null;

  return (
    <div
      className={cn(
        "mt-3 rounded-lg p-3 text-sm border",
        report.ok
          ? "bg-emerald-50 text-emerald-800 border-emerald-100 dark:bg-emerald-900/30 dark:text-emerald-400 dark:border-emerald-800"
          : "bg-rose-50 text-rose-800 border-rose-100 dark:bg-rose-900/30 dark:text-rose-400 dark:border-rose-800"
      )}
    >
      {report.message}
    </div>
  );
}

function useWslSettingsCardController({
  available,
  saving,
  settings,
}: WslSettingsCardProps): WslSettingsCardController {
  const aboutQuery = useAppAboutQuery({ enabled: available });
  const aboutOs = aboutQuery.data?.os ?? null;

  const wslSupported = aboutOs === "windows";

  const settingsPatchMutation = useSettingsPatchMutation();
  const settingsMutating = settingsPatchMutation.isPending;

  const wslOverviewQuery = useWslOverviewQuery({
    enabled: available && wslSupported,
  });
  const refetchWslOverviewRef = useRef(wslOverviewQuery.refetch);
  refetchWslOverviewRef.current = wslOverviewQuery.refetch;
  const wslConfigureMutation = useWslConfigureClientsMutation();

  const detection = wslOverviewQuery.data?.detection ?? null;
  const hostIp = wslOverviewQuery.data?.hostIp ?? null;
  const statusRows = wslOverviewQuery.data?.statusRows ?? null;

  const checkedOnce = wslOverviewQuery.isFetched;
  const loading = wslOverviewQuery.isFetching;
  const configuring = wslConfigureMutation.isPending;
  const codexWslSyncEnabled = settings.wsl_target_cli?.codex ?? false;
  const codexHomeMode = settings.codex_home_mode;
  const codexHomeOverride = settings.codex_home_override?.trim() ?? "";
  const codexHostConfigPath =
    codexHomeMode === "custom" && codexHomeOverride
      ? buildConfigTomlPath(codexHomeOverride)
      : codexHomeMode === "follow_codex_home"
        ? "跟随 Windows 侧 $CODEX_HOME（未设置时回退到当前用户 ~/.codex/config.toml）"
        : "固定使用 Windows 当前用户目录 ~/.codex/config.toml";

  const [lastReport, setLastReport] = useState<WslConfigureReport | null>(null);
  const [showListenModeDialog, setShowListenModeDialog] = useState(false);
  const [switchingListenMode, setSwitchingListenMode] = useState(false);

  const wslDetected = Boolean(detection?.detected);
  const distros = detection?.distros ?? [];

  const nextAddressDraftState = createWslAddressDraftState(settings);
  const [addressDraftState, dispatchAddressDraft] = useReducer(
    wslAddressDraftReducer,
    nextAddressDraftState
  );
  const effectiveAddressDraftState =
    addressDraftState.sourceKey === nextAddressDraftState.sourceKey
      ? addressDraftState
      : nextAddressDraftState;
  if (addressDraftState.sourceKey !== nextAddressDraftState.sourceKey) {
    dispatchAddressDraft({ type: "resetFromSettings", state: nextAddressDraftState });
  }
  const { hostAddressMode, customHostAddress } = effectiveAddressDraftState;

  function setHostAddressMode(hostAddressMode: WslHostAddressMode) {
    dispatchAddressDraft({ type: "setHostAddressMode", hostAddressMode });
  }

  function setCustomHostAddress(customHostAddress: string) {
    dispatchAddressDraft({ type: "setCustomHostAddress", customHostAddress });
  }

  // 监听后端启动时自动配置结果事件 + 监听模式切换提示
  useEffect(() => {
    if (!available || !wslSupported) return;

    let cancelled = false;
    const cleanupFns: (() => void)[] = [];

    void Promise.allSettled([
      listenDesktopEvent<WslConfigureReport>("wsl:auto_config_result", (payload) => {
        setLastReport(payload);
        void refetchWslOverviewRef.current();
      }),
      listenDesktopEvent("wsl:localhost_switch_prompt", () => {
        setShowListenModeDialog(true);
      }),
    ]).then(([autoConfigResult, localhostSwitchPromptResult]) => {
      const unlistenAutoConfigResult =
        autoConfigResult.status === "fulfilled" ? autoConfigResult.value : null;
      const unlistenLocalhostSwitchPrompt =
        localhostSwitchPromptResult.status === "fulfilled"
          ? localhostSwitchPromptResult.value
          : null;

      if (cancelled) {
        unlistenAutoConfigResult?.();
        unlistenLocalhostSwitchPrompt?.();
        return;
      }

      if (
        autoConfigResult.status === "rejected" ||
        localhostSwitchPromptResult.status === "rejected"
      ) {
        unlistenAutoConfigResult?.();
        unlistenLocalhostSwitchPrompt?.();
        logToConsole("error", "初始化 WSL 事件监听失败", {
          autoConfigError:
            autoConfigResult.status === "rejected" ? String(autoConfigResult.reason) : null,
          localhostPromptError:
            localhostSwitchPromptResult.status === "rejected"
              ? String(localhostSwitchPromptResult.reason)
              : null,
        });
        return;
      }

      if (unlistenAutoConfigResult) cleanupFns.push(unlistenAutoConfigResult);
      if (unlistenLocalhostSwitchPrompt) cleanupFns.push(unlistenLocalhostSwitchPrompt);
    });

    return () => {
      cancelled = true;
      cleanupFns.forEach((fn) => fn());
    };
  }, [available, wslSupported]);

  async function refreshAll() {
    if (!available) return;
    setLastReport(null);

    try {
      await wslOverviewQuery.refetch();
    } catch (err) {
      logToConsole("error", "刷新 WSL 状态失败", { error: String(err) });
      toast("刷新 WSL 状态失败：请稍后重试");
    }
  }

  async function configureNow() {
    if (!available) return;
    if (configuring) return;
    if (!wslSupported) {
      toast("仅 Windows 支持 WSL 配置");
      return;
    }
    if (!wslDetected) {
      toast("未检测到 WSL");
      return;
    }

    setLastReport(null);
    try {
      const report = await wslConfigureMutation.mutateAsync();
      if (!report) {
        return;
      }
      setLastReport(report);
      logToConsole("info", "WSL 一键配置", report);
      toast(report.message || (report.ok ? "配置成功" : "配置失败"));
      await refreshAll();
    } catch (err) {
      logToConsole("error", "WSL 一键配置失败", { error: String(err) });
      toast("WSL 一键配置失败：请查看控制台日志");
    }
  }

  async function commitHostAddressMode(next: WslHostAddressMode) {
    if (!available) return;
    if (saving || settingsMutating) return;

    setHostAddressMode(next);

    try {
      const updated = await settingsPatchMutation.mutateAsync({
        wsl_host_address_mode: next,
      });
      if (!updated) {
        setHostAddressMode(settings.wsl_host_address_mode);
        return;
      }
      toast("已保存");
    } catch (err) {
      logToConsole("error", "更新 WSL 宿主机地址模式失败", { error: String(err), next });
      toast("更新失败：请稍后重试");
      setHostAddressMode(settings.wsl_host_address_mode);
    }
  }

  async function commitCustomHostAddress() {
    if (!available) return;
    if (saving || settingsMutating) return;
    if (hostAddressMode !== "custom") return;

    const trimmed = customHostAddress.trim();
    const current = settings.wsl_custom_host_address.trim();
    if (trimmed === current) return;

    const err = validateWslCustomHostAddress(trimmed);
    if (err) {
      toast(err);
      setCustomHostAddress(settings.wsl_custom_host_address);
      return;
    }

    try {
      const updated = await settingsPatchMutation.mutateAsync({
        wsl_host_address_mode: "custom",
        wsl_custom_host_address: trimmed,
      });
      if (!updated) {
        setCustomHostAddress(settings.wsl_custom_host_address);
        return;
      }
      toast("已保存");
    } catch (err) {
      logToConsole("error", "更新 WSL 宿主机地址失败", {
        error: String(err),
        address: trimmed,
      });
      toast("更新失败：请稍后重试");
      setCustomHostAddress(settings.wsl_custom_host_address);
    }
  }

  async function confirmSwitchListenMode() {
    if (!available) return;
    if (saving || settingsMutating || switchingListenMode) return;
    setSwitchingListenMode(true);
    try {
      const updated = await settingsPatchMutation.mutateAsync({
        gateway_listen_mode: "wsl_auto",
      });
      if (updated) {
        toast('已切换到"WSL 自动检测"模式');
      }
    } catch (err) {
      logToConsole("error", "切换监听模式失败", { error: String(err) });
      toast("切换监听模式失败：请稍后重试");
    } finally {
      setSwitchingListenMode(false);
      setShowListenModeDialog(false);
    }
  }

  async function commitWslGemini(value: boolean) {
    if (!available) return;
    if (saving || settingsMutating) return;
    try {
      const updated = await settingsPatchMutation.mutateAsync({
        wsl_target_cli: { ...settings.wsl_target_cli, gemini: value },
      });
      if (!updated) return;
      toast("已保存");
    } catch (err) {
      logToConsole("error", "更新 WSL Gemini 同步设置失败", { error: String(err) });
      toast("更新失败：请稍后重试");
    }
  }

  async function commitWslAutoConfig(value: boolean) {
    if (!available) return;
    if (saving || settingsMutating) return;
    try {
      const updated = await settingsPatchMutation.mutateAsync({
        wsl_auto_config: value,
      });
      if (!updated) return;
      toast("已保存");
    } catch (err) {
      logToConsole("error", "更新 WSL 自动同步设置失败", { error: String(err) });
      toast("更新失败：请稍后重试");
    }
  }

  const listenModeIsLocalhost = settings.gateway_listen_mode === "localhost";
  const effectiveHost =
    hostAddressMode === "custom"
      ? customHostAddress.trim() || "127.0.0.1"
      : (hostIp ?? "127.0.0.1");

  return {
    aboutOs,
    checkedOnce,
    codexHomeMode,
    codexHostConfigPath,
    codexWslSyncEnabled,
    configuring,
    customHostAddress,
    detectionPresent: Boolean(detection),
    distros,
    effectiveHost,
    hostAddressMode,
    hostIp,
    lastReport,
    listenModeIsLocalhost,
    loading,
    settingsMutating,
    showListenModeDialog,
    statusRows,
    switchingListenMode,
    wslDetected,
    wslSupported,
    commitCustomHostAddress,
    commitHostAddressMode,
    commitWslAutoConfig,
    commitWslGemini,
    configureNow,
    confirmSwitchListenMode,
    refreshAll,
    setCustomHostAddress,
    setShowListenModeDialog,
  };
}

export function WslSettingsCard({ available, saving, settings }: WslSettingsCardProps) {
  const controller = useWslSettingsCardController({ available, saving, settings });

  return (
    <Card className="md:col-span-2">
      <WslSettingsHeader
        available={available}
        loading={controller.loading}
        onRefresh={() => void controller.refreshAll()}
      />

      {!available ? (
        <WslUnavailableMessage>数据不可用</WslUnavailableMessage>
      ) : controller.aboutOs && !controller.wslSupported ? (
        <WslUnavailableMessage>仅 Windows 支持 WSL 配置</WslUnavailableMessage>
      ) : (
        <div className="space-y-1">
          <WslDetectionSummary
            checkedOnce={controller.checkedOnce}
            detectionPresent={controller.detectionPresent}
            distros={controller.distros}
            loading={controller.loading}
            wslDetected={controller.wslDetected}
          />
          <WslStatusTable statusRows={controller.statusRows} />
          <WslAutoConfigSection
            disabled={saving || controller.settingsMutating}
            listenModeIsLocalhost={controller.listenModeIsLocalhost}
            settings={settings}
            onCommitWslAutoConfig={(checked) => void controller.commitWslAutoConfig(checked)}
          />
          <WslCodexSyncTarget
            codexHomeMode={controller.codexHomeMode}
            codexHostConfigPath={controller.codexHostConfigPath}
            codexWslSyncEnabled={controller.codexWslSyncEnabled}
          />
          <WslGeminiTarget
            disabled={saving || controller.settingsMutating}
            enabled={settings.wsl_target_cli?.gemini ?? false}
            onCommit={(checked) => void controller.commitWslGemini(checked)}
          />
          <WslConfigureActions
            configuring={controller.configuring}
            saving={saving}
            statusRows={controller.statusRows}
            onConfigureNow={() => void controller.configureNow()}
          />
          <WslAdvancedAddressOptions
            customHostAddress={controller.customHostAddress}
            disabled={saving || controller.settingsMutating}
            effectiveHost={controller.effectiveHost}
            hostAddressMode={controller.hostAddressMode}
            hostIp={controller.hostIp}
            onCommitCustomHostAddress={() => void controller.commitCustomHostAddress()}
            onCommitHostAddressMode={(next) => void controller.commitHostAddressMode(next)}
            onCustomHostAddressChange={controller.setCustomHostAddress}
          />
          <WslConfigureReportBanner report={controller.lastReport} />
        </div>
      )}

      <ConfirmDialog
        open={controller.showListenModeDialog}
        title="检测到 WSL 环境"
        description={'网关监听模式为"仅本地"，WSL 无法访问网关。是否切换到"WSL 自动检测"模式？'}
        onClose={() => controller.setShowListenModeDialog(false)}
        onConfirm={() => void controller.confirmSwitchListenMode()}
        confirmLabel="切换"
        confirmingLabel="切换中..."
        confirming={controller.switchingListenMode}
        disabled={saving || controller.settingsMutating}
      />
    </Card>
  );
}
