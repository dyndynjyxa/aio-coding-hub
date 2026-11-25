import { useState, type ReactNode } from "react";
import type {
  GeminiConfigPatch,
  GeminiConfigState,
  GeminiModelOption,
  SimpleCliInfo,
} from "../../../services/cli/cliManager";
import { cn } from "../../../utils/cn";
import { Button } from "../../../ui/Button";
import { Card } from "../../../ui/Card";
import { Input } from "../../../ui/Input";
import { Select } from "../../../ui/Select";
import { Switch } from "../../../ui/Switch";
import { CliVersionBadge } from "../CliVersionBadge";
import {
  AlertTriangle,
  CheckCircle2,
  Cpu,
  FileJson,
  FolderOpen,
  RefreshCw,
  Settings,
} from "lucide-react";

export type CliManagerAvailability = "checking" | "available" | "unavailable";

export type CliManagerGeminiTabProps = {
  geminiAvailable: CliManagerAvailability;
  geminiLoading: boolean;
  geminiInfo: SimpleCliInfo | null;
  geminiConfigLoading: boolean;
  geminiConfigSaving: boolean;
  geminiConfig: GeminiConfigState | null;
  geminiModels: GeminiModelOption[] | null;
  geminiModelsError: boolean;
  refreshGeminiInfo: () => Promise<void> | void;
  persistGeminiConfig: (patch: GeminiConfigPatch) => Promise<void> | void;
};

type EnumField =
  | "agentMode"
  | "toolPermission"
  | "artifactReviewPolicy"
  | "colorScheme"
  | "altScreenMode"
  | "verbosity"
  | "editorMode";

type BoolField = "enableTerminalSandbox" | "allowNonWorkspaceAccess" | "notifications" | "showTips";

// Values documented for agy's settings.json; "" removes the key (CLI default).
const ENUM_SETTINGS: Array<{
  field: EnumField;
  label: string;
  subtitle: string;
  options: string[];
}> = [
  {
    field: "agentMode",
    label: "执行模式 (agentMode)",
    subtitle:
      "default 每次改文件前确认；accept-edits 自动接受工作区改动；auto 需终端沙箱；plan 先出计划。",
    options: ["default", "accept-edits", "auto", "plan"],
  },
  {
    field: "toolPermission",
    label: "工具权限 (toolPermission)",
    subtitle: "auto 模式之外的命令审批策略，CLI 默认为 request-review。",
    options: ["request-review", "proceed-in-sandbox", "always-proceed", "strict"],
  },
  {
    field: "artifactReviewPolicy",
    label: "Artifact 审阅 (artifactReviewPolicy)",
    subtitle: "Agent 请求审阅计划等产物时是否等待你确认。",
    options: ["asks-for-review", "agent-decides", "always-proceed"],
  },
  {
    field: "colorScheme",
    label: "配色 (colorScheme)",
    subtitle: "terminal 表示沿用终端配色。",
    options: [
      "terminal",
      "light",
      "solarized light",
      "colorblind-friendly light",
      "dark",
      "solarized dark",
      "colorblind-friendly dark",
      "tokyo night",
    ],
  },
  {
    field: "altScreenMode",
    label: "全屏缓冲 (altScreenMode)",
    subtitle: "always 整个界面用备用屏幕；never 始终内联输出。",
    options: ["default", "always", "never"],
  },
  {
    field: "verbosity",
    label: "输出详细程度 (verbosity)",
    subtitle: "high 展示全部工具调用与思考；low 只展示回复。",
    options: ["high", "medium", "low"],
  },
  {
    field: "editorMode",
    label: "输入编辑模式 (editorMode)",
    subtitle: "vim 启用模态编辑。",
    options: ["default", "vim"],
  },
];

const BOOL_SETTINGS: Array<{
  field: BoolField;
  label: string;
  subtitle: string;
  cliDefault: boolean;
}> = [
  {
    field: "enableTerminalSandbox",
    label: "终端沙箱 (enableTerminalSandbox)",
    subtitle: "把 Agent 执行的命令限制在系统沙箱内。",
    cliDefault: false,
  },
  {
    field: "allowNonWorkspaceAccess",
    label: "允许访问工作区外文件 (allowNonWorkspaceAccess)",
    subtitle: "开启后 Agent 可读写工作区目录之外的文件。",
    cliDefault: false,
  },
  {
    field: "notifications",
    label: "通知 (notifications)",
    subtitle: "需要你确认或回合结束时响铃并发桌面通知。",
    cliDefault: false,
  },
  {
    field: "showTips",
    label: "显示提示 (showTips)",
    subtitle: "生成回复时显示使用提示。",
    cliDefault: true,
  },
];

/** Keeps a value already on disk selectable even if this list doesn't know it. */
function withCurrent(options: string[], current: string | null) {
  return current && !options.includes(current) ? [...options, current] : options;
}

function SettingItem({
  label,
  subtitle,
  children,
}: {
  label: string;
  subtitle: string;
  children: ReactNode;
}) {
  return (
    <div className="flex flex-col gap-2 py-3 sm:flex-row sm:items-start sm:justify-between">
      <div className="min-w-0">
        <div className="text-sm text-secondary-foreground">{label}</div>
        <div className="mt-1 text-xs text-muted-foreground leading-relaxed">{subtitle}</div>
      </div>
      <div className="flex flex-wrap items-center justify-end gap-2">{children}</div>
    </div>
  );
}

function InfoTile({
  icon,
  label,
  value,
  detail,
}: {
  icon: ReactNode;
  label: string;
  value: string;
  detail?: ReactNode;
}) {
  return (
    <div className="bg-secondary rounded-lg p-3 border border-border">
      <div className="flex items-center gap-1.5 text-xs text-muted-foreground mb-1.5">
        {icon}
        {label}
      </div>
      <div className="font-mono text-xs text-secondary-foreground truncate" title={value}>
        {value}
      </div>
      {detail}
    </div>
  );
}

function ModelSetting({
  geminiConfig,
  geminiModels,
  geminiModelsError,
  saving,
  persistGeminiConfig,
}: {
  geminiConfig: GeminiConfigState;
  geminiModels: GeminiModelOption[] | null;
  geminiModelsError: boolean;
  saving: boolean;
  persistGeminiConfig: (patch: GeminiConfigPatch) => Promise<void> | void;
}) {
  const current = geminiConfig.model ?? "";
  // Reset the free-text draft whenever the saved value changes.
  const [draft, setDraft] = useState({ source: current, text: current });
  if (draft.source !== current) setDraft({ source: current, text: current });

  const listed = geminiModels != null && geminiModels.length > 0;
  return (
    <SettingItem
      label="默认模型 (model)"
      subtitle={
        listed
          ? "列表来自 agy models；开启网关接管时即网关提供的模型。agy 按显示名称保存模型，不识别模型 ID。"
          : geminiModelsError
            ? "读取模型列表失败，可直接填写模型名称，例如 Gemini 3.8 Flash (Low)；留空恢复 CLI 默认。"
            : "填写模型名称，例如 Gemini 3.8 Flash (Low)；留空恢复 CLI 默认。"
      }
    >
      {listed ? (
        <Select
          aria-label="默认模型"
          value={current}
          onChange={(e) => void persistGeminiConfig({ model: e.currentTarget.value })}
          className="w-[280px] max-w-full"
          disabled={saving}
        >
          <option value="">CLI 默认</option>
          {withCurrent(
            geminiModels.map((m) => m.label),
            current
          ).map((label) => (
            <option key={label} value={label}>
              {label}
            </option>
          ))}
        </Select>
      ) : (
        <Input
          aria-label="默认模型"
          value={draft.text}
          onChange={(e) => setDraft({ source: current, text: e.currentTarget.value })}
          onBlur={() => {
            const next = draft.text.trim();
            if (next !== current) void persistGeminiConfig({ model: next });
          }}
          placeholder="Gemini 3.8 Flash (Low)"
          className="w-[280px] max-w-full"
          disabled={saving}
        />
      )}
    </SettingItem>
  );
}

function ConfigSections({
  geminiConfig,
  geminiModels,
  geminiModelsError,
  saving,
  persistGeminiConfig,
}: {
  geminiConfig: GeminiConfigState;
  geminiModels: GeminiModelOption[] | null;
  geminiModelsError: boolean;
  saving: boolean;
  persistGeminiConfig: (patch: GeminiConfigPatch) => Promise<void> | void;
}) {
  return (
    <div className="rounded-lg border border-border bg-white dark:bg-secondary p-5">
      <h3 className="text-sm font-semibold text-foreground flex items-center gap-2 mb-3">
        <Settings className="h-4 w-4 text-muted-foreground" />
        settings.json
      </h3>
      <div className="divide-y divide-border">
        <ModelSetting
          geminiConfig={geminiConfig}
          geminiModels={geminiModels}
          geminiModelsError={geminiModelsError}
          saving={saving}
          persistGeminiConfig={persistGeminiConfig}
        />
        {ENUM_SETTINGS.map(({ field, label, subtitle, options }) => {
          const current = geminiConfig[field];
          return (
            <SettingItem key={field} label={label} subtitle={subtitle}>
              <Select
                aria-label={field}
                value={current ?? ""}
                onChange={(e) => void persistGeminiConfig({ [field]: e.currentTarget.value })}
                className="w-[240px] max-w-full font-mono"
                disabled={saving}
              >
                <option value="">CLI 默认</option>
                {withCurrent(options, current).map((option) => (
                  <option key={option} value={option}>
                    {option}
                  </option>
                ))}
              </Select>
            </SettingItem>
          );
        })}
        {BOOL_SETTINGS.map(({ field, label, subtitle, cliDefault }) => (
          <SettingItem key={field} label={label} subtitle={subtitle}>
            <Switch
              aria-label={field}
              checked={geminiConfig[field] ?? cliDefault}
              onCheckedChange={(checked) => void persistGeminiConfig({ [field]: checked })}
              disabled={saving}
            />
          </SettingItem>
        ))}
      </div>
    </div>
  );
}

export function CliManagerGeminiTab({
  geminiAvailable,
  geminiLoading,
  geminiInfo,
  geminiConfigLoading,
  geminiConfigSaving,
  geminiConfig,
  geminiModels,
  geminiModelsError,
  refreshGeminiInfo,
  persistGeminiConfig,
}: CliManagerGeminiTabProps) {
  const [versionRefreshToken, setVersionRefreshToken] = useState(0);
  const loading = geminiLoading || geminiConfigLoading;
  const configDir = geminiConfig?.configDir ?? "—";
  const configPath = geminiConfig?.configPath ?? "—";

  async function refreshStatus() {
    try {
      await refreshGeminiInfo();
    } finally {
      setVersionRefreshToken((value) => value + 1);
    }
  }

  return (
    <div className="space-y-6">
      <Card className="overflow-hidden">
        <div className="border-b border-border">
          <div className="flex flex-col md:flex-row items-start md:items-center justify-between gap-4 p-6">
            <div className="flex items-center gap-4">
              <div className="h-14 w-14 rounded-xl bg-card/5 dark:bg-secondary flex items-center justify-center text-secondary-foreground">
                <Cpu className="h-8 w-8" />
              </div>
              <div>
                <h2 className="text-base font-semibold text-foreground">Gemini</h2>
                <div className="flex items-center gap-2 mt-1 flex-wrap">
                  {geminiAvailable === "available" && geminiInfo?.found ? (
                    <>
                      <span className="inline-flex items-center gap-1.5 rounded-full bg-green-50 dark:bg-green-900/30 px-2.5 py-0.5 text-xs font-medium text-green-700 dark:text-green-400 ring-1 ring-inset ring-green-600/20">
                        <CheckCircle2 className="h-3 w-3" />
                        已安装 {geminiInfo.version}
                      </span>
                      <CliVersionBadge
                        cliKey="gemini"
                        installedVersion={geminiInfo.version}
                        refreshToken={versionRefreshToken}
                        onUpdateComplete={() => void refreshStatus()}
                      />
                    </>
                  ) : geminiAvailable === "checking" || loading ? (
                    <span className="inline-flex items-center gap-1.5 rounded-full bg-blue-50 dark:bg-blue-900/30 px-2.5 py-0.5 text-xs font-medium text-blue-700 dark:text-blue-400 ring-1 ring-inset ring-blue-600/20">
                      <RefreshCw className="h-3 w-3 animate-spin" />
                      检测中...
                    </span>
                  ) : (
                    <span className="inline-flex items-center gap-1.5 rounded-full bg-secondary px-2.5 py-0.5 text-xs font-medium text-muted-foreground ring-1 ring-inset ring-border">
                      未检测到
                    </span>
                  )}
                </div>
              </div>
            </div>

            <Button
              onClick={() => void refreshStatus()}
              variant="secondary"
              size="sm"
              disabled={loading}
              className="gap-2"
            >
              <RefreshCw className={cn("h-3.5 w-3.5", loading && "animate-spin")} />
              刷新状态
            </Button>
          </div>
        </div>

        <div className="px-6 pb-6">
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-3 mt-2">
            <InfoTile
              icon={<FolderOpen className="h-3 w-3" />}
              label="配置目录"
              value={configDir}
            />
            <InfoTile
              icon={<FileJson className="h-3 w-3" />}
              label="settings.json"
              value={configPath}
              detail={
                geminiConfig ? (
                  <div className="mt-1 text-[11px] text-muted-foreground">
                    {geminiConfig.exists ? "已存在" : "不存在（保存时自动创建）"}
                  </div>
                ) : null
              }
            />
            <InfoTile
              icon={<Cpu className="h-3 w-3" />}
              label="可执行文件"
              value={geminiInfo?.executable_path ?? "—"}
            />
            <InfoTile
              icon={<Settings className="h-3 w-3" />}
              label="解析方式"
              value={geminiInfo?.resolved_via ?? "—"}
              detail={
                <div className="mt-1 text-[11px] text-muted-foreground">
                  SHELL: {geminiInfo?.shell ?? "—"}
                </div>
              }
            />
          </div>
          {geminiInfo && !geminiInfo.found ? (
            <div className="mt-3 text-xs text-muted-foreground">
              安装：
              <code className="font-mono">
                curl -fsSL https://antigravity.google/cli/install.sh | bash
              </code>
              ，Windows 见 antigravity.google/docs/getting-started。
            </div>
          ) : null}
          <div className="mt-3 text-xs text-muted-foreground leading-relaxed">
            Gemini 对接的是 Antigravity CLI（agy）和 Antigravity 应用。这里修改 agy 的
            settings.json；是否经过网关由 Gemini 代理开关决定。
          </div>
        </div>

        {geminiAvailable === "unavailable" ? (
          <div className="text-sm text-muted-foreground text-center py-8">数据不可用</div>
        ) : !geminiInfo ? (
          <div className="text-sm text-muted-foreground text-center py-8">暂无信息，请尝试刷新</div>
        ) : (
          <div className="p-6 space-y-6">
            {geminiConfig ? (
              <ConfigSections
                geminiConfig={geminiConfig}
                geminiModels={geminiModels}
                geminiModelsError={geminiModelsError}
                saving={geminiConfigSaving}
                persistGeminiConfig={persistGeminiConfig}
              />
            ) : (
              <div className="text-sm text-muted-foreground text-center py-8">
                暂无配置，请尝试刷新
              </div>
            )}
          </div>
        )}

        {geminiInfo?.error && (
          <div className="mt-4 rounded-lg bg-rose-50 dark:bg-rose-900/30 p-4 text-sm text-rose-600 dark:text-rose-400 flex items-start gap-2">
            <AlertTriangle className="h-5 w-5 shrink-0" />
            <div>
              <span className="font-semibold">检测失败：</span>
              {geminiInfo.error}
            </div>
          </div>
        )}
      </Card>
    </div>
  );
}
