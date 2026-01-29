// Usage: UI for configuring Claude Code global settings (settings.json) and safe env toggles.

import { useEffect, useState, type ReactNode } from "react";
import { toast } from "sonner";
import type {
  ClaudeCliInfo,
  ClaudeSettingsPatch,
  ClaudeSettingsState,
} from "../../../services/cliManager";
import { cn } from "../../../utils/cn";
import { Button } from "../../../ui/Button";
import { Card } from "../../../ui/Card";
import { Input } from "../../../ui/Input";
import { Switch } from "../../../ui/Switch";
import { Textarea } from "../../../ui/Textarea";
import {
  AlertTriangle,
  Bot,
  CheckCircle2,
  ExternalLink,
  FileJson,
  FolderOpen,
  RefreshCw,
  Settings,
  Shield,
  Terminal,
} from "lucide-react";

export type CliManagerAvailability = "checking" | "available" | "unavailable";

export type CliManagerClaudeTabProps = {
  claudeAvailable: CliManagerAvailability;
  claudeLoading: boolean;
  claudeInfo: ClaudeCliInfo | null;
  claudeSettingsLoading: boolean;
  claudeSettingsSaving: boolean;
  claudeSettings: ClaudeSettingsState | null;
  refreshClaude: () => Promise<void> | void;
  openClaudeConfigDir: () => Promise<void> | void;
  persistClaudeSettings: (patch: ClaudeSettingsPatch) => Promise<void> | void;
};

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
        <div className="text-sm text-slate-700">{label}</div>
        <div className="mt-1 text-xs text-slate-500 leading-relaxed">{subtitle}</div>
      </div>
      <div className="flex flex-wrap items-center justify-end gap-2">{children}</div>
    </div>
  );
}

function boolOrDefault(value: boolean | null | undefined, fallback: boolean) {
  return value ?? fallback;
}

function parseLines(text: string): string[] {
  return text
    .split("\n")
    .map((l) => l.trim())
    .filter(Boolean);
}

function PermissionTextareaItem({
  label,
  subtitle,
  value,
  onValueChange,
  onPersist,
  placeholder,
  disabled,
}: {
  label: string;
  subtitle: string;
  value: string;
  onValueChange: (value: string) => void;
  onPersist: (lines: string[]) => void;
  placeholder: string;
  disabled: boolean;
}) {
  return (
    <SettingItem label={label} subtitle={subtitle} className="items-start">
      <div className="w-full sm:w-[560px]">
        <Textarea
          mono
          value={value}
          onChange={(e) => onValueChange(e.currentTarget.value)}
          onBlur={() => onPersist(parseLines(value))}
          rows={6}
          disabled={disabled}
          placeholder={placeholder}
        />
      </div>
    </SettingItem>
  );
}

type ClaudeEnvTimeoutPatchKey = "env_mcp_timeout_ms" | "env_mcp_tool_timeout_ms";

function EnvTimeoutItem({
  label,
  envVarName,
  subtitle,
  value,
  onValueChange,
  patchKey,
  maxTimeoutMs,
  disabled,
  normalizeTimeoutMsOrZero,
  revert,
  persist,
}: {
  label: string;
  envVarName: string;
  subtitle: string;
  value: string;
  onValueChange: (value: string) => void;
  patchKey: ClaudeEnvTimeoutPatchKey;
  maxTimeoutMs: number;
  disabled: boolean;
  normalizeTimeoutMsOrZero: (raw: string) => number;
  revert: () => void;
  persist: (patch: ClaudeSettingsPatch) => Promise<void> | void;
}) {
  return (
    <SettingItem label={label} subtitle={subtitle}>
      <Input
        type="number"
        value={value}
        onChange={(e) => onValueChange(e.currentTarget.value)}
        onBlur={() => {
          const normalized = normalizeTimeoutMsOrZero(value);
          if (!Number.isFinite(normalized) || normalized > maxTimeoutMs) {
            toast(`${envVarName} 必须为 0-${maxTimeoutMs} 毫秒`);
            revert();
            return;
          }
          void persist({ [patchKey]: normalized } as ClaudeSettingsPatch);
        }}
        className="font-mono w-[220px] max-w-full"
        min={0}
        max={maxTimeoutMs}
        disabled={disabled}
        placeholder="默认"
      />
    </SettingItem>
  );
}

type ClaudeEnvU64PatchKey =
  | "env_claude_code_blocking_limit_override"
  | "env_claude_autocompact_pct_override"
  | "env_claude_code_max_output_tokens"
  | "env_max_mcp_output_tokens";

function EnvU64Item({
  label,
  envVarName,
  subtitle,
  value,
  onValueChange,
  patchKey,
  inputMax,
  disabled,
  validate,
  revert,
  persist,
  placeholder,
}: {
  label: string;
  envVarName: string;
  subtitle: string;
  value: string;
  onValueChange: (value: string) => void;
  patchKey: ClaudeEnvU64PatchKey;
  inputMax?: number;
  disabled: boolean;
  validate?: (value: number) => string | null;
  revert: () => void;
  persist: (patch: ClaudeSettingsPatch) => Promise<void> | void;
  placeholder: string;
}) {
  return (
    <SettingItem label={label} subtitle={subtitle}>
      <Input
        type="number"
        value={value}
        onChange={(e) => onValueChange(e.currentTarget.value)}
        onBlur={() => {
          const trimmed = value.trim();
          if (!trimmed) {
            void persist({ [patchKey]: 0 } as ClaudeSettingsPatch);
            return;
          }
          const n = Math.floor(Number(trimmed));
          if (!Number.isFinite(n) || n < 0) {
            toast(`${envVarName} 必须为非负整数`);
            revert();
            return;
          }
          if (n > Number.MAX_SAFE_INTEGER) {
            toast(`${envVarName} 值过大（超过 JS 安全整数）`);
            revert();
            return;
          }
          const customError = validate?.(n);
          if (customError) {
            toast(customError);
            revert();
            return;
          }
          void persist({ [patchKey]: n } as ClaudeSettingsPatch);
        }}
        className="font-mono w-[220px] max-w-full"
        min={0}
        max={inputMax}
        disabled={disabled}
        placeholder={placeholder}
      />
    </SettingItem>
  );
}

export function CliManagerClaudeTab({
  claudeAvailable,
  claudeLoading,
  claudeInfo,
  claudeSettingsLoading,
  claudeSettingsSaving,
  claudeSettings,
  refreshClaude,
  openClaudeConfigDir,
  persistClaudeSettings,
}: CliManagerClaudeTabProps) {
  const [modelText, setModelText] = useState("");
  const [outputStyleText, setOutputStyleText] = useState("");
  const [languageText, setLanguageText] = useState("");
  const [mcpTimeoutMsText, setMcpTimeoutMsText] = useState("");
  const [mcpToolTimeoutMsText, setMcpToolTimeoutMsText] = useState("");
  const [blockingLimitOverrideText, setBlockingLimitOverrideText] = useState("");
  const [maxOutputTokensText, setMaxOutputTokensText] = useState("");
  const [maxMcpOutputTokensText, setMaxMcpOutputTokensText] = useState("");
  const [permissionsAllowText, setPermissionsAllowText] = useState("");
  const [permissionsAskText, setPermissionsAskText] = useState("");
  const [permissionsDenyText, setPermissionsDenyText] = useState("");

  useEffect(() => {
    if (!claudeSettings) return;
    setModelText(claudeSettings.model ?? "");
    setOutputStyleText(claudeSettings.output_style ?? "");
    setLanguageText(claudeSettings.language ?? "");
    setMcpTimeoutMsText(
      claudeSettings.env_mcp_timeout_ms == null ? "" : String(claudeSettings.env_mcp_timeout_ms)
    );
    setMcpToolTimeoutMsText(
      claudeSettings.env_mcp_tool_timeout_ms == null
        ? ""
        : String(claudeSettings.env_mcp_tool_timeout_ms)
    );
    setBlockingLimitOverrideText(
      claudeSettings.env_claude_code_blocking_limit_override == null
        ? ""
        : String(claudeSettings.env_claude_code_blocking_limit_override)
    );
    setMaxOutputTokensText(
      claudeSettings.env_claude_code_max_output_tokens == null
        ? ""
        : String(claudeSettings.env_claude_code_max_output_tokens)
    );
    setMaxMcpOutputTokensText(
      claudeSettings.env_max_mcp_output_tokens == null
        ? ""
        : String(claudeSettings.env_max_mcp_output_tokens)
    );
    setPermissionsAllowText((claudeSettings.permissions_allow ?? []).join("\n"));
    setPermissionsAskText((claudeSettings.permissions_ask ?? []).join("\n"));
    setPermissionsDenyText((claudeSettings.permissions_deny ?? []).join("\n"));
  }, [claudeSettings]);

  const loading = claudeLoading || claudeSettingsLoading;
  const saving = claudeSettingsSaving;

  const configDir = claudeSettings?.config_dir ?? claudeInfo?.config_dir;
  const settingsPath = claudeSettings?.settings_path ?? claudeInfo?.settings_path;

  const MAX_TIMEOUT_MS = 24 * 60 * 60 * 1000;
  function normalizeTimeoutMsOrZero(raw: string): number {
    const trimmed = raw.trim();
    if (!trimmed) return 0;
    const n = Math.floor(Number(trimmed));
    if (!Number.isFinite(n) || n < 0) return NaN;
    if (n > MAX_TIMEOUT_MS) return Infinity;
    return n;
  }

  function revertTimeoutInputs() {
    if (!claudeSettings) return;
    setMcpTimeoutMsText(
      claudeSettings.env_mcp_timeout_ms == null ? "" : String(claudeSettings.env_mcp_timeout_ms)
    );
    setMcpToolTimeoutMsText(
      claudeSettings.env_mcp_tool_timeout_ms == null
        ? ""
        : String(claudeSettings.env_mcp_tool_timeout_ms)
    );
  }

  function revertBlockingLimitOverrideInput() {
    if (!claudeSettings) return;
    setBlockingLimitOverrideText(
      claudeSettings.env_claude_code_blocking_limit_override == null
        ? ""
        : String(claudeSettings.env_claude_code_blocking_limit_override)
    );
  }

  function revertMaxOutputTokensInput() {
    if (!claudeSettings) return;
    setMaxOutputTokensText(
      claudeSettings.env_claude_code_max_output_tokens == null
        ? ""
        : String(claudeSettings.env_claude_code_max_output_tokens)
    );
  }

  function revertMaxMcpOutputTokensInput() {
    if (!claudeSettings) return;
    setMaxMcpOutputTokensText(
      claudeSettings.env_max_mcp_output_tokens == null
        ? ""
        : String(claudeSettings.env_max_mcp_output_tokens)
    );
  }

  return (
    <div className="space-y-6">
      <Card className="overflow-hidden">
        <div className="border-b border-slate-100">
          <div className="flex flex-col gap-4 p-6">
            <div className="flex flex-col md:flex-row items-start md:items-center justify-between gap-4">
              <div className="flex items-center gap-4">
                <div className="h-14 w-14 rounded-xl bg-[#D97757]/10 flex items-center justify-center text-[#D97757]">
                  <Bot className="h-8 w-8" />
                </div>
                <div>
                  <h2 className="text-xl font-bold text-slate-900">Claude Code</h2>
                  <div className="flex items-center gap-2 mt-1">
                    {claudeAvailable === "available" && claudeInfo?.found ? (
                      <span className="inline-flex items-center gap-1.5 rounded-full bg-green-50 px-2.5 py-0.5 text-xs font-medium text-green-700 ring-1 ring-inset ring-green-600/20">
                        <CheckCircle2 className="h-3 w-3" />
                        已安装 {claudeInfo.version}
                      </span>
                    ) : claudeAvailable === "checking" || loading ? (
                      <span className="inline-flex items-center gap-1.5 rounded-full bg-blue-50 px-2.5 py-0.5 text-xs font-medium text-blue-700 ring-1 ring-inset ring-blue-600/20">
                        <RefreshCw className="h-3 w-3 animate-spin" />
                        加载中...
                      </span>
                    ) : (
                      <span className="inline-flex items-center gap-1.5 rounded-full bg-slate-100 px-2.5 py-0.5 text-xs font-medium text-slate-600 ring-1 ring-inset ring-slate-500/10">
                        未检测到
                      </span>
                    )}
                  </div>
                </div>
              </div>

              <Button
                onClick={() => void refreshClaude()}
                variant="secondary"
                size="sm"
                disabled={loading}
                className="gap-2"
              >
                <RefreshCw className={cn("h-3.5 w-3.5", loading && "animate-spin")} />
                刷新
              </Button>
            </div>

            {(configDir || settingsPath || claudeInfo) && (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-3 mt-2">
                <div className="bg-slate-50 rounded-lg p-3 border border-slate-100">
                  <div className="flex items-center gap-1.5 text-xs text-slate-500 mb-1.5">
                    <FolderOpen className="h-3 w-3" />
                    配置目录
                  </div>
                  <div className="flex items-center gap-1.5">
                    <div
                      className="font-mono text-xs text-slate-700 truncate flex-1"
                      title={configDir}
                    >
                      {configDir ?? "—"}
                    </div>
                    <Button
                      onClick={() => void openClaudeConfigDir()}
                      disabled={!configDir}
                      size="sm"
                      variant="ghost"
                      className="shrink-0 h-6 w-6 p-0 hover:bg-slate-200"
                      title="打开配置目录"
                    >
                      <ExternalLink className="h-3 w-3" />
                    </Button>
                  </div>
                </div>

                <div className="bg-slate-50 rounded-lg p-3 border border-slate-100">
                  <div className="flex items-center gap-1.5 text-xs text-slate-500 mb-1.5">
                    <FileJson className="h-3 w-3" />
                    settings.json
                  </div>
                  <div
                    className="font-mono text-xs text-slate-700 truncate"
                    title={settingsPath ?? "—"}
                  >
                    {settingsPath ?? "—"}
                  </div>
                  {claudeSettings ? (
                    <div className="mt-1 text-[11px] text-slate-500">
                      {claudeSettings.exists ? "已存在" : "不存在（将自动创建）"}
                    </div>
                  ) : null}
                </div>

                <div className="bg-slate-50 rounded-lg p-3 border border-slate-100">
                  <div className="flex items-center gap-1.5 text-xs text-slate-500 mb-1.5">
                    <Terminal className="h-3 w-3" />
                    可执行文件
                  </div>
                  <div
                    className="font-mono text-xs text-slate-700 truncate"
                    title={claudeInfo?.executable_path ?? "—"}
                  >
                    {claudeInfo?.executable_path ?? "—"}
                  </div>
                </div>

                <div className="bg-slate-50 rounded-lg p-3 border border-slate-100">
                  <div className="flex items-center gap-1.5 text-xs text-slate-500 mb-1.5">
                    <Settings className="h-3 w-3" />
                    解析方式
                  </div>
                  <div
                    className="font-mono text-xs text-slate-700 truncate"
                    title={claudeInfo?.resolved_via ?? "—"}
                  >
                    {claudeInfo?.resolved_via ?? "—"}
                  </div>
                  <div className="mt-1 text-[11px] text-slate-500">
                    SHELL: {claudeInfo?.shell ?? "—"}
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>

        {claudeAvailable === "unavailable" ? (
          <div className="text-sm text-slate-600 text-center py-8">仅在 Tauri Desktop 环境可用</div>
        ) : !claudeSettings ? (
          <div className="text-sm text-slate-500 text-center py-8">暂无配置，请尝试刷新</div>
        ) : (
          <div className="p-6 space-y-6">
            <div className="rounded-lg border border-slate-200 bg-white p-5">
              <h3 className="text-sm font-semibold text-slate-900 flex items-center gap-2 mb-3">
                <Settings className="h-4 w-4 text-slate-400" />
                基础配置
              </h3>
              <div className="divide-y divide-slate-100">
                <SettingItem
                  label="默认模型 (model)"
                  subtitle="覆盖 Claude Code 默认使用的模型。留空表示不设置（交由 Claude Code 默认/上层配置决定）。"
                >
                  <Input
                    value={modelText}
                    onChange={(e) => setModelText(e.currentTarget.value)}
                    onBlur={() => void persistClaudeSettings({ model: modelText.trim() })}
                    placeholder="例如：claude-sonnet-4-5-20250929"
                    className="font-mono w-[320px] max-w-full"
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem
                  label="输出风格 (outputStyle)"
                  subtitle="配置输出风格（对应 /output-style）。留空表示不设置。"
                >
                  <Input
                    value={outputStyleText}
                    onChange={(e) => setOutputStyleText(e.currentTarget.value)}
                    onBlur={() =>
                      void persistClaudeSettings({ output_style: outputStyleText.trim() })
                    }
                    placeholder='例如："Explanatory"'
                    className="font-mono w-[320px] max-w-full"
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem label="语言 (language)" subtitle="设置默认回复语言。留空表示不设置。">
                  <Input
                    value={languageText}
                    onChange={(e) => setLanguageText(e.currentTarget.value)}
                    onBlur={() => void persistClaudeSettings({ language: languageText.trim() })}
                    placeholder='例如："japanese"'
                    className="font-mono w-[320px] max-w-full"
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem
                  label="默认启用 Thinking (alwaysThinkingEnabled)"
                  subtitle="默认启用 extended thinking（通常建议用 /config 配置；此处为显式开关）。"
                >
                  <Switch
                    checked={boolOrDefault(claudeSettings.always_thinking_enabled, false)}
                    onCheckedChange={(checked) =>
                      void persistClaudeSettings({ always_thinking_enabled: checked })
                    }
                    disabled={saving}
                  />
                </SettingItem>
              </div>
            </div>

            <div className="rounded-lg border border-slate-200 bg-white p-5">
              <h3 className="text-sm font-semibold text-slate-900 flex items-center gap-2 mb-3">
                <Settings className="h-4 w-4 text-slate-400" />
                交互与显示
              </h3>
              <div className="divide-y divide-slate-100">
                <SettingItem
                  label="显示耗时 (showTurnDuration)"
                  subtitle="显示 turn duration（默认开启）。"
                >
                  <Switch
                    checked={boolOrDefault(claudeSettings.show_turn_duration, true)}
                    onCheckedChange={(checked) =>
                      void persistClaudeSettings({ show_turn_duration: checked })
                    }
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem
                  label="Spinner Tips (spinnerTipsEnabled)"
                  subtitle="在 spinner 中显示提示（默认开启）。"
                >
                  <Switch
                    checked={boolOrDefault(claudeSettings.spinner_tips_enabled, true)}
                    onCheckedChange={(checked) =>
                      void persistClaudeSettings({ spinner_tips_enabled: checked })
                    }
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem
                  label="Terminal Progress Bar (terminalProgressBarEnabled)"
                  subtitle="在支持的终端显示进度条（默认开启）。"
                >
                  <Switch
                    checked={boolOrDefault(claudeSettings.terminal_progress_bar_enabled, true)}
                    onCheckedChange={(checked) =>
                      void persistClaudeSettings({ terminal_progress_bar_enabled: checked })
                    }
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem
                  label="Respect .gitignore (respectGitignore)"
                  subtitle="@ 文件选择器是否遵循 .gitignore（默认开启）。"
                >
                  <Switch
                    checked={boolOrDefault(claudeSettings.respect_gitignore, true)}
                    onCheckedChange={(checked) =>
                      void persistClaudeSettings({ respect_gitignore: checked })
                    }
                    disabled={saving}
                  />
                </SettingItem>
              </div>
            </div>

            <div className="rounded-lg border border-slate-200 bg-white p-5">
              <h3 className="text-sm font-semibold text-slate-900 flex items-center gap-2 mb-3">
                <Shield className="h-4 w-4 text-slate-400" />
                Permissions
              </h3>
              <div className="divide-y divide-slate-100">
                <PermissionTextareaItem
                  label="permissions.allow"
                  subtitle="允许的工具规则（每行一条）。留空表示不设置。"
                  value={permissionsAllowText}
                  onValueChange={setPermissionsAllowText}
                  onPersist={(lines) => void persistClaudeSettings({ permissions_allow: lines })}
                  disabled={saving}
                  placeholder={"例如：\nBash(git diff:*)\nRead(./docs/**)"}
                />

                <PermissionTextareaItem
                  label="permissions.ask"
                  subtitle="需要确认的工具规则（每行一条）。留空表示不设置。"
                  value={permissionsAskText}
                  onValueChange={setPermissionsAskText}
                  onPersist={(lines) => void persistClaudeSettings({ permissions_ask: lines })}
                  disabled={saving}
                  placeholder={"例如：\nBash(git push:*)"}
                />

                <PermissionTextareaItem
                  label="permissions.deny"
                  subtitle="拒绝的工具规则（每行一条）。建议用于敏感文件与危险命令。"
                  value={permissionsDenyText}
                  onValueChange={setPermissionsDenyText}
                  onPersist={(lines) => void persistClaudeSettings({ permissions_deny: lines })}
                  disabled={saving}
                  placeholder={"例如：\nRead(./.env)\nRead(./secrets/**)\nBash(rm -rf:*)"}
                />
              </div>
            </div>

            <div className="rounded-lg border border-amber-200 bg-amber-50/50 p-5">
              <h3 className="text-sm font-semibold text-amber-900 flex items-center gap-2 mb-1">
                <AlertTriangle className="h-4 w-4 text-amber-500" />
                实验性功能
              </h3>
              <p className="text-xs text-amber-700 mb-3">
                以下功能为实验性质，可能随时变更或移除。
              </p>
              <div className="divide-y divide-amber-100">
                <SettingItem
                  label="ENABLE_EXPERIMENTAL_MCP_CLI"
                  subtitle="启用 MCP-CLI 模式，按需加载工具以节省上下文（可节省约 95% 上下文）。⚠️ 与 ENABLE_TOOL_SEARCH 互斥。"
                >
                  <Switch
                    checked={claudeSettings.env_enable_experimental_mcp_cli}
                    onCheckedChange={(checked) => {
                      if (checked) {
                        void persistClaudeSettings({
                          env_enable_experimental_mcp_cli: true,
                          env_enable_tool_search: false,
                        });
                      } else {
                        void persistClaudeSettings({ env_enable_experimental_mcp_cli: false });
                      }
                    }}
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem
                  label="ENABLE_TOOL_SEARCH"
                  subtitle="启用工具搜索，当 MCP 工具超过 10% 上下文时自动懒加载。⚠️ 与 ENABLE_EXPERIMENTAL_MCP_CLI 互斥。"
                >
                  <Switch
                    checked={claudeSettings.env_enable_tool_search}
                    onCheckedChange={(checked) => {
                      if (checked) {
                        void persistClaudeSettings({
                          env_enable_tool_search: true,
                          env_enable_experimental_mcp_cli: false,
                        });
                      } else {
                        void persistClaudeSettings({ env_enable_tool_search: false });
                      }
                    }}
                    disabled={saving}
                  />
                </SettingItem>

                <EnvU64Item
                  label="MAX_MCP_OUTPUT_TOKENS"
                  envVarName="MAX_MCP_OUTPUT_TOKENS"
                  subtitle="MCP 工具响应的最大 tokens。留空或 0 表示使用默认值（25000）。"
                  value={maxMcpOutputTokensText}
                  onValueChange={setMaxMcpOutputTokensText}
                  patchKey="env_max_mcp_output_tokens"
                  disabled={saving}
                  revert={revertMaxMcpOutputTokensInput}
                  persist={persistClaudeSettings}
                  placeholder="25000"
                />
              </div>
            </div>

            <div className="rounded-lg border border-slate-200 bg-white p-5">
              <h3 className="text-sm font-semibold text-slate-900 flex items-center gap-2 mb-3">
                <FileJson className="h-4 w-4 text-slate-400" />
                环境配置（env / 白名单）
              </h3>
              <div className="divide-y divide-slate-100">
                <EnvTimeoutItem
                  label="MCP_TIMEOUT (ms)"
                  envVarName="MCP_TIMEOUT"
                  subtitle={`MCP server 启动超时（0/留空=默认，范围 0-${MAX_TIMEOUT_MS}）。`}
                  value={mcpTimeoutMsText}
                  onValueChange={setMcpTimeoutMsText}
                  patchKey="env_mcp_timeout_ms"
                  maxTimeoutMs={MAX_TIMEOUT_MS}
                  disabled={saving}
                  normalizeTimeoutMsOrZero={normalizeTimeoutMsOrZero}
                  revert={revertTimeoutInputs}
                  persist={persistClaudeSettings}
                />

                <EnvTimeoutItem
                  label="MCP_TOOL_TIMEOUT (ms)"
                  envVarName="MCP_TOOL_TIMEOUT"
                  subtitle={`MCP tool 执行超时（0/留空=默认，范围 0-${MAX_TIMEOUT_MS}）。`}
                  value={mcpToolTimeoutMsText}
                  onValueChange={setMcpToolTimeoutMsText}
                  patchKey="env_mcp_tool_timeout_ms"
                  maxTimeoutMs={MAX_TIMEOUT_MS}
                  disabled={saving}
                  normalizeTimeoutMsOrZero={normalizeTimeoutMsOrZero}
                  revert={revertTimeoutInputs}
                  persist={persistClaudeSettings}
                />

                <EnvU64Item
                  label="CLAUDE_CODE_BLOCKING_LIMIT_OVERRIDE"
                  envVarName="CLAUDE_CODE_BLOCKING_LIMIT_OVERRIDE"
                  subtitle="覆盖 blocking limit（有效上下文/阻断阈值）。留空或 0 表示不设置该项。"
                  value={blockingLimitOverrideText}
                  onValueChange={setBlockingLimitOverrideText}
                  patchKey="env_claude_code_blocking_limit_override"
                  disabled={saving}
                  revert={revertBlockingLimitOverrideInput}
                  persist={persistClaudeSettings}
                  placeholder="例如：193000"
                />

                <EnvU64Item
                  label="CLAUDE_CODE_MAX_OUTPUT_TOKENS"
                  envVarName="CLAUDE_CODE_MAX_OUTPUT_TOKENS"
                  subtitle="限制最大输出 tokens（可能影响有效上下文窗口）。留空或 0 表示不设置该项。"
                  value={maxOutputTokensText}
                  onValueChange={setMaxOutputTokensText}
                  patchKey="env_claude_code_max_output_tokens"
                  disabled={saving}
                  revert={revertMaxOutputTokensInput}
                  persist={persistClaudeSettings}
                  placeholder="默认"
                />

                <SettingItem
                  label="CLAUDE_CODE_ATTRIBUTION_HEADER"
                  subtitle="禁用 attribution header（开启写入 0；关闭删除该项）。"
                >
                  <Switch
                    checked={claudeSettings.env_claude_code_attribution_header_disabled}
                    onCheckedChange={(checked) =>
                      void persistClaudeSettings({
                        env_claude_code_attribution_header_disabled: checked,
                      })
                    }
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem label="DISABLE_ERROR_REPORTING" subtitle="禁用错误上报（Sentry）。">
                  <Switch
                    checked={claudeSettings.env_disable_error_reporting}
                    onCheckedChange={(checked) =>
                      void persistClaudeSettings({ env_disable_error_reporting: checked })
                    }
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem label="DISABLE_TELEMETRY" subtitle="禁用 telemetry（Statsig）。">
                  <Switch
                    checked={claudeSettings.env_disable_telemetry}
                    onCheckedChange={(checked) =>
                      void persistClaudeSettings({ env_disable_telemetry: checked })
                    }
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem
                  label="CLAUDE_CODE_DISABLE_BACKGROUND_TASKS"
                  subtitle="禁用后台任务与自动 backgrounding。"
                >
                  <Switch
                    checked={claudeSettings.env_disable_background_tasks}
                    onCheckedChange={(checked) =>
                      void persistClaudeSettings({ env_disable_background_tasks: checked })
                    }
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem
                  label="CLAUDE_CODE_DISABLE_TERMINAL_TITLE"
                  subtitle="禁用自动更新终端标题。"
                >
                  <Switch
                    checked={claudeSettings.env_disable_terminal_title}
                    onCheckedChange={(checked) =>
                      void persistClaudeSettings({ env_disable_terminal_title: checked })
                    }
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem label="CLAUDE_BASH_NO_LOGIN" subtitle="跳过 login shell（BashTool）。">
                  <Switch
                    checked={claudeSettings.env_claude_bash_no_login}
                    onCheckedChange={(checked) =>
                      void persistClaudeSettings({ env_claude_bash_no_login: checked })
                    }
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem
                  label="CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC"
                  subtitle="等同于设置 DISABLE_AUTOUPDATER、DISABLE_BUG_COMMAND、DISABLE_ERROR_REPORTING、DISABLE_TELEMETRY。"
                >
                  <Switch
                    checked={claudeSettings.env_claude_code_disable_nonessential_traffic}
                    onCheckedChange={(checked) =>
                      void persistClaudeSettings({
                        env_claude_code_disable_nonessential_traffic: checked,
                      })
                    }
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem
                  label="CLAUDE_CODE_PROXY_RESOLVES_HOSTS"
                  subtitle="如果 WEB_SEARCH 或 FETCH 经常获取不到结果可以打开试试。"
                >
                  <Switch
                    checked={claudeSettings.env_claude_code_proxy_resolves_hosts}
                    onCheckedChange={(checked) =>
                      void persistClaudeSettings({ env_claude_code_proxy_resolves_hosts: checked })
                    }
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem
                  label="CLAUDE_CODE_SKIP_PROMPT_HISTORY"
                  subtitle="多开 Claude Code 可能产生竞态冲突，打开此选项屏蔽相关日志。"
                >
                  <Switch
                    checked={claudeSettings.env_claude_code_skip_prompt_history}
                    onCheckedChange={(checked) =>
                      void persistClaudeSettings({ env_claude_code_skip_prompt_history: checked })
                    }
                    disabled={saving}
                  />
                </SettingItem>
              </div>
            </div>
          </div>
        )}

        {claudeInfo?.error && (
          <div className="mt-4 rounded-lg bg-rose-50 p-4 text-sm text-rose-600 flex items-start gap-2">
            <AlertTriangle className="h-5 w-5 shrink-0" />
            <div>
              <span className="font-semibold">检测失败：</span>
              {claudeInfo.error}
            </div>
          </div>
        )}
      </Card>
    </div>
  );
}
