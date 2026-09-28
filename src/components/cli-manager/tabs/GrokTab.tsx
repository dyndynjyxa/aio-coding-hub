import type { ReactNode } from "react";
import type { EnvConflict } from "../../../services/cli/envConflicts";
import type {
  GrokApiBackend,
  GrokConfigState,
  GrokProxyPreferences,
  SimpleCliInfo,
} from "../../../services/cli/cliManager";
import { Button } from "../../../ui/Button";
import { Card } from "../../../ui/Card";
import { Input } from "../../../ui/Input";
import { RadioGroup } from "../../../ui/RadioGroup";
import { Switch } from "../../../ui/Switch";
import { cn } from "../../../utils/cn";
import {
  AlertTriangle,
  CheckCircle2,
  ExternalLink,
  FileJson,
  FolderOpen,
  RefreshCw,
  Settings,
  Terminal,
} from "lucide-react";

export type CliManagerAvailability = "checking" | "available" | "unavailable";

export type CliManagerGrokTabProps = {
  grokAvailable: CliManagerAvailability;
  grokLoading: boolean;
  grokInfo: SimpleCliInfo | null;
  grokConfigLoading: boolean;
  grokConfigSaving: boolean;
  grokConfig: GrokConfigState | null;
  grokConfigError: string | null;
  preferencesDraft: GrokProxyPreferences;
  envConflicts: EnvConflict[] | null;
  envConflictsLoading: boolean;
  envConflictsError: string | null;
  refreshGrok: () => Promise<void> | void;
  openGrokConfigDir: () => Promise<void> | void;
  setModelIdDraft: (modelId: string) => void;
  setApiBackendDraft: (apiBackend: GrokApiBackend) => void;
  setContextWindowDraft: (contextWindow: number | null) => void;
  setTelemetryDraft: (telemetry: boolean | null) => void;
  setSupportsBackendSearchDraft: (supportsBackendSearch: boolean | null) => void;
  persistModelId: (modelId: string) => Promise<void> | void;
  persistApiBackend: (apiBackend: GrokApiBackend) => Promise<void> | void;
  persistContextWindow: (contextWindow: number | null) => Promise<void> | void;
  persistTelemetry: (telemetry: boolean | null) => Promise<void> | void;
  persistSupportsBackendSearch: (supportsBackendSearch: boolean | null) => Promise<void> | void;
};

const PREFERENCE_SOURCE_LABELS: Record<GrokConfigState["preference_source"], string> = {
  existing_config: "现有 Grok 配置",
  fallback: "默认偏好",
  aio_settings: "AIO 已保存偏好",
};

function deriveConfigDir(configPath: string | undefined | null): string {
  if (!configPath) return "—";
  const normalized = configPath.trim().replace(/[\\/]+$/, "");
  const separatorIndex = Math.max(normalized.lastIndexOf("/"), normalized.lastIndexOf("\\"));
  if (separatorIndex < 0) return normalized;
  if (separatorIndex === 0) return normalized.slice(0, 1);
  return normalized.slice(0, separatorIndex);
}

function ProfileRow({
  label,
  profile,
  warnWhenConfigured = false,
  managedSlot = false,
}: {
  label: string;
  profile: string | null;
  warnWhenConfigured?: boolean;
  managedSlot?: boolean;
}) {
  const mayBypassGateway = warnWhenConfigured && profile != null && profile !== "aio";

  return (
    <div className="flex min-h-11 flex-wrap items-center justify-between gap-2 py-2.5">
      <span className="text-sm text-secondary-foreground">{label}</span>
      <div className="flex min-w-0 flex-wrap items-center justify-end gap-2">
        <span className="max-w-full break-all font-mono text-xs text-muted-foreground">
          {profile ?? "未显式配置"}
        </span>
        {managedSlot ? (
          <span
            className={
              profile === "aio"
                ? "text-xs font-medium text-emerald-700 dark:text-emerald-400"
                : "text-xs font-medium text-amber-700 dark:text-amber-400"
            }
          >
            {profile === "aio" ? "已接管" : "未接管"}
          </span>
        ) : null}
        {mayBypassGateway ? (
          <span className="inline-flex items-center gap-1 text-xs text-amber-700 dark:text-amber-400">
            <AlertTriangle className="h-3.5 w-3.5" />
            可能绕过网关
          </span>
        ) : null}
      </div>
    </div>
  );
}

function SettingItem({
  label,
  subtitle,
  children,
  className,
}: {
  label: string;
  subtitle: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "flex flex-col gap-2 py-3 sm:flex-row sm:items-start sm:justify-between",
        className
      )}
    >
      <div className="min-w-0">
        <div className="text-sm text-secondary-foreground">{label}</div>
        <div className="mt-1 text-xs text-muted-foreground leading-relaxed">{subtitle}</div>
      </div>
      <div className="flex flex-wrap items-center justify-end gap-2">{children}</div>
    </div>
  );
}

function GrokHeader({
  grokAvailable,
  grokInfo,
  loading,
  saving,
  onRefresh,
}: Pick<CliManagerGrokTabProps, "grokAvailable" | "grokInfo"> & {
  loading: boolean;
  saving?: boolean;
  onRefresh: () => void;
}) {
  return (
    <div className="flex flex-col md:flex-row items-start md:items-center justify-between gap-4">
      <div className="flex items-center gap-4">
        <div className="flex h-14 w-14 items-center justify-center rounded-lg bg-secondary text-secondary-foreground">
          <Terminal className="h-8 w-8" />
        </div>
        <div>
          <h2 className="text-base font-semibold text-foreground">Grok</h2>
          <div className="flex items-center gap-2 mt-1">
            {grokAvailable === "available" && grokInfo?.found ? (
              <span className="inline-flex items-center gap-1.5 rounded-full bg-emerald-50 px-2.5 py-0.5 text-xs font-medium text-emerald-700 ring-1 ring-inset ring-emerald-600/20 dark:bg-emerald-950/30 dark:text-emerald-400">
                <CheckCircle2 className="h-3 w-3" />
                已安装 {grokInfo.version ?? "版本未知"}
              </span>
            ) : grokAvailable === "checking" || loading ? (
              <span className="inline-flex items-center gap-1.5 rounded-full bg-primary/10 px-2.5 py-0.5 text-xs font-medium text-primary ring-1 ring-inset ring-primary/20">
                <RefreshCw className="h-3 w-3 animate-spin" />
                加载中...
              </span>
            ) : (
              <span className="inline-flex items-center gap-1.5 rounded-full bg-secondary px-2.5 py-0.5 text-xs font-medium text-muted-foreground ring-1 ring-inset ring-border">
                未检测到
              </span>
            )}
            {grokInfo?.error ? (
              <span className="text-xs text-destructive">检测失败：{grokInfo.error}</span>
            ) : null}
          </div>
        </div>
      </div>

      <Button
        onClick={onRefresh}
        variant="secondary"
        size="sm"
        disabled={loading || !!saving}
        className="gap-2"
      >
        <RefreshCw className={cn("h-3.5 w-3.5", loading && "animate-spin")} />
        刷新
      </Button>
    </div>
  );
}

function GrokInfoGrid({
  grokConfig,
  grokInfo,
  activeConfigDirSummaryText,
  openGrokConfigDir,
  openDisabled,
}: {
  grokConfig: GrokConfigState;
  grokInfo: SimpleCliInfo | null;
  activeConfigDirSummaryText: string;
  openGrokConfigDir: () => Promise<void> | void;
  openDisabled: boolean;
}) {
  const configDir = deriveConfigDir(grokConfig.config_path);

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-3 mt-2">
      <div className="bg-secondary rounded-lg p-3 border border-border">
        <div className="flex items-center gap-1.5 text-xs text-muted-foreground mb-1.5">
          <FolderOpen className="h-3 w-3" />
          当前 .grok 目录
        </div>
        <div className="flex items-center gap-1.5">
          <div
            className="font-mono text-xs text-secondary-foreground truncate flex-1"
            title={configDir}
          >
            {configDir}
          </div>
          <Button
            onClick={() => void openGrokConfigDir()}
            disabled={openDisabled}
            size="sm"
            variant="ghost"
            className="h-6 w-6 shrink-0 p-0 hover:bg-muted"
            title="打开当前生效目录"
          >
            <ExternalLink className="h-3 w-3" />
          </Button>
        </div>
        {activeConfigDirSummaryText ? (
          <div className="mt-1 text-[11px] text-muted-foreground">{activeConfigDirSummaryText}</div>
        ) : null}
      </div>

      <div className="bg-secondary rounded-lg p-3 border border-border">
        <div className="flex items-center gap-1.5 text-xs text-muted-foreground mb-1.5">
          <FileJson className="h-3 w-3" />
          config.toml
        </div>
        <div
          className="font-mono text-xs text-secondary-foreground truncate"
          title={grokConfig.config_path}
        >
          {grokConfig.config_path}
        </div>
        <div className="mt-1 text-[11px] text-muted-foreground">
          {grokConfig.file_exists ? "已存在" : "不存在（将自动创建）"}
        </div>
      </div>

      <div className="bg-secondary rounded-lg p-3 border border-border">
        <div className="flex items-center gap-1.5 text-xs text-muted-foreground mb-1.5">
          <Terminal className="h-3 w-3" />
          可执行文件
        </div>
        <div
          className="font-mono text-xs text-secondary-foreground truncate"
          title={grokInfo?.executable_path ?? "—"}
        >
          {grokInfo?.executable_path ?? "—"}
        </div>
      </div>

      <div className="bg-secondary rounded-lg p-3 border border-border">
        <div className="flex items-center gap-1.5 text-xs text-muted-foreground mb-1.5">
          <Settings className="h-3 w-3" />
          解析方式
        </div>
        <div
          className="font-mono text-xs text-secondary-foreground truncate"
          title={grokInfo?.resolved_via ?? "—"}
        >
          {grokInfo?.resolved_via ?? "—"}
        </div>
        <div className="mt-1 text-[11px] text-muted-foreground">
          SHELL: {grokInfo?.shell ?? "—"}
        </div>
      </div>
    </div>
  );
}

export function CliManagerGrokTab(props: CliManagerGrokTabProps) {
  const { grokAvailable, grokInfo, grokConfig, grokConfigError, preferencesDraft } = props;
  const existingPolicyFiles = grokConfig?.policy_files.filter((file) => file.exists) ?? [];
  const configUnavailable = grokConfigError != null || grokConfig == null;
  const configControlsDisabled =
    props.grokConfigLoading || props.grokConfigSaving || configUnavailable;
  const openConfigDisabled = grokAvailable !== "available" || configUnavailable;
  const loading = props.grokLoading || props.grokConfigLoading;

  const activeConfigDirSummaryText = grokConfig
    ? PREFERENCE_SOURCE_LABELS[grokConfig.preference_source]
    : "";

  return (
    <div className="space-y-6">
      <Card className="overflow-hidden">
        <div className="border-b border-border">
          <div className="flex flex-col gap-4 p-6">
            <GrokHeader
              grokAvailable={grokAvailable}
              grokInfo={grokInfo}
              loading={loading}
              saving={props.grokConfigSaving}
              onRefresh={() => void props.refreshGrok()}
            />

            {grokConfig ? (
              <GrokInfoGrid
                grokConfig={grokConfig}
                grokInfo={grokInfo}
                activeConfigDirSummaryText={activeConfigDirSummaryText}
                openGrokConfigDir={props.openGrokConfigDir}
                openDisabled={openConfigDisabled}
              />
            ) : null}
          </div>
        </div>
      </Card>

      {grokConfigError ? (
        <div
          role="alert"
          className="flex items-start gap-2 rounded-lg border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive"
        >
          <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
          <span className="min-w-0 break-words">{grokConfigError}</span>
        </div>
      ) : null}

      <div className="rounded-lg border border-border bg-card p-5">
        <h3 className="text-sm font-semibold text-foreground flex items-center gap-2 mb-3">
          <Settings className="h-4 w-4 text-muted-foreground" />
          基础配置
        </h3>

        <div className="divide-y divide-border">
          <SettingItem
            label="模型 ID (model_id)"
            subtitle="设置通过 AIO 网关使用 Grok 时的默认模型。模型 ID 不能为空。"
          >
            <Input
              value={preferencesDraft.model_id}
              onChange={(e) => props.setModelIdDraft(e.currentTarget.value)}
              onBlur={() => void props.persistModelId(preferencesDraft.model_id)}
              placeholder="例如 grok-4.5"
              aria-label="模型 ID (model_id)"
              className="font-mono w-[280px] max-w-full"
              disabled={configControlsDisabled}
              mono
            />
          </SettingItem>

          <SettingItem
            label="API 协议 (api_backend)"
            subtitle="选择与上游的通信协议。Responses 后端推荐用于服务端搜索能力。"
          >
            <RadioGroup
              name="grok-api-backend"
              ariaLabel="API 协议 (api_backend)"
              value={preferencesDraft.api_backend}
              onChange={(value) => {
                const backend = value === "chat_completions" ? "chat_completions" : "responses";
                props.setApiBackendDraft(backend);
                void props.persistApiBackend(backend);
              }}
              options={[
                { value: "responses", label: "Responses" },
                { value: "chat_completions", label: "Chat Completions" },
              ]}
              disabled={configControlsDisabled}
            />
          </SettingItem>

          <SettingItem
            label="context_window（可选）"
            subtitle="覆盖模型上下文窗口上限。留空表示不覆盖，使用 Grok 默认行为。"
          >
            <Input
              type="number"
              value={
                preferencesDraft.context_window != null
                  ? String(preferencesDraft.context_window)
                  : ""
              }
              onChange={(e) => {
                const n = Number(e.currentTarget.value.trim());
                const normalized = Number.isSafeInteger(n) && n > 0 ? n : null;
                props.setContextWindowDraft(normalized);
              }}
              onBlur={() =>
                void props.persistContextWindow(preferencesDraft.context_window ?? null)
              }
              placeholder="例如 500000"
              aria-label="context_window"
              className="font-mono w-[220px] max-w-full"
              disabled={configControlsDisabled}
              mono
            />
          </SettingItem>

          <SettingItem
            label="关闭客户端遥测 (features.telemetry)"
            subtitle="在 ~/.grok/config.toml 的 [features] 下设置 telemetry = false，关闭 Grok CLI 客户端遥测上报。留空表示不写入该项（交由 Grok 默认行为）。"
          >
            <Switch
              checked={preferencesDraft.telemetry === false}
              onCheckedChange={(checked) => {
                const next = checked ? false : null;
                props.setTelemetryDraft(next);
                void props.persistTelemetry(next);
              }}
              disabled={configControlsDisabled}
              aria-label="关闭客户端遥测"
            />
          </SettingItem>

          <SettingItem
            label="服务端搜索 (supports_backend_search)"
            subtitle="控制 AIO 网关模型配置是否声明 supports_backend_search = true（启用服务端 Web Search）。默认启用；关闭时显式写入 false。"
          >
            <Switch
              checked={preferencesDraft.supports_backend_search !== false}
              onCheckedChange={(checked) => {
                const next = checked ? true : false;
                props.setSupportsBackendSearchDraft(next);
                void props.persistSupportsBackendSearch(next);
              }}
              disabled={configControlsDisabled}
              aria-label="服务端搜索"
            />
          </SettingItem>
        </div>

        <div className="mt-3 text-xs text-muted-foreground">
          Web Search 与图像能力将跟随同一网关模型配置。
        </div>
      </div>

      <Card>
        <div className="flex items-center gap-2">
          <Settings className="h-4 w-4 text-muted-foreground" />
          <h3 className="text-sm font-semibold text-foreground">配置诊断</h3>
        </div>

        <div className="mt-4 divide-y divide-border">
          <ProfileRow label="默认模型" profile={grokConfig?.default_profile ?? null} managedSlot />
          <ProfileRow
            label="会话摘要"
            profile={grokConfig?.session_summary_profile ?? null}
            managedSlot
          />
          <ProfileRow
            label="Web Search"
            profile={grokConfig?.web_search_profile ?? null}
            warnWhenConfigured
          />
          <ProfileRow
            label="图像描述"
            profile={grokConfig?.image_description_profile ?? null}
            warnWhenConfigured
          />
        </div>

        <div className="mt-4 rounded-lg border border-border bg-secondary p-3 text-xs text-muted-foreground">
          {props.envConflictsLoading ? (
            "正在检查相关环境变量..."
          ) : props.envConflictsError ? (
            <span className="text-destructive">{props.envConflictsError}</span>
          ) : props.envConflicts && props.envConflicts.length > 0 ? (
            <div className="space-y-2">
              <div>检测到 {props.envConflicts.length} 个相关环境变量</div>
              <ul className="space-y-1">
                {props.envConflicts.map((conflict) => (
                  <li
                    key={`${conflict.var_name}:${conflict.source_type}:${conflict.source_path}`}
                    className="flex min-w-0 flex-wrap justify-between gap-2"
                  >
                    <span className="font-mono text-foreground">{conflict.var_name}</span>
                    <span className="min-w-0 break-all">{conflict.source_path}</span>
                  </li>
                ))}
              </ul>
            </div>
          ) : (
            "未检测到相关环境变量"
          )}
        </div>

        <div className="mt-3 rounded-lg border border-border bg-secondary p-3 text-xs text-muted-foreground">
          {existingPolicyFiles.length === 0 ? (
            "未检测到企业策略文件"
          ) : (
            <div className="space-y-2">
              <div>检测到 {existingPolicyFiles.length} 个企业策略文件</div>
              <ul className="space-y-1">
                {existingPolicyFiles.map((file) => (
                  <li key={`${file.kind}:${file.path}`} className="break-all font-mono">
                    {file.path}
                  </li>
                ))}
              </ul>
            </div>
          )}
        </div>
      </Card>
    </div>
  );
}
