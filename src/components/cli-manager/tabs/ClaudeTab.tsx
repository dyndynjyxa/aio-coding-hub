// Usage: UI for configuring Claude Code global settings (settings.json) and safe env toggles.

import { useCallback, useEffect, useState, type ReactNode } from "react";
import { toast } from "sonner";
import type {
  ClaudeCliInfo,
  ClaudeHookGroup,
  ClaudeSettingsPatch,
  ClaudeSettingsState,
} from "../../../services/cli/cliManager";
import type { ProviderSummary } from "../../../services/providers/providers";
import {
  useCliManagerClaudeHooksQuery,
  useCliManagerClaudeHooksSetMutation,
} from "../../../query/cliManager";
import { logToConsole } from "../../../services/consoleLog";
import { cn } from "../../../utils/cn";
import { CliVersionBadge } from "../CliVersionBadge";
import { Button } from "../../../ui/Button";
import { Card } from "../../../ui/Card";
import { Dialog } from "../../../ui/Dialog";
import { EmptyState } from "../../../ui/EmptyState";
import { ErrorState } from "../../../ui/ErrorState";
import { Input } from "../../../ui/Input";
import { Select } from "../../../ui/Select";
import { Spinner } from "../../../ui/Spinner";
import { Switch } from "../../../ui/Switch";
import { Textarea } from "../../../ui/Textarea";
import {
  AlertTriangle,
  Bot,
  CheckCircle2,
  ExternalLink,
  FileJson,
  FolderOpen,
  Pencil,
  Plus,
  RefreshCw,
  Settings,
  Shield,
  Terminal,
  Trash2,
  Zap,
} from "lucide-react";

export type CliManagerAvailability = "checking" | "available" | "unavailable";

export type CliManagerClaudeTabProps = {
  claudeAvailable: CliManagerAvailability;
  claudeLoading: boolean;
  claudeInfo: ClaudeCliInfo | null;
  claudeSettingsLoading: boolean;
  claudeSettingsSaving: boolean;
  claudeSettings: ClaudeSettingsState | null;
  providers: ProviderSummary[] | null;
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
        <div className="text-sm text-secondary-foreground">{label}</div>
        <div className="mt-1 text-xs text-muted-foreground leading-relaxed">{subtitle}</div>
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
  | "env_claude_code_auto_compact_window"
  | "env_claude_code_blocking_limit_override"
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

const HOOK_EVENTS = [
  "SessionStart",
  "Setup",
  "UserPromptSubmit",
  "UserPromptExpansion",
  "PreToolUse",
  "PermissionRequest",
  "PermissionDenied",
  "PostToolUse",
  "PostToolUseFailure",
  "PostToolBatch",
  "Notification",
  "SubagentStart",
  "SubagentStop",
  "TaskCreated",
  "TaskCompleted",
  "Stop",
  "StopFailure",
  "TeammateIdle",
  "InstructionsLoaded",
  "ConfigChange",
  "CwdChanged",
  "FileChanged",
  "WorktreeCreate",
  "WorktreeRemove",
  "PreCompact",
  "PostCompact",
  "Elicitation",
  "ElicitationResult",
  "SessionEnd",
] as const;

type HookEditorState = {
  mode: "create" | "edit";
  index: number;
  hookIndex: number;
  event: string;
  matcher: string;
  command: string;
  timeout: string;
};

function ClaudeHooksSection() {
  const hooksQuery = useCliManagerClaudeHooksQuery();
  const hooksMutation = useCliManagerClaudeHooksSetMutation();

  const groups = hooksQuery.data?.groups ?? [];
  const loading = hooksQuery.isLoading;
  const loadError = hooksQuery.isError ? String(hooksQuery.error) : "";
  const saving = hooksMutation.isPending;

  const [editor, setEditor] = useState<HookEditorState | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<number | null>(null);

  const persistGroups = useCallback(
    async (nextGroups: ClaudeHookGroup[]) => {
      try {
        await hooksMutation.mutateAsync({ groups: nextGroups });
        toast("已保存 Hooks 配置");
        return true;
      } catch (err) {
        logToConsole("error", "保存 Hooks 配置失败", { error: String(err) });
        toast("保存 Hooks 失败：请稍后重试");
        return false;
      }
    },
    [hooksMutation]
  );

  function openCreate() {
    setEditor({
      mode: "create",
      index: -1,
      hookIndex: 0,
      event: "PreToolUse",
      matcher: "",
      command: "",
      timeout: "",
    });
  }

  function openEdit(index: number, hookIndex = 0) {
    const group = groups[index];
    if (!group) return;
    const hook = group.hooks[hookIndex];
    setEditor({
      mode: "edit",
      index,
      hookIndex,
      event: group.event,
      matcher: group.matcher,
      command: hook?.command ?? "",
      timeout: hook?.timeout != null ? String(hook.timeout) : "",
    });
  }

  async function handleSave() {
    if (!editor) return;
    if (!editor.command.trim()) {
      toast("请填写命令");
      return;
    }

    const timeout = editor.timeout.trim() ? Number(editor.timeout.trim()) : null;
    if (timeout != null && (!Number.isSafeInteger(timeout) || timeout < 0)) {
      toast("超时必须为非负安全整数");
      return;
    }

    const editedHook = {
      hook_type: "command" as const,
      command: editor.command.trim(),
      timeout,
    };

    const next = [...groups];
    if (editor.mode === "edit" && editor.index >= 0) {
      const existing = groups[editor.index];
      const updatedHooks = [...(existing?.hooks ?? [])];
      updatedHooks[editor.hookIndex] = editedHook;
      next[editor.index] = {
        event: editor.event,
        matcher: editor.matcher,
        hooks: updatedHooks,
      };
    } else {
      next.push({
        event: editor.event,
        matcher: editor.matcher,
        hooks: [editedHook],
      });
    }

    const ok = await persistGroups(next);
    if (ok) setEditor(null);
  }

  async function handleDelete() {
    if (deleteTarget == null) return;
    const next = groups.filter((_, index) => index !== deleteTarget);
    const ok = await persistGroups(next);
    if (ok) setDeleteTarget(null);
  }

  return (
    <div className="rounded-lg border border-border bg-white p-5 dark:border-border dark:bg-secondary">
      <div className="mb-4 flex items-center justify-between gap-3">
        <h3 className="flex items-center gap-2 text-sm font-semibold text-foreground">
          <Zap className="h-4 w-4 text-muted-foreground" />
          Claude Code Hooks ({groups.length})
        </h3>
        <div className="flex items-center gap-2">
          <Button
            onClick={() => void hooksQuery.refetch()}
            variant="secondary"
            size="sm"
            className="h-8"
            disabled={loading}
          >
            <RefreshCw className="mr-1 h-3 w-3" />
            刷新 Hooks
          </Button>
          <Button
            onClick={openCreate}
            variant="secondary"
            size="sm"
            className="h-8"
            disabled={loading || hooksQuery.isError}
          >
            <Plus className="mr-1 h-3 w-3" />
            添加
          </Button>
        </div>
      </div>

      {loading ? (
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Spinner size="sm" />
          加载中…
        </div>
      ) : hooksQuery.isError ? (
        <ErrorState
          title="读取 Hooks 失败"
          message={loadError}
          onRetry={() => void hooksQuery.refetch()}
        />
      ) : groups.length === 0 ? (
        <EmptyState
          title="暂无 Hooks 配置"
          description="Hooks 可在 Claude Code 执行特定操作时自动运行脚本。点击「添加」创建第一个 Hook。"
          variant="dashed"
        />
      ) : (
        <div className="space-y-2">
          {groups.map((group, index) => (
            <div
              key={`${group.event}-${group.matcher}-${index}`}
              className="flex items-start justify-between gap-3 rounded-lg border border-border bg-white p-3 dark:border-border dark:bg-secondary"
            >
              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-center gap-1.5">
                  <span className="inline-flex items-center rounded-md bg-blue-50 px-2 py-0.5 text-xs font-medium text-blue-700 dark:bg-blue-900/30 dark:text-blue-300">
                    {group.event}
                  </span>
                  {group.matcher ? (
                    <span className="inline-flex items-center rounded-md bg-secondary px-2 py-0.5 text-xs text-muted-foreground dark:bg-secondary dark:text-secondary-foreground">
                      {group.matcher}
                    </span>
                  ) : null}
                </div>
                {group.hooks.map((hook, hookIndex) => (
                  <div key={hookIndex} className="mt-1.5 flex items-center gap-1.5">
                    <div
                      className="min-w-0 flex-1 truncate font-mono text-xs text-muted-foreground"
                      title={hook.command}
                    >
                      {hook.command}
                      {hook.timeout != null ? (
                        <span className="ml-2 text-muted-foreground">({hook.timeout}s)</span>
                      ) : null}
                    </div>
                    {group.hooks.length > 1 ? (
                      <Button
                        onClick={() => openEdit(index, hookIndex)}
                        variant="secondary"
                        size="sm"
                        className="h-5 w-5 shrink-0 p-0"
                        title="编辑此命令"
                      >
                        <Pencil className="h-2.5 w-2.5" />
                      </Button>
                    ) : null}
                  </div>
                ))}
              </div>
              <div className="flex shrink-0 items-center gap-1">
                <Button
                  onClick={() => openEdit(index)}
                  variant="secondary"
                  size="sm"
                  className="h-7 w-7 p-0"
                  title="编辑"
                >
                  <Pencil className="h-3 w-3" />
                </Button>
                <Button
                  onClick={() => setDeleteTarget(index)}
                  variant="secondary"
                  size="sm"
                  className="h-7 w-7 p-0"
                  title="删除"
                >
                  <Trash2 className="h-3 w-3" />
                </Button>
              </div>
            </div>
          ))}
        </div>
      )}

      {editor ? (
        <Dialog
          open={true}
          onOpenChange={(open) => {
            if (!open && !saving) setEditor(null);
          }}
          title={editor.mode === "create" ? "添加 Hook" : "编辑 Hook"}
          className="max-w-lg"
        >
          <div className="flex flex-col gap-4">
            <div>
              <label className="mb-1 block text-xs font-medium text-muted-foreground dark:text-secondary-foreground">
                事件
              </label>
              <Select
                value={editor.event}
                onChange={(event) => setEditor({ ...editor, event: event.target.value })}
                className="w-full"
              >
                {HOOK_EVENTS.map((eventName) => (
                  <option key={eventName} value={eventName}>
                    {eventName}
                  </option>
                ))}
              </Select>
            </div>
            <div>
              <label className="mb-1 block text-xs font-medium text-muted-foreground dark:text-secondary-foreground">
                Matcher（匹配工具名或事件子类型，多个用 | 分隔，留空匹配全部）
              </label>
              <Input
                value={editor.matcher}
                onChange={(event) => setEditor({ ...editor, matcher: event.target.value })}
                placeholder="例如 Edit|Write 或留空"
                className="text-sm"
              />
            </div>
            <div>
              <label className="mb-1 block text-xs font-medium text-muted-foreground dark:text-secondary-foreground">
                命令
              </label>
              <Input
                value={editor.command}
                onChange={(event) => setEditor({ ...editor, command: event.target.value })}
                placeholder="要执行的 shell 命令"
                className="font-mono text-sm"
              />
            </div>
            <div>
              <label className="mb-1 block text-xs font-medium text-muted-foreground dark:text-secondary-foreground">
                超时（秒，留空使用默认值）
              </label>
              <Input
                value={editor.timeout}
                onChange={(event) =>
                  setEditor({
                    ...editor,
                    timeout: event.target.value.replace(/[^0-9]/g, ""),
                  })
                }
                placeholder="例如 30"
                className="text-sm"
              />
            </div>
            <div className="flex justify-end gap-2">
              <Button
                onClick={() => setEditor(null)}
                variant="secondary"
                size="sm"
                disabled={saving}
              >
                取消
              </Button>
              <Button
                onClick={() => void handleSave()}
                variant="primary"
                size="sm"
                disabled={saving || !editor.command.trim()}
              >
                {saving ? "保存中…" : "保存"}
              </Button>
            </div>
          </div>
        </Dialog>
      ) : null}

      <Dialog
        open={deleteTarget != null}
        onOpenChange={(open) => {
          if (!open && !saving) setDeleteTarget(null);
        }}
        title="确认删除 Hook"
        description={
          deleteTarget != null && groups[deleteTarget]
            ? `将删除事件 ${groups[deleteTarget].event} 的 Hook${
                groups[deleteTarget].matcher ? `（matcher: ${groups[deleteTarget].matcher}）` : ""
              }`
            : undefined
        }
        className="max-w-lg"
      >
        <div className="flex flex-wrap items-center justify-end gap-2">
          <Button onClick={() => setDeleteTarget(null)} variant="secondary" disabled={saving}>
            取消
          </Button>
          <Button onClick={() => void handleDelete()} variant="primary" disabled={saving}>
            {saving ? "删除中…" : "确认删除"}
          </Button>
        </div>
      </Dialog>
    </div>
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
  const [versionRefreshToken, setVersionRefreshToken] = useState(0);
  const [modelText, setModelText] = useState("");
  const [outputStyleText, setOutputStyleText] = useState("");
  const [languageText, setLanguageText] = useState("");
  const [mcpTimeoutMsText, setMcpTimeoutMsText] = useState("");
  const [mcpToolTimeoutMsText, setMcpToolTimeoutMsText] = useState("");
  const [autoCompactWindowText, setAutoCompactWindowText] = useState("");
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
    setAutoCompactWindowText(
      claudeSettings.env_claude_code_auto_compact_window == null
        ? ""
        : String(claudeSettings.env_claude_code_auto_compact_window)
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

  async function refreshClaudeStatus() {
    try {
      await refreshClaude();
    } finally {
      setVersionRefreshToken((value) => value + 1);
    }
  }

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

  function revertAutoCompactWindowInput() {
    if (!claudeSettings) return;
    setAutoCompactWindowText(
      claudeSettings.env_claude_code_auto_compact_window == null
        ? ""
        : String(claudeSettings.env_claude_code_auto_compact_window)
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
        <div className="border-b border-border">
          <div className="flex flex-col gap-4 p-6">
            <div className="flex flex-col md:flex-row items-start md:items-center justify-between gap-4">
              <div className="flex items-center gap-4">
                <div className="h-14 w-14 rounded-xl bg-[#D97757]/10 flex items-center justify-center text-[#D97757]">
                  <Bot className="h-8 w-8" />
                </div>
                <div>
                  <h2 className="text-xl font-bold text-foreground">Claude Code</h2>
                  <div className="flex items-center gap-2 mt-1">
                    {claudeAvailable === "available" && claudeInfo?.found ? (
                      <>
                        <span className="inline-flex items-center gap-1.5 rounded-full bg-green-50 dark:bg-green-900/30 px-2.5 py-0.5 text-xs font-medium text-green-700 dark:text-green-400 ring-1 ring-inset ring-green-600/20">
                          <CheckCircle2 className="h-3 w-3" />
                          已安装 {claudeInfo.version}
                        </span>
                        <CliVersionBadge
                          cliKey="claude"
                          installedVersion={claudeInfo.version}
                          refreshToken={versionRefreshToken}
                          onUpdateComplete={refreshClaudeStatus}
                        />
                      </>
                    ) : claudeAvailable === "checking" || loading ? (
                      <span className="inline-flex items-center gap-1.5 rounded-full bg-blue-50 dark:bg-blue-900/30 px-2.5 py-0.5 text-xs font-medium text-blue-700 dark:text-blue-400 ring-1 ring-inset ring-blue-600/20">
                        <RefreshCw className="h-3 w-3 animate-spin" />
                        加载中...
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
                onClick={() => void refreshClaudeStatus()}
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
                <div className="bg-secondary rounded-lg p-3 border border-border">
                  <div className="flex items-center gap-1.5 text-xs text-muted-foreground mb-1.5">
                    <FolderOpen className="h-3 w-3" />
                    配置目录
                  </div>
                  <div className="flex items-center gap-1.5">
                    <div
                      className="font-mono text-xs text-secondary-foreground truncate flex-1"
                      title={configDir}
                    >
                      {configDir ?? "—"}
                    </div>
                    <Button
                      onClick={() => void openClaudeConfigDir()}
                      disabled={!configDir}
                      size="sm"
                      variant="ghost"
                      className="shrink-0 h-6 w-6 p-0 hover:bg-muted dark:hover:bg-secondary"
                      title="打开配置目录"
                    >
                      <ExternalLink className="h-3 w-3" />
                    </Button>
                  </div>
                </div>

                <div className="bg-secondary rounded-lg p-3 border border-border">
                  <div className="flex items-center gap-1.5 text-xs text-muted-foreground mb-1.5">
                    <FileJson className="h-3 w-3" />
                    settings.json
                  </div>
                  <div
                    className="font-mono text-xs text-secondary-foreground truncate"
                    title={settingsPath ?? "—"}
                  >
                    {settingsPath ?? "—"}
                  </div>
                  {claudeSettings ? (
                    <div className="mt-1 text-[11px] text-muted-foreground">
                      {claudeSettings.exists ? "已存在" : "不存在（将自动创建）"}
                    </div>
                  ) : null}
                </div>

                <div className="bg-secondary rounded-lg p-3 border border-border">
                  <div className="flex items-center gap-1.5 text-xs text-muted-foreground mb-1.5">
                    <Terminal className="h-3 w-3" />
                    可执行文件
                  </div>
                  <div
                    className="font-mono text-xs text-secondary-foreground truncate"
                    title={claudeInfo?.executable_path ?? "—"}
                  >
                    {claudeInfo?.executable_path ?? "—"}
                  </div>
                </div>

                <div className="bg-secondary rounded-lg p-3 border border-border">
                  <div className="flex items-center gap-1.5 text-xs text-muted-foreground mb-1.5">
                    <Settings className="h-3 w-3" />
                    解析方式
                  </div>
                  <div
                    className="font-mono text-xs text-secondary-foreground truncate"
                    title={claudeInfo?.resolved_via ?? "—"}
                  >
                    {claudeInfo?.resolved_via ?? "—"}
                  </div>
                  <div className="mt-1 text-[11px] text-muted-foreground">
                    SHELL: {claudeInfo?.shell ?? "—"}
                  </div>
                </div>
              </div>
            )}
          </div>
        </div>

        {claudeAvailable === "unavailable" ? (
          <div className="text-sm text-muted-foreground text-center py-8">数据不可用</div>
        ) : !claudeSettings ? (
          <div className="p-6 space-y-6">
            <div className="text-sm text-muted-foreground text-center py-8">
              暂无配置，请尝试刷新
            </div>
          </div>
        ) : (
          <div className="p-6 space-y-6">
            <div className="rounded-lg border border-border bg-white dark:bg-secondary p-5">
              <h3 className="text-sm font-semibold text-foreground flex items-center gap-2 mb-3">
                <Settings className="h-4 w-4 text-muted-foreground" />
                基础配置
              </h3>
              <div className="divide-y divide-border">
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

            <div className="rounded-lg border border-border bg-white dark:bg-secondary p-5">
              <h3 className="text-sm font-semibold text-foreground flex items-center gap-2 mb-3">
                <Settings className="h-4 w-4 text-muted-foreground" />
                交互与显示
              </h3>
              <div className="divide-y divide-border">
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

                <SettingItem
                  label="关闭 Claude Git 参与者"
                  subtitle="开启后会写入 attribution.commit / attribution.pr 为空字符串，隐藏 Git commit / PR 里的 Claude 标记；关闭后删除这两个字段。"
                >
                  <Switch
                    checked={claudeSettings.disable_git_participant}
                    onCheckedChange={(checked) =>
                      void persistClaudeSettings({ disable_git_participant: checked })
                    }
                    disabled={saving}
                  />
                </SettingItem>
              </div>
            </div>

            <div className="rounded-lg border border-border bg-white dark:bg-secondary p-5">
              <h3 className="text-sm font-semibold text-foreground flex items-center gap-2 mb-3">
                <Shield className="h-4 w-4 text-muted-foreground" />
                Permissions
              </h3>
              <div className="divide-y divide-border">
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

            <ClaudeHooksSection />

            <div className="rounded-lg border border-amber-200 dark:border-amber-700 bg-amber-50/50 dark:bg-amber-900/20 p-5">
              <h3 className="text-sm font-semibold text-amber-900 dark:text-amber-400 flex items-center gap-2 mb-1">
                <AlertTriangle className="h-4 w-4 text-amber-500" />
                实验性功能
              </h3>
              <p className="text-xs text-amber-700 dark:text-amber-400 mb-3">
                以下功能为实验性质，可能随时变更或移除。
              </p>
              <div className="divide-y divide-amber-100 dark:divide-amber-800">
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

            <div className="rounded-lg border border-border bg-white dark:bg-secondary p-5">
              <h3 className="text-sm font-semibold text-foreground flex items-center gap-2 mb-3">
                <FileJson className="h-4 w-4 text-muted-foreground" />
                环境配置（env / 白名单）
              </h3>
              <div className="divide-y divide-border">
                <SettingItem
                  label="CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS"
                  subtitle="启用 Agent Teams 功能，允许多个 Agent 协作完成任务。"
                >
                  <Switch
                    checked={claudeSettings.env_experimental_agent_teams}
                    onCheckedChange={(checked) =>
                      void persistClaudeSettings({ env_experimental_agent_teams: checked })
                    }
                    disabled={saving}
                  />
                </SettingItem>

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
                  label="CLAUDE_CODE_AUTO_COMPACT_WINDOW"
                  envVarName="CLAUDE_CODE_AUTO_COMPACT_WINDOW"
                  subtitle="设置 auto compact window。留空或 0 表示删除该键并回退 Claude Code 默认值。"
                  value={autoCompactWindowText}
                  onValueChange={setAutoCompactWindowText}
                  patchKey="env_claude_code_auto_compact_window"
                  disabled={saving}
                  revert={revertAutoCompactWindowInput}
                  persist={persistClaudeSettings}
                  placeholder="例如: 200000"
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
                  subtitle="启用 attribution header, 开启后解决部分中转无法使用的问题"
                >
                  <Switch
                    checked={claudeSettings.env_claude_code_attribution_header}
                    onCheckedChange={(checked) =>
                      void persistClaudeSettings({
                        env_claude_code_attribution_header: checked,
                      })
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
                  label="CLAUDE_CODE_DISABLE_1M_CONTEXT"
                  subtitle="关闭 1M context 支持。开启时写入 1，关闭时删除该键。"
                >
                  <Switch
                    checked={claudeSettings.env_claude_code_disable_1m_context}
                    onCheckedChange={(checked) =>
                      void persistClaudeSettings({ env_claude_code_disable_1m_context: checked })
                    }
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem
                  label="CLAUDE_CODE_SKIP_PROMPT_HISTORY"
                  subtitle="多开 Claude Code 可能产生竞态冲突，打开此选项屏蔽相关日志（开启写入 1；关闭删除该项）。"
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
          <div className="mt-4 rounded-lg bg-rose-50 dark:bg-rose-900/30 p-4 text-sm text-rose-600 dark:text-rose-400 flex items-start gap-2">
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
