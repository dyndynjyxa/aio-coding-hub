import type { ReactNode } from "react";
import type { EnvConflict } from "../../../services/cli/envConflicts";
import type {
  GrokApiBackend,
  GrokConfigState,
  GrokProxyPreferences,
  SimpleCliInfo,
} from "../../../services/cli/cliManager";
import type { ProviderSummary } from "../../../services/providers/providers";
import type { PendingCliProxyEnablePrompt } from "../../../hooks/useCliProxyControls";
import { CliProxyConflictDialog } from "../../cli-proxy/CliProxyConflictDialog";
import { Button } from "../../../ui/Button";
import { Card } from "../../../ui/Card";
import { Input } from "../../../ui/Input";
import { RadioGroup } from "../../../ui/RadioGroup";
import { Switch } from "../../../ui/Switch";
import {
  AlertTriangle,
  CheckCircle2,
  CircleDashed,
  FileCode2,
  FolderOpen,
  RefreshCw,
  Save,
  Server,
  ShieldCheck,
  TerminalSquare,
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
  providers: ProviderSummary[] | null;
  providersLoading: boolean;
  envConflicts: EnvConflict[] | null;
  envConflictsLoading: boolean;
  envConflictsError: string | null;
  cliProxyLoading: boolean;
  cliProxyAvailable: boolean | null;
  cliProxyEnabled: boolean;
  cliProxyAppliedToCurrentGateway: boolean | null;
  cliProxyToggling: boolean;
  pendingCliProxyEnablePrompt: PendingCliProxyEnablePrompt | null;
  refreshGrok: () => Promise<void> | void;
  openGrokConfigDir: () => Promise<void> | void;
  setModelIdDraft: (modelId: string) => void;
  setApiBackendDraft: (apiBackend: GrokApiBackend) => void;
  persistPreferences: () => Promise<void> | void;
  requestCliProxyEnabledSwitch: (next: boolean) => void;
  clearPendingCliProxyEnablePrompt: () => void;
  confirmPendingCliProxyEnable: () => void;
};

const PREFERENCE_SOURCE_LABELS: Record<GrokConfigState["preference_source"], string> = {
  existing_config: "现有 Grok 配置",
  fallback: "默认偏好",
  aio_settings: "AIO 已保存偏好",
};

function InfoTile({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="min-w-0 rounded-lg border border-line-subtle bg-surface-inset p-3">
      <div className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
        {label}
      </div>
      <div className="mt-1 min-w-0 break-all text-xs text-secondary-foreground">{children}</div>
    </div>
  );
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

function GrokHeader({
  grokAvailable,
  grokInfo,
  loading,
  onRefresh,
}: Pick<CliManagerGrokTabProps, "grokAvailable" | "grokInfo"> & {
  loading: boolean;
  onRefresh: () => void;
}) {
  return (
    <div className="flex flex-col gap-4 border-b border-line-subtle p-5 sm:flex-row sm:items-center sm:justify-between md:p-6">
      <div className="flex min-w-0 items-center gap-4">
        <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-lg border border-line bg-surface-inset text-foreground">
          <TerminalSquare className="h-6 w-6" />
        </div>
        <div className="min-w-0">
          <h2 className="text-base font-semibold text-foreground">Grok CLI</h2>
          <div className="mt-1 flex flex-wrap items-center gap-2">
            {grokAvailable === "checking" ? (
              <span className="inline-flex items-center gap-1.5 text-xs text-muted-foreground">
                <CircleDashed className="h-3.5 w-3.5 animate-spin" />
                检测中...
              </span>
            ) : grokAvailable === "available" && grokInfo?.found ? (
              <span className="inline-flex items-center gap-1.5 text-xs font-medium text-emerald-700 dark:text-emerald-400">
                <CheckCircle2 className="h-3.5 w-3.5" />
                已安装 {grokInfo.version ?? "版本未知"}
              </span>
            ) : (
              <span className="text-xs text-muted-foreground">未检测到</span>
            )}
            {grokInfo?.error ? (
              <span className="text-xs text-destructive">检测失败：{grokInfo.error}</span>
            ) : null}
          </div>
        </div>
      </div>

      <Button size="sm" onClick={onRefresh} disabled={loading}>
        <RefreshCw className={loading ? "h-3.5 w-3.5 animate-spin" : "h-3.5 w-3.5"} />
        刷新 Grok 状态
      </Button>
    </div>
  );
}

export function CliManagerGrokTab(props: CliManagerGrokTabProps) {
  const { grokAvailable, grokInfo, grokConfig, grokConfigError, preferencesDraft, providers } =
    props;
  const enabledProviders = providers?.filter((provider) => provider.enabled).length ?? 0;
  const existingPolicyFiles = grokConfig?.policy_files.filter((file) => file.exists) ?? [];
  const configUnavailable = grokConfigError != null || grokConfig == null;
  const configControlsDisabled =
    props.grokConfigLoading || props.grokConfigSaving || configUnavailable;
  const saveDisabled = configControlsDisabled || preferencesDraft.model_id.trim().length === 0;
  const proxyDisabled =
    grokAvailable !== "available" ||
    props.cliProxyAvailable !== true ||
    props.cliProxyLoading ||
    props.cliProxyToggling ||
    configUnavailable;
  const openConfigDisabled = grokAvailable !== "available" || configUnavailable;
  const loading = props.grokLoading || props.grokConfigLoading;

  return (
    <div className="space-y-6 pb-6">
      <Card padding="none">
        <GrokHeader
          grokAvailable={grokAvailable}
          grokInfo={grokInfo}
          loading={loading}
          onRefresh={() => void props.refreshGrok()}
        />

        <div className="grid gap-3 p-5 sm:grid-cols-2 md:p-6 lg:grid-cols-3">
          <InfoTile label="可执行文件">{grokInfo?.executable_path ?? "—"}</InfoTile>
          <InfoTile label="有效配置">
            <div>{grokConfig?.config_path ?? "—"}</div>
            {grokConfig ? (
              <div className="mt-1 text-muted-foreground">
                {grokConfig.file_exists ? "配置文件已存在" : "配置文件尚未创建"}
              </div>
            ) : null}
          </InfoTile>
          <InfoTile label="配置来源">
            {grokConfig ? PREFERENCE_SOURCE_LABELS[grokConfig.preference_source] : "—"}
          </InfoTile>
        </div>

        <div className="flex justify-end border-t border-line-subtle px-5 py-3 md:px-6">
          <Button
            size="sm"
            onClick={() => void props.openGrokConfigDir()}
            disabled={openConfigDisabled}
          >
            <FolderOpen className="h-3.5 w-3.5" />
            打开 Grok 配置目录
          </Button>
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

      <div className="grid gap-6 xl:grid-cols-[minmax(0,1.2fr)_minmax(320px,0.8fr)]">
        <Card>
          <div className="flex items-center gap-2">
            <FileCode2 className="h-4 w-4 text-muted-foreground" />
            <h3 className="text-sm font-semibold text-foreground">网关模型偏好</h3>
          </div>

          <div className="mt-5 space-y-5">
            <label className="block space-y-2">
              <span className="text-sm text-secondary-foreground">模型 ID</span>
              <Input
                aria-label="模型 ID"
                value={preferencesDraft.model_id}
                onChange={(event) => props.setModelIdDraft(event.currentTarget.value)}
                disabled={configControlsDisabled}
                mono
              />
            </label>

            <div className="space-y-2">
              <div className="text-sm text-secondary-foreground">API 协议</div>
              <RadioGroup
                name="grok-api-backend"
                ariaLabel="API 协议"
                value={preferencesDraft.api_backend}
                onChange={(value) =>
                  props.setApiBackendDraft(
                    value === "chat_completions" ? "chat_completions" : "responses"
                  )
                }
                options={[
                  { value: "responses", label: "Responses" },
                  { value: "chat_completions", label: "Chat Completions" },
                ]}
                disabled={configControlsDisabled}
              />
            </div>

            <div className="flex justify-end">
              <Button
                variant="primary"
                onClick={() => void props.persistPreferences()}
                disabled={saveDisabled}
              >
                <Save className="h-4 w-4" />
                保存偏好
              </Button>
            </div>
          </div>
        </Card>

        <Card>
          <div className="flex items-center gap-2">
            <Server className="h-4 w-4 text-muted-foreground" />
            <h3 className="text-sm font-semibold text-foreground">供应商</h3>
          </div>
          <div className="mt-4 text-sm text-secondary-foreground">
            {props.providersLoading
              ? "正在读取供应商..."
              : `${enabledProviders} 个已启用 / ${providers?.length ?? 0} 个供应商`}
          </div>

          <div className="mt-5 flex items-center justify-between gap-3 border-t border-line-subtle pt-4">
            <div className="min-w-0">
              <div className="text-sm text-secondary-foreground">CLI 代理</div>
              <div className="mt-1 text-xs text-muted-foreground">
                {props.cliProxyEnabled
                  ? props.cliProxyAppliedToCurrentGateway === false
                    ? "配置需要修复"
                    : "已启用"
                  : "未启用"}
              </div>
            </div>
            <Switch
              aria-label="Grok CLI 代理"
              checked={props.cliProxyEnabled}
              onCheckedChange={props.requestCliProxyEnabledSwitch}
              disabled={proxyDisabled}
            />
          </div>
        </Card>
      </div>

      <Card>
        <div className="flex items-center gap-2">
          <ShieldCheck className="h-4 w-4 text-muted-foreground" />
          <h3 className="text-sm font-semibold text-foreground">配置诊断</h3>
        </div>

        <div className="mt-4 divide-y divide-line-subtle">
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

        <div className="mt-4 rounded-lg border border-line-subtle bg-surface-inset p-3 text-xs text-muted-foreground">
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

        <div className="mt-3 rounded-lg border border-line-subtle bg-surface-inset p-3 text-xs text-muted-foreground">
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

      <CliProxyConflictDialog
        prompt={props.pendingCliProxyEnablePrompt}
        onCancel={props.clearPendingCliProxyEnablePrompt}
        onConfirm={props.confirmPendingCliProxyEnable}
      />
    </div>
  );
}
