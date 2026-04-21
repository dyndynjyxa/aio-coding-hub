import {
  lazy,
  Suspense,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import {
  cliManagerCodexConfigTomlValidate,
  type CodexConfigPatch,
  type CodexConfigState,
  type CodexConfigTomlState,
  type CodexConfigTomlValidationResult,
  type SimpleCliInfo,
} from "../../../services/cli/cliManager";
import type { AppSettings, CodexHomeMode } from "../../../services/settings/settings";
import { normalizeCustomCodexHome, buildConfigTomlPath } from "../../../utils/codexPaths";
import { isWindowsRuntime } from "../../../utils/platform";
import { cn } from "../../../utils/cn";
import { CliVersionBadge } from "../CliVersionBadge";
import { Button } from "../../../ui/Button";
import { Card } from "../../../ui/Card";
import { Input } from "../../../ui/Input";
import { Select } from "../../../ui/Select";
import { Switch } from "../../../ui/Switch";
import { RadioGroup } from "../../../ui/RadioGroup";
import {
  AlertTriangle,
  CheckCircle2,
  ExternalLink,
  FileJson,
  FolderOpen,
  RefreshCw,
  Terminal,
  Settings,
} from "lucide-react";

const LazyCodeEditor = lazy(() =>
  import("../../../ui/CodeEditor").then((m) => ({ default: m.CodeEditor }))
);

const GPT_54_MODEL = "gpt-5.4";
const GPT_54_CONTEXT_WINDOW = 1_000_000;
const GPT_54_AUTO_COMPACT_TOKEN_LIMIT = 900_000;
const FAST_SERVICE_TIER = "fast";
type PersistConfigLocationResult = "saved" | "validation_failed" | "persist_failed";

function buildModelPatch(
  model: string,
  contextWindow?: string,
  autoCompactLimit?: string
): CodexConfigPatch {
  const trimmed = model.trim();
  const isGpt54 = trimmed === GPT_54_MODEL;

  return {
    model: trimmed,
    model_context_window: isGpt54 ? parsePositiveInt(contextWindow) : null,
    model_auto_compact_token_limit: isGpt54 ? parsePositiveInt(autoCompactLimit) : null,
  };
}

/** Parse a string to a positive integer; return null on empty / NaN / <= 0. */
function parsePositiveInt(v: string | undefined): number | null {
  if (v == null) return null;
  const n = Number(v.trim());
  if (!Number.isFinite(n) || n <= 0) return null;
  return Math.round(n);
}

function buildFastModePatch(enabled: boolean): CodexConfigPatch {
  return {
    features_fast_mode: enabled,
    service_tier: enabled ? FAST_SERVICE_TIER : "",
  };
}

function buildPersonalityPatch(value: string): CodexConfigPatch {
  return {
    personality: value === "none" ? "" : value,
  };
}

function isGpt54Model(model: string | null | undefined) {
  return (model ?? "").trim() === GPT_54_MODEL;
}

function validateCustomCodexHome(value: string): string | null {
  const trimmed = value.trim();
  const normalized = normalizeCustomCodexHome(trimmed);
  if (!normalized) return "请输入 .codex 目录路径。";

  const lower = trimmed.replace(/\\/g, "/").toLowerCase();
  if (lower.includes("://")) {
    return "这里填写的是本地目录路径，不要包含协议头。";
  }
  if (/[\r\n\u0000]/.test(trimmed)) {
    return "路径中不能包含换行或控制字符。";
  }
  if (lower.endsWith(".toml") && lower !== "config.toml" && !lower.endsWith("/config.toml")) {
    return "这里填写的是 .codex 目录，不是其他 TOML 文件。";
  }

  return null;
}

function normalizeComparablePath(path: string) {
  return path
    .trim()
    .replace(/[\\/]+$/, "")
    .replace(/\\/g, "/")
    .toLowerCase();
}

export type CliManagerAvailability = "checking" | "available" | "unavailable";

export type CliManagerCodexTabProps = {
  codexAvailable: CliManagerAvailability;
  codexLoading: boolean;
  codexConfigLoading: boolean;
  codexConfigSaving: boolean;
  codexConfigTomlLoading: boolean;
  codexConfigTomlSaving: boolean;
  codexInfo: SimpleCliInfo | null;
  codexConfig: CodexConfigState | null;
  codexConfigToml: CodexConfigTomlState | null;
  appSettings?: AppSettings | null;
  codexHomeSettingsSaving?: boolean;
  refreshCodex: () => Promise<void> | void;
  openCodexConfigDir: () => Promise<void> | void;
  persistCodexConfig: (patch: CodexConfigPatch) => Promise<void> | void;
  persistCodexConfigToml: (toml: string) => Promise<boolean> | boolean;
  persistCodexHomeSettings?: (
    codexHomeMode: CodexHomeMode,
    codexHomeOverride: string
  ) => Promise<boolean> | boolean;
  pickCodexHomeDirectory?: (initialPath?: string) => Promise<string | null> | string | null;
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
        <div className="text-sm text-slate-700 dark:text-slate-300">{label}</div>
        <div className="mt-1 text-xs text-slate-500 dark:text-slate-400 leading-relaxed">
          {subtitle}
        </div>
      </div>
      <div className="flex flex-wrap items-center justify-end gap-2">{children}</div>
    </div>
  );
}

function boolOrDefault(value: boolean | null, fallback: boolean) {
  return value ?? fallback;
}

function enumOrDefault(value: string | null, fallback: string) {
  return (value ?? fallback).trim();
}

export function CliManagerCodexTab({
  codexAvailable,
  codexLoading,
  codexConfigLoading,
  codexConfigSaving,
  codexConfigTomlLoading,
  codexConfigTomlSaving,
  codexInfo,
  codexConfig,
  codexConfigToml,
  appSettings,
  codexHomeSettingsSaving = false,
  refreshCodex,
  openCodexConfigDir,
  persistCodexConfig,
  persistCodexConfigToml,
  persistCodexHomeSettings,
  pickCodexHomeDirectory,
}: CliManagerCodexTabProps) {
  const [versionRefreshToken, setVersionRefreshToken] = useState(0);
  const [modelText, setModelText] = useState("");
  const [contextWindowText, setContextWindowText] = useState("");
  const [autoCompactLimitText, setAutoCompactLimitText] = useState("");
  const [sandboxModeText, setSandboxModeText] = useState("");
  const [webSearchText, setWebSearchText] = useState("");
  const [personalityText, setPersonalityText] = useState("none");
  const [reasoningEffortText, setReasoningEffortText] = useState("");
  const [planModeReasoningEffortText, setPlanModeReasoningEffortText] = useState("");
  const [configLocationMode, setConfigLocationMode] = useState<CodexHomeMode>("user_home_default");
  const [customHomeText, setCustomHomeText] = useState("");
  const [configLocationError, setConfigLocationError] = useState<string | null>(null);
  const [selectingCodexHomeDir, setSelectingCodexHomeDir] = useState(false);

  const [tomlAdvancedOpen, setTomlAdvancedOpen] = useState(false);
  const [tomlEditEnabled, setTomlEditEnabled] = useState(false);
  const [tomlDraft, setTomlDraft] = useState("");
  const [tomlDirty, setTomlDirty] = useState(false);
  const [tomlValidating, setTomlValidating] = useState(false);
  const [tomlValidation, setTomlValidation] = useState<CodexConfigTomlValidationResult | null>(
    null
  );

  const validateSeqRef = useRef(0);
  const validateTimerRef = useRef<number | null>(null);
  const lastTomlConfigPathRef = useRef<string | null>(null);

  const validateToml = useCallback(
    async (toml: string): Promise<CodexConfigTomlValidationResult | null> => {
      const seq = validateSeqRef.current + 1;
      validateSeqRef.current = seq;
      setTomlValidating(true);
      try {
        const result = await cliManagerCodexConfigTomlValidate(toml);
        if (seq !== validateSeqRef.current) return null;
        if (!result) return null;
        setTomlValidation(result);
        return result;
      } finally {
        if (seq === validateSeqRef.current) {
          setTomlValidating(false);
        }
      }
    },
    []
  );

  useEffect(() => {
    if (!codexConfig) return;
    setModelText(codexConfig.model ?? "");
    setContextWindowText(
      codexConfig.model_context_window != null ? String(codexConfig.model_context_window) : ""
    );
    setAutoCompactLimitText(
      codexConfig.model_auto_compact_token_limit != null
        ? String(codexConfig.model_auto_compact_token_limit)
        : ""
    );
    setSandboxModeText(codexConfig.sandbox_mode ?? "");
    setWebSearchText(codexConfig.web_search ?? "cached");
    setPersonalityText(codexConfig.personality?.trim() || "none");
    setReasoningEffortText(codexConfig.model_reasoning_effort ?? "");
    setPlanModeReasoningEffortText(codexConfig.plan_mode_reasoning_effort ?? "");
  }, [codexConfig]);

  useEffect(() => {
    const savedOverride = appSettings?.codex_home_override?.trim() ?? "";
    const savedMode =
      appSettings?.codex_home_mode ?? (savedOverride ? "custom" : "user_home_default");
    setConfigLocationMode(savedMode);
    setCustomHomeText(savedOverride);
    setConfigLocationError(null);
  }, [appSettings?.codex_home_mode, appSettings?.codex_home_override]);

  function readSavedConfigLocationState() {
    const savedOverride = appSettings?.codex_home_override?.trim() ?? "";
    const savedMode =
      appSettings?.codex_home_mode ?? (savedOverride ? "custom" : "user_home_default");
    return { savedMode, savedOverride };
  }

  function restoreSavedConfigLocationState() {
    const { savedMode, savedOverride } = readSavedConfigLocationState();
    setConfigLocationMode(savedMode);
    setCustomHomeText(savedOverride);
    setConfigLocationError(null);
  }

  const saving = codexConfigSaving;
  const loading = codexLoading || codexConfigLoading;
  const tomlBusy = codexConfigTomlLoading || codexConfigTomlSaving;
  const configLocationBusy = saving || codexHomeSettingsSaving;
  const configLocationControlsDisabled = configLocationBusy || selectingCodexHomeDir;

  async function refreshCodexStatus() {
    try {
      await refreshCodex();
    } finally {
      setVersionRefreshToken((value) => value + 1);
    }
  }

  // sandbox_mode 的本地 text 已由上方 codexConfig 整体同步 effect 更新，
  // 此处不再需要额外的 saving 守卫同步——之前的实现会在 saving 从
  // true→false 时用旧的 codexConfig 覆盖本地状态，导致 danger-full-access
  // 选择后被重置为默认值。

  const defaults = useMemo(() => {
    return {
      sandbox_mode: "workspace-write",
    };
  }, []);

  const effectiveSandboxMode = useMemo(() => {
    return enumOrDefault(sandboxModeText.trim() || null, defaults.sandbox_mode);
  }, [sandboxModeText, defaults.sandbox_mode]);

  const effectiveFastModeEnabled = useMemo(() => {
    if (!codexConfig) return false;
    return (
      boolOrDefault(codexConfig.features_fast_mode, false) ||
      codexConfig.service_tier === FAST_SERVICE_TIER
    );
  }, [codexConfig]);

  const showsGpt54LinkedSettings = useMemo(() => {
    return isGpt54Model(modelText);
  }, [modelText]);

  const configLocationPreviewPath = useMemo(() => {
    return buildConfigTomlPath(customHomeText);
  }, [customHomeText]);

  const userDefaultResolvedHomeDir = useMemo(() => {
    return codexConfig?.user_home_default_dir?.trim() || "~/.codex";
  }, [codexConfig?.user_home_default_dir]);

  const followCodexHomeResolvedDir = useMemo(() => {
    return codexConfig?.follow_codex_home_dir?.trim() || "~/.codex";
  }, [codexConfig?.follow_codex_home_dir]);

  const followModeMatchesDefault = useMemo(() => {
    return (
      normalizeComparablePath(followCodexHomeResolvedDir) ===
      normalizeComparablePath(userDefaultResolvedHomeDir)
    );
  }, [followCodexHomeResolvedDir, userDefaultResolvedHomeDir]);

  const followModeLabel = useMemo(() => {
    return followModeMatchesDefault
      ? "跟随环境变量 $CODEX_HOME（当前路径与固定目录一致）"
      : "跟随环境变量 $CODEX_HOME";
  }, [followModeMatchesDefault]);

  const configLocationBrowsePath = useMemo(() => {
    const trimmedCustomHome = customHomeText.trim();
    if (trimmedCustomHome) {
      return normalizeCustomCodexHome(trimmedCustomHome);
    }

    const savedOverride = appSettings?.codex_home_override?.trim();
    if (configLocationMode === "custom" && savedOverride) {
      return savedOverride;
    }

    if (configLocationMode === "follow_codex_home") {
      return codexConfig?.follow_codex_home_dir?.trim() || "";
    }

    return codexConfig?.user_home_default_dir?.trim() || "";
  }, [
    appSettings?.codex_home_override,
    codexConfig?.follow_codex_home_dir,
    codexConfig?.user_home_default_dir,
    configLocationMode,
    customHomeText,
  ]);

  const configLocationSummaryText = useMemo(() => {
    if (configLocationMode === "custom") {
      return customHomeText.trim()
        ? "自定义模式已启用。应用会在你指定的 .codex 目录下读写 config.toml。"
        : "自定义模式待保存。请输入一个 .codex 目录路径后按 Enter 或移出输入框保存。";
    }

    if (configLocationMode === "follow_codex_home") {
      return `跟随模式已启用。当前将使用 ${followCodexHomeResolvedDir}；如果没有设置 $CODEX_HOME，则回退到 Windows 用户目录下的 .codex，后续也会随环境变量变化。`;
    }

    return `固定模式已启用。当前固定使用 Windows 用户目录下的 .codex：${userDefaultResolvedHomeDir}；不会跟随当前的 $CODEX_HOME。`;
  }, [configLocationMode, customHomeText, followCodexHomeResolvedDir, userDefaultResolvedHomeDir]);

  const activeConfigDirSummaryText = useMemo(() => {
    if (configLocationMode === "custom") {
      return "当前为手动指定目录模式。";
    }

    if (configLocationMode === "follow_codex_home") {
      return "当前路径跟随 $CODEX_HOME 解析；后续会随环境变量变化。";
    }

    return isWindowsRuntime()
      ? "当前固定使用 Windows 用户目录下的 .codex。"
      : "当前固定使用用户主目录下的 .codex。";
  }, [configLocationMode]);

  const activeConfigModeBadgeText = useMemo(() => {
    if (configLocationMode === "custom") {
      return "手动指定";
    }

    if (configLocationMode === "follow_codex_home") {
      return "跟随变量";
    }

    return "固定目录";
  }, [configLocationMode]);

  const activeConfigDirPrimaryText = useMemo(() => {
    if (configLocationMode === "custom") {
      return customHomeText.trim() || codexConfig?.config_dir || "";
    }

    if (configLocationMode === "follow_codex_home") {
      return followCodexHomeResolvedDir;
    }

    return userDefaultResolvedHomeDir;
  }, [
    codexConfig?.config_dir,
    configLocationMode,
    customHomeText,
    followCodexHomeResolvedDir,
    userDefaultResolvedHomeDir,
  ]);

  useEffect(() => {
    const nextPath = codexConfigToml?.config_path ?? null;
    const prevPath = lastTomlConfigPathRef.current;

    if (!nextPath) {
      lastTomlConfigPathRef.current = null;
      return;
    }

    if (prevPath && prevPath !== nextPath) {
      if (validateTimerRef.current) {
        window.clearTimeout(validateTimerRef.current);
        validateTimerRef.current = null;
      }

      validateSeqRef.current += 1;
      setTomlDraft(codexConfigToml?.toml ?? "");
      setTomlDirty(false);
      setTomlValidating(false);
      setTomlValidation(null);
      setTomlEditEnabled(false);
    }

    lastTomlConfigPathRef.current = nextPath;
  }, [codexConfigToml?.config_path, codexConfigToml?.toml]);

  useEffect(() => {
    if (!codexConfigToml) return;
    if (tomlDirty) return;
    setTomlDraft(codexConfigToml.toml ?? "");
  }, [codexConfigToml, tomlDirty]);

  useEffect(() => {
    if (!tomlAdvancedOpen) return;
    if (!tomlEditEnabled) return;
    if (!tomlDirty) return;

    if (validateTimerRef.current) {
      window.clearTimeout(validateTimerRef.current);
    }

    validateTimerRef.current = window.setTimeout(() => {
      void validateToml(tomlDraft);
    }, 500);

    return () => {
      if (validateTimerRef.current) {
        window.clearTimeout(validateTimerRef.current);
        validateTimerRef.current = null;
      }
    };
  }, [tomlDraft, tomlDirty, tomlAdvancedOpen, tomlEditEnabled, validateToml]);

  async function saveTomlDraft() {
    if (tomlBusy) return;
    const result = await validateToml(tomlDraft);
    if (!result) return;
    if (!result.ok) return;

    const ok = await persistCodexConfigToml(tomlDraft);
    if (!ok) return;

    setTomlEditEnabled(false);
    setTomlDirty(false);
  }

  async function persistConfigLocation(
    nextMode: CodexHomeMode,
    nextCustomHome = customHomeText
  ): Promise<PersistConfigLocationResult> {
    if (!persistCodexHomeSettings) return "persist_failed";

    const trimmed = nextCustomHome.trim();
    const normalized = normalizeCustomCodexHome(trimmed);
    if (nextMode === "custom") {
      const error = validateCustomCodexHome(trimmed);
      setConfigLocationError(error);
      if (error) return "validation_failed";
    } else {
      setConfigLocationError(null);
    }

    const nextOverride = nextMode === "custom" ? normalized : "";
    const saved = Boolean(await persistCodexHomeSettings(nextMode, nextOverride));
    if (!saved) {
      return "persist_failed";
    }

    setConfigLocationMode(nextMode);
    setCustomHomeText(nextMode === "custom" ? nextOverride : "");
    setConfigLocationError(null);
    return "saved";
  }

  async function handleConfigLocationModeChange(nextMode: CodexHomeMode) {
    setConfigLocationMode(nextMode);

    if (nextMode !== "custom") {
      setCustomHomeText("");
      setConfigLocationError(null);
      const result = await persistConfigLocation(nextMode, "");
      if (result === "persist_failed") {
        restoreSavedConfigLocationState();
      }
      return;
    }

    const error = validateCustomCodexHome(customHomeText);
    setConfigLocationError(error);
    if (error) {
      return;
    }

    const result = await persistConfigLocation("custom", customHomeText);
    if (result === "persist_failed") {
      restoreSavedConfigLocationState();
    }
  }

  async function resetConfigLocation() {
    setConfigLocationMode("user_home_default");
    setCustomHomeText("");
    setConfigLocationError(null);
    const result = await persistConfigLocation("user_home_default", "");
    if (result === "persist_failed") {
      restoreSavedConfigLocationState();
    }
  }

  async function handlePickCustomHome() {
    if (!pickCodexHomeDirectory) return;
    if (configLocationControlsDisabled) return;

    setSelectingCodexHomeDir(true);
    try {
      const picked = await pickCodexHomeDirectory(configLocationBrowsePath || undefined);
      if (!picked) return;

      const normalized = normalizeCustomCodexHome(picked);
      setConfigLocationMode("custom");
      setCustomHomeText(normalized);

      const error = validateCustomCodexHome(normalized);
      setConfigLocationError(error);
      if (error) {
        return;
      }

      const result = await persistConfigLocation("custom", normalized);
      if (result === "persist_failed") {
        restoreSavedConfigLocationState();
      }
    } finally {
      setSelectingCodexHomeDir(false);
    }
  }

  return (
    <div className="space-y-6">
      <Card className="overflow-hidden">
        <div className="border-b border-slate-100 dark:border-slate-700">
          <div className="flex flex-col gap-4 p-6">
            <div className="flex flex-col md:flex-row items-start md:items-center justify-between gap-4">
              <div className="flex items-center gap-4">
                <div className="h-14 w-14 rounded-xl bg-slate-900/5 dark:bg-slate-700 flex items-center justify-center text-slate-700 dark:text-slate-300">
                  <Terminal className="h-8 w-8" />
                </div>
                <div>
                  <h2 className="text-xl font-bold text-slate-900 dark:text-slate-100">Codex</h2>
                  <div className="flex items-center gap-2 mt-1">
                    {codexAvailable === "available" && codexInfo?.found ? (
                      <>
                        <span className="inline-flex items-center gap-1.5 rounded-full bg-green-50 dark:bg-green-900/30 px-2.5 py-0.5 text-xs font-medium text-green-700 dark:text-green-400 ring-1 ring-inset ring-green-600/20">
                          <CheckCircle2 className="h-3 w-3" />
                          已安装 {codexInfo.version}
                        </span>
                        <CliVersionBadge
                          cliKey="codex"
                          installedVersion={codexInfo.version}
                          refreshToken={versionRefreshToken}
                          onUpdateComplete={refreshCodexStatus}
                        />
                      </>
                    ) : codexAvailable === "checking" || loading ? (
                      <span className="inline-flex items-center gap-1.5 rounded-full bg-blue-50 dark:bg-blue-900/30 px-2.5 py-0.5 text-xs font-medium text-blue-700 dark:text-blue-400 ring-1 ring-inset ring-blue-600/20">
                        <RefreshCw className="h-3 w-3 animate-spin" />
                        加载中...
                      </span>
                    ) : (
                      <span className="inline-flex items-center gap-1.5 rounded-full bg-slate-100 dark:bg-slate-700 px-2.5 py-0.5 text-xs font-medium text-slate-600 dark:text-slate-400 ring-1 ring-inset ring-slate-500/10">
                        未检测到
                      </span>
                    )}
                  </div>
                </div>
              </div>

              <Button
                onClick={() => void refreshCodexStatus()}
                variant="secondary"
                size="sm"
                disabled={loading}
                className="gap-2"
              >
                <RefreshCw className={cn("h-3.5 w-3.5", loading && "animate-spin")} />
                刷新
              </Button>
            </div>

            {codexConfig && (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-3 mt-2">
                <div className="bg-slate-50 dark:bg-slate-800 rounded-lg p-3 border border-slate-100 dark:border-slate-700">
                  <div className="flex items-center gap-1.5 text-xs text-slate-500 dark:text-slate-400 mb-1.5">
                    <FolderOpen className="h-3 w-3" />
                    当前 .codex 目录
                  </div>
                  <div className="flex items-center gap-1.5">
                    <div
                      className="font-mono text-xs text-slate-700 dark:text-slate-300 truncate flex-1"
                      title={codexConfig.config_dir}
                    >
                      {codexConfig.config_dir}
                    </div>
                    <Button
                      onClick={() => void openCodexConfigDir()}
                      disabled={!codexConfig.can_open_config_dir}
                      size="sm"
                      variant="ghost"
                      className="shrink-0 h-6 w-6 p-0 hover:bg-slate-200 dark:hover:bg-slate-700"
                      title={
                        codexConfig.can_open_config_dir
                          ? "打开当前生效目录"
                          : "受权限限制，无法自动打开该目录"
                      }
                    >
                      <ExternalLink className="h-3 w-3" />
                    </Button>
                  </div>
                  <div className="mt-1 text-[11px] text-slate-500 dark:text-slate-400">
                    {activeConfigDirSummaryText}
                  </div>
                  {!codexConfig.can_open_config_dir ? (
                    <div className="mt-1 text-[11px] text-amber-700 dark:text-amber-400">
                      受权限限制，无法自动打开该目录；请手动打开该路径。
                    </div>
                  ) : null}
                </div>

                <div className="bg-slate-50 dark:bg-slate-800 rounded-lg p-3 border border-slate-100 dark:border-slate-700">
                  <div className="flex items-center gap-1.5 text-xs text-slate-500 dark:text-slate-400 mb-1.5">
                    <FileJson className="h-3 w-3" />
                    config.toml
                  </div>
                  <div
                    className="font-mono text-xs text-slate-700 dark:text-slate-300 truncate"
                    title={codexConfig.config_path}
                  >
                    {codexConfig.config_path}
                  </div>
                  <div className="mt-1 text-[11px] text-slate-500 dark:text-slate-400">
                    {codexConfig.exists ? "已存在" : "不存在（将自动创建）"}
                  </div>
                </div>

                <div className="bg-slate-50 dark:bg-slate-800 rounded-lg p-3 border border-slate-100 dark:border-slate-700">
                  <div className="flex items-center gap-1.5 text-xs text-slate-500 dark:text-slate-400 mb-1.5">
                    <Terminal className="h-3 w-3" />
                    可执行文件
                  </div>
                  <div
                    className="font-mono text-xs text-slate-700 dark:text-slate-300 truncate"
                    title={codexInfo?.executable_path ?? "—"}
                  >
                    {codexInfo?.executable_path ?? "—"}
                  </div>
                </div>

                <div className="bg-slate-50 dark:bg-slate-800 rounded-lg p-3 border border-slate-100 dark:border-slate-700">
                  <div className="flex items-center gap-1.5 text-xs text-slate-500 dark:text-slate-400 mb-1.5">
                    <Settings className="h-3 w-3" />
                    解析方式
                  </div>
                  <div
                    className="font-mono text-xs text-slate-700 dark:text-slate-300 truncate"
                    title={codexInfo?.resolved_via ?? "—"}
                  >
                    {codexInfo?.resolved_via ?? "—"}
                  </div>
                  <div className="mt-1 text-[11px] text-slate-500 dark:text-slate-400">
                    SHELL: {codexInfo?.shell ?? "—"}
                  </div>
                </div>
              </div>
            )}

            {codexConfig && isWindowsRuntime() ? (
              <div className="rounded-xl border border-slate-200/80 bg-white/80 p-4 dark:border-slate-700 dark:bg-slate-900/20">
                <div className="flex flex-col gap-4">
                  <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
                    <div className="min-w-0">
                      <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">
                        Windows 本机配置
                      </div>
                      <div className="mt-1 text-xs leading-relaxed text-slate-500 dark:text-slate-400">
                        仅影响 Windows 本机上的 Codex 用户级{" "}
                        <span className="font-mono">.codex</span> 目录，不改写 WSL 内各 distro 的
                        目标路径。
                      </div>
                    </div>

                    <div className="flex items-center gap-2">
                      <span className="inline-flex items-center rounded-full border border-slate-200 bg-slate-50 px-2.5 py-1 text-[11px] font-medium text-slate-700 dark:border-slate-600 dark:bg-slate-800 dark:text-slate-200">
                        {activeConfigModeBadgeText}
                      </span>
                      <Button
                        type="button"
                        size="sm"
                        variant="ghost"
                        onClick={resetConfigLocation}
                        disabled={
                          configLocationControlsDisabled ||
                          (configLocationMode === "user_home_default" &&
                            customHomeText.trim().length === 0)
                        }
                      >
                        恢复默认
                      </Button>
                    </div>
                  </div>

                  <div className="rounded-lg border border-slate-200/70 bg-slate-50/80 p-3 dark:border-slate-700 dark:bg-slate-800/80">
                    <div className="flex flex-col gap-2 md:flex-row md:items-start md:justify-between">
                      <div className="min-w-0">
                        <div className="text-[11px] uppercase tracking-wide text-slate-500 dark:text-slate-400">
                          当前会使用
                        </div>
                        <div
                          className="mt-1 break-all font-mono text-xs text-slate-700 dark:text-slate-300"
                          title={activeConfigDirPrimaryText}
                        >
                          {activeConfigDirPrimaryText}
                        </div>
                        <div className="mt-1 text-[11px] leading-relaxed text-slate-500 dark:text-slate-400">
                          {configLocationSummaryText}
                        </div>
                      </div>

                      <div className="min-w-0 md:max-w-[320px]">
                        <div className="text-[11px] uppercase tracking-wide text-slate-500 dark:text-slate-400">
                          config.toml
                        </div>
                        <div
                          className="mt-1 break-all font-mono text-xs text-slate-700 dark:text-slate-300"
                          title={codexConfig.config_path}
                        >
                          {codexConfig.config_path}
                        </div>
                        <div className="mt-1 text-[11px] text-slate-500 dark:text-slate-400">
                          {activeConfigDirSummaryText}
                        </div>
                      </div>
                    </div>
                  </div>

                  <div className="rounded-lg border border-slate-200/70 bg-white/70 p-3 dark:border-slate-700 dark:bg-slate-900/20">
                    <div className="text-[11px] uppercase tracking-wide text-slate-500 dark:text-slate-400">
                      目录来源
                    </div>
                    <div className="mt-2">
                      <RadioGroup
                        name="codex_config_location_mode"
                        value={configLocationMode}
                        onChange={(value) =>
                          handleConfigLocationModeChange(
                            value === "follow_codex_home"
                              ? "follow_codex_home"
                              : value === "custom"
                                ? "custom"
                                : "user_home_default"
                          )
                        }
                        options={[
                          { value: "user_home_default", label: "固定到 Windows 用户目录" },
                          { value: "follow_codex_home", label: followModeLabel },
                          { value: "custom", label: "手动指定目录" },
                        ]}
                        disabled={configLocationControlsDisabled}
                      />
                    </div>
                    <div className="mt-2 space-y-1 text-[11px] leading-relaxed text-slate-500 dark:text-slate-400">
                      <div>
                        固定目录：
                        <span className="ml-1 font-mono">{userDefaultResolvedHomeDir}</span>
                      </div>
                      <div>
                        <span className="font-mono">$CODEX_HOME</span> 当前解析：
                        <span className="ml-1 font-mono">{followCodexHomeResolvedDir}</span>
                        {followModeMatchesDefault ? (
                          <span className="ml-2 text-amber-700 dark:text-amber-400">
                            当前路径相同，但后续会随 $CODEX_HOME 变化。
                          </span>
                        ) : null}
                      </div>
                    </div>
                  </div>

                  {configLocationMode === "custom" ? (
                    <div className="rounded-lg border border-slate-200/70 bg-slate-50/80 p-3 dark:border-slate-700 dark:bg-slate-800/80">
                      <label className="text-xs font-medium text-slate-700 dark:text-slate-300">
                        自定义 .codex 目录
                      </label>

                      <div className="mt-3 flex flex-col gap-2 lg:flex-row">
                        <Input
                          value={customHomeText}
                          onChange={(e) => {
                            const next = e.currentTarget.value;
                            setCustomHomeText(next);
                            if (configLocationError) {
                              setConfigLocationError(validateCustomCodexHome(next));
                            }
                          }}
                          onBlur={() => {
                            if (configLocationMode !== "custom") return;
                            void persistConfigLocation("custom", customHomeText).then((result) => {
                              if (result === "persist_failed") {
                                restoreSavedConfigLocationState();
                              }
                            });
                          }}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") e.currentTarget.blur();
                          }}
                          placeholder="例如：D:\\Users\\you\\.codex"
                          className={cn(
                            "font-mono text-xs lg:flex-1",
                            configLocationError &&
                              "border-rose-300 focus-visible:ring-rose-200 dark:border-rose-700"
                          )}
                          disabled={configLocationControlsDisabled}
                        />

                        <div className="flex flex-wrap gap-2">
                          <Button
                            type="button"
                            size="sm"
                            variant="secondary"
                            onClick={() => void handlePickCustomHome()}
                            disabled={configLocationControlsDisabled}
                          >
                            <FolderOpen className="mr-1.5 h-3.5 w-3.5" />
                            {selectingCodexHomeDir ? "选择中..." : "选择目录"}
                          </Button>
                        </div>
                      </div>

                      <div
                        className={cn(
                          "mt-2 text-[11px] leading-relaxed",
                          configLocationError
                            ? "text-rose-600 dark:text-rose-400"
                            : "text-slate-500 dark:text-slate-400"
                        )}
                      >
                        {configLocationError
                          ? configLocationError
                          : configLocationPreviewPath
                            ? `保存后将使用 ${configLocationPreviewPath}。支持普通 Windows 路径、UNC 路径，也可以点“选择目录”。`
                            : "请输入一个 .codex 目录路径，然后按 Enter、移出输入框，或直接使用目录选择器保存。"}
                      </div>
                    </div>
                  ) : (
                    <div className="rounded-lg border border-dashed border-slate-200/80 bg-slate-50/50 px-3 py-2 text-[11px] leading-relaxed text-slate-500 dark:border-slate-700 dark:bg-slate-800/40 dark:text-slate-400">
                      {configLocationMode === "follow_codex_home"
                        ? `当前为跟随模式，手动目录选择器已收起；现在会使用 ${followCodexHomeResolvedDir}。`
                        : `当前为默认模式，手动目录选择器已收起；固定使用 ${userDefaultResolvedHomeDir}。`}
                    </div>
                  )}
                </div>
              </div>
            ) : null}

            <div className="text-xs text-slate-500 dark:text-slate-400">
              提示：Codex 还会读取 Team Config（例如 repo 内 `.codex/`），其优先级可能高于这里的
              用户级目录设置。
            </div>
          </div>
        </div>

        {codexAvailable === "unavailable" ? (
          <div className="text-sm text-slate-600 dark:text-slate-400 text-center py-8">
            数据不可用
          </div>
        ) : !codexConfig ? (
          <div className="text-sm text-slate-500 dark:text-slate-400 text-center py-8">
            暂无配置，请尝试刷新
          </div>
        ) : (
          <div className="p-6 space-y-6">
            <div className="rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 p-5">
              <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100 flex items-center gap-2 mb-3">
                <Settings className="h-4 w-4 text-slate-400 dark:text-slate-500" />
                基础配置
              </h3>
              <div className="divide-y divide-slate-100 dark:divide-slate-700">
                <SettingItem
                  label="默认模型 (model)"
                  subtitle="设置 Codex 默认使用的模型（例如 gpt-5-codex）。留空表示不设置（交由 Codex 默认/上层配置决定）。"
                >
                  <Input
                    value={modelText}
                    onChange={(e) => setModelText(e.currentTarget.value)}
                    onBlur={() =>
                      void persistCodexConfig(
                        buildModelPatch(modelText, contextWindowText, autoCompactLimitText)
                      )
                    }
                    placeholder="例如：gpt-5-codex"
                    className="font-mono w-[280px] max-w-full"
                    disabled={saving}
                  />
                </SettingItem>

                {showsGpt54LinkedSettings ? (
                  <>
                    <SettingItem
                      label="model_context_window"
                      subtitle={`模型上下文窗口大小。仅当 model=${GPT_54_MODEL} 时生效；切换到其他模型时自动删除。留空则不写入配置，默认参考值 ${GPT_54_CONTEXT_WINDOW.toLocaleString()}。`}
                    >
                      <Input
                        type="number"
                        value={contextWindowText}
                        onChange={(e) => setContextWindowText(e.currentTarget.value)}
                        onBlur={() =>
                          void persistCodexConfig({
                            model_context_window: parsePositiveInt(contextWindowText),
                          })
                        }
                        placeholder={String(GPT_54_CONTEXT_WINDOW)}
                        className="font-mono w-[220px] max-w-full"
                        disabled={saving}
                      />
                    </SettingItem>

                    <SettingItem
                      label="model_auto_compact_token_limit"
                      subtitle={`自动压缩 token 上限。仅当 model=${GPT_54_MODEL} 时生效；切换到其他模型时自动删除。留空则不写入配置，默认参考值 ${GPT_54_AUTO_COMPACT_TOKEN_LIMIT.toLocaleString()}。`}
                    >
                      <Input
                        type="number"
                        value={autoCompactLimitText}
                        onChange={(e) => setAutoCompactLimitText(e.currentTarget.value)}
                        onBlur={() =>
                          void persistCodexConfig({
                            model_auto_compact_token_limit: parsePositiveInt(autoCompactLimitText),
                          })
                        }
                        placeholder={String(GPT_54_AUTO_COMPACT_TOKEN_LIMIT)}
                        className="font-mono w-[220px] max-w-full"
                        disabled={saving}
                      />
                    </SettingItem>
                  </>
                ) : null}

                <SettingItem
                  label="审批策略 (approval_policy)"
                  subtitle="控制何时需要你确认才会执行命令。推荐 on-request（默认）或 on-failure。"
                >
                  <Select
                    value={codexConfig.approval_policy ?? ""}
                    onChange={(e) =>
                      void persistCodexConfig({ approval_policy: e.currentTarget.value })
                    }
                    disabled={saving}
                    className="w-[220px] max-w-full font-mono"
                  >
                    <option value="">默认（不设置）</option>
                    <option value="untrusted">不信任（untrusted）</option>
                    <option value="on-failure">失败时（on-failure）</option>
                    <option value="on-request">请求时（on-request）</option>
                    <option value="never">从不询问（never）</option>
                  </Select>
                </SettingItem>

                <SettingItem
                  label="沙箱模式 (sandbox_mode)"
                  subtitle="控制文件/网络访问策略。danger-full-access 风险极高，仅在完全信任的环境使用。"
                >
                  <Select
                    value={sandboxModeText}
                    onChange={(e) => {
                      const next = e.currentTarget.value;
                      if (next === "danger-full-access") {
                        const ok = window.confirm(
                          "你选择了 danger-full-access（危险：完全访问）。确认要继续吗？"
                        );
                        if (!ok) {
                          setSandboxModeText(codexConfig.sandbox_mode ?? "");
                          return;
                        }
                      }
                      setSandboxModeText(next);
                      void persistCodexConfig({ sandbox_mode: next });
                    }}
                    disabled={saving}
                    className="w-[220px] max-w-full font-mono"
                  >
                    <option value="">默认（不设置）</option>
                    <option value="read-only">只读（read-only）</option>
                    <option value="workspace-write">工作区写入（workspace-write）</option>
                    <option value="danger-full-access">危险：完全访问（danger-full-access）</option>
                  </Select>
                </SettingItem>

                <SettingItem
                  label="推理强度 (model_reasoning_effort)"
                  subtitle="调整推理强度（仅对支持的模型/Responses API 生效）。值越高通常越稳健但更慢。"
                >
                  <RadioGroup
                    name="model_reasoning_effort"
                    value={reasoningEffortText}
                    onChange={(value) => {
                      setReasoningEffortText(value);
                      void persistCodexConfig({ model_reasoning_effort: value });
                    }}
                    options={[
                      { value: "", label: "默认" },
                      { value: "minimal", label: "最低 (minimal)" },
                      { value: "low", label: "低 (low)" },
                      { value: "medium", label: "中 (medium)" },
                      { value: "high", label: "高 (high)" },
                      { value: "xhigh", label: "极高 (xhigh)" },
                    ]}
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem
                  label="计划模式推理强度 (plan_mode_reasoning_effort)"
                  subtitle="调整计划模式下的推理强度。值越高通常规划越充分但更慢。"
                >
                  <RadioGroup
                    name="plan_mode_reasoning_effort"
                    value={planModeReasoningEffortText}
                    onChange={(value) => {
                      setPlanModeReasoningEffortText(value);
                      void persistCodexConfig({ plan_mode_reasoning_effort: value });
                    }}
                    options={[
                      { value: "", label: "默认" },
                      { value: "low", label: "低 (low)" },
                      { value: "medium", label: "中 (medium)" },
                      { value: "high", label: "高 (high)" },
                      { value: "xhigh", label: "极高 (xhigh)" },
                    ]}
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem
                  label="网络搜索模式 (web_search)"
                  subtitle="控制 Web Search 工具的行为。cached：使用缓存结果；live：获取最新数据；disabled：禁用。"
                >
                  <RadioGroup
                    name="web_search"
                    value={webSearchText}
                    onChange={(value) => {
                      setWebSearchText(value);
                      void persistCodexConfig({ web_search: value });
                    }}
                    options={[
                      { value: "cached", label: "缓存 (cached)" },
                      { value: "live", label: "实时 (live)" },
                      { value: "disabled", label: "禁用 (disabled)" },
                    ]}
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem
                  label="输出风格 (personality)"
                  subtitle="控制 web_search 结果的输出风格。pragmatic 更务实，friendly 更友好；none 会删除该配置，交给 Codex 默认行为。"
                >
                  <RadioGroup
                    name="personality"
                    value={personalityText}
                    onChange={(value) => {
                      setPersonalityText(value);
                      void persistCodexConfig(buildPersonalityPatch(value));
                    }}
                    options={[
                      { value: "pragmatic", label: "务实 (pragmatic)" },
                      { value: "friendly", label: "友好 (friendly)" },
                      { value: "none", label: "默认 / 删除配置 (none)" },
                    ]}
                    disabled={saving}
                  />
                </SettingItem>
              </div>
            </div>

            <div className="rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 p-5">
              <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100 flex items-center gap-2 mb-3">
                <Settings className="h-4 w-4 text-slate-400 dark:text-slate-500" />
                Sandbox（workspace-write）
              </h3>
              <div className="divide-y divide-slate-100 dark:divide-slate-700">
                <SettingItem
                  label="允许联网 (sandbox_workspace_write.network_access)"
                  subtitle="仅在 sandbox_mode=workspace-write 时生效。开启写入 network_access=true；关闭删除该项（不写 false）。"
                >
                  <Switch
                    checked={boolOrDefault(
                      codexConfig.sandbox_workspace_write_network_access,
                      false
                    )}
                    onCheckedChange={(checked) =>
                      void persistCodexConfig({ sandbox_workspace_write_network_access: checked })
                    }
                    disabled={saving}
                  />
                </SettingItem>
              </div>
              {effectiveSandboxMode !== "workspace-write" ? (
                <div className="mt-3 rounded-lg bg-amber-50 dark:bg-amber-900/30 p-3 text-xs text-amber-700 dark:text-amber-400 flex items-start gap-2">
                  <AlertTriangle className="h-4 w-4 shrink-0 mt-0.5" />
                  <div>
                    当前 sandbox_mode 不是 <span className="font-mono">workspace-write</span>
                    ，此分区设置可能不会生效。
                  </div>
                </div>
              ) : null}
            </div>

            <div className="rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 p-5">
              <h3 className="text-sm font-semibold text-slate-900 dark:text-slate-100 flex items-center gap-2 mb-3">
                <Settings className="h-4 w-4 text-slate-400 dark:text-slate-500" />
                Features（实验/可选能力）
              </h3>
              <div className="divide-y divide-slate-100 dark:divide-slate-700">
                <SettingItem
                  label="shell_snapshot"
                  subtitle="测试版：快照 shell 环境以加速重复命令。开启写入 shell_snapshot=true；"
                >
                  <Switch
                    checked={boolOrDefault(codexConfig.features_shell_snapshot, false)}
                    onCheckedChange={(checked) =>
                      void persistCodexConfig({ features_shell_snapshot: checked })
                    }
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem
                  label="unified_exec"
                  subtitle="测试版：使用统一的、基于 PTY 的 exec 工具。开启写入 unified_exec=true；"
                >
                  <Switch
                    checked={boolOrDefault(codexConfig.features_unified_exec, false)}
                    onCheckedChange={(checked) =>
                      void persistCodexConfig({ features_unified_exec: checked })
                    }
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem
                  label="shell_tool"
                  subtitle="稳定：启用默认 shell 工具。开启写入 shell_tool=true；"
                >
                  <Switch
                    checked={boolOrDefault(codexConfig.features_shell_tool, false)}
                    onCheckedChange={(checked) =>
                      void persistCodexConfig({ features_shell_tool: checked })
                    }
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem
                  label="exec_policy"
                  subtitle="实验性：对 shell/unified_exec 强制执行规则检查。开启写入 exec_policy=true；"
                >
                  <Switch
                    checked={boolOrDefault(codexConfig.features_exec_policy, false)}
                    onCheckedChange={(checked) =>
                      void persistCodexConfig({ features_exec_policy: checked })
                    }
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem
                  label="apply_patch_freeform"
                  subtitle="实验性：启用自由格式 apply_patch 工具。开启写入 apply_patch_freeform=true；"
                >
                  <Switch
                    checked={boolOrDefault(codexConfig.features_apply_patch_freeform, false)}
                    onCheckedChange={(checked) =>
                      void persistCodexConfig({ features_apply_patch_freeform: checked })
                    }
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem
                  label="remote_compaction"
                  subtitle="实验性：启用 remote compaction（需要 ChatGPT 身份验证）。开启写入 remote_compaction=true；"
                >
                  <Switch
                    checked={boolOrDefault(codexConfig.features_remote_compaction, false)}
                    onCheckedChange={(checked) =>
                      void persistCodexConfig({ features_remote_compaction: checked })
                    }
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem
                  label="fast_mode"
                  subtitle={
                    '实验性：启用快速模式。开启同时写入 fast_mode=true 与 service_tier="fast"；关闭删除这两项。'
                  }
                >
                  <Switch
                    checked={effectiveFastModeEnabled}
                    onCheckedChange={(checked) =>
                      void persistCodexConfig(buildFastModePatch(checked))
                    }
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem
                  label="responses_websockets_v2"
                  subtitle="实验性：启用 Responses API websocket 支持（需要中转站支持）。开启写入 responses_websockets_v2=true；关闭删除该项。"
                >
                  <Switch
                    checked={boolOrDefault(codexConfig.features_responses_websockets_v2, false)}
                    onCheckedChange={(checked) =>
                      void persistCodexConfig({ features_responses_websockets_v2: checked })
                    }
                    disabled={saving}
                  />
                </SettingItem>

                <SettingItem
                  label="multi_agent"
                  subtitle="实验性：通过并行生成多个专门化代理来协作完成复杂任务，最后整合结果。开启写入 multi_agent=true；"
                >
                  <Switch
                    checked={boolOrDefault(codexConfig.features_multi_agent, false)}
                    onCheckedChange={(checked) =>
                      void persistCodexConfig({ features_multi_agent: checked })
                    }
                    disabled={saving}
                  />
                </SettingItem>
              </div>
            </div>

            <div className="rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 p-5">
              <details
                className="group"
                onToggle={(e) => setTomlAdvancedOpen((e.currentTarget as HTMLDetailsElement).open)}
              >
                <summary className="cursor-pointer select-none text-sm font-semibold text-slate-900 dark:text-slate-100 flex items-center justify-between">
                  <span className="flex items-center gap-2">
                    <Settings className="h-4 w-4 text-slate-400 dark:text-slate-500" />
                    高级配置（config.toml）
                  </span>
                  <span className="text-xs font-normal text-slate-500 dark:text-slate-400">
                    仅在需要编辑原始 TOML 时使用
                  </span>
                </summary>

                {tomlAdvancedOpen ? (
                  <div className="mt-4 space-y-3">
                    <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
                      <div className="min-w-0">
                        <div className="text-xs text-slate-500 dark:text-slate-400">路径</div>
                        <div className="mt-1 font-mono text-xs text-slate-700 dark:text-slate-300 truncate">
                          {codexConfig?.config_path ?? codexConfigToml?.config_path ?? "—"}
                        </div>
                      </div>
                      <div className="flex items-center justify-end gap-2">
                        <Button
                          type="button"
                          size="sm"
                          variant="ghost"
                          onClick={() => {
                            setTomlDraft(codexConfigToml?.toml ?? "");
                            setTomlDirty(false);
                            setTomlValidation(null);
                          }}
                          disabled={tomlBusy || tomlEditEnabled}
                        >
                          重新加载
                        </Button>

                        {!tomlEditEnabled ? (
                          <Button
                            type="button"
                            size="sm"
                            onClick={() => {
                              setTomlEditEnabled(true);
                              setTomlDraft(codexConfigToml?.toml ?? "");
                              setTomlDirty(false);
                              setTomlValidation(null);
                              void validateToml(codexConfigToml?.toml ?? "");
                            }}
                            disabled={tomlBusy}
                          >
                            编辑
                          </Button>
                        ) : (
                          <>
                            <Button
                              type="button"
                              size="sm"
                              variant="ghost"
                              onClick={() => {
                                setTomlEditEnabled(false);
                                setTomlDraft(codexConfigToml?.toml ?? "");
                                setTomlDirty(false);
                                setTomlValidation(null);
                              }}
                              disabled={tomlBusy}
                            >
                              取消
                            </Button>
                            <Button
                              type="button"
                              size="sm"
                              onClick={() => void saveTomlDraft()}
                              disabled={
                                tomlBusy ||
                                tomlValidating ||
                                !tomlDirty ||
                                (tomlValidation ? !tomlValidation.ok : false)
                              }
                            >
                              {tomlValidating ? "校验中…" : "保存"}
                            </Button>
                          </>
                        )}
                      </div>
                    </div>

                    {codexConfigTomlLoading ? (
                      <div className="text-sm text-slate-500 dark:text-slate-400 py-6 text-center">
                        加载中…
                      </div>
                    ) : (
                      <Suspense
                        fallback={
                          <div className="text-sm text-slate-500 dark:text-slate-400 py-6 text-center">
                            加载编辑器…
                          </div>
                        }
                      >
                        <LazyCodeEditor
                          value={tomlDraft}
                          onChange={
                            tomlEditEnabled
                              ? (next) => {
                                  setTomlDraft(next);
                                  setTomlDirty(true);
                                }
                              : undefined
                          }
                          readOnly={!tomlEditEnabled || tomlBusy}
                          language="toml"
                          minHeight="260px"
                          placeholder='例如：approval_policy = "on-request"'
                        />
                      </Suspense>
                    )}

                    {tomlValidation?.ok === false && tomlValidation.error ? (
                      <div className="rounded-lg bg-rose-50 dark:bg-rose-900/30 p-3 text-xs text-rose-700 dark:text-rose-400 flex items-start gap-2">
                        <AlertTriangle className="h-4 w-4 shrink-0 mt-0.5" />
                        <div className="min-w-0">
                          <div className="font-semibold">TOML 校验失败</div>
                          <div className="mt-1 break-words">
                            {tomlValidation.error.message}
                            {tomlValidation.error.line ? (
                              <span className="ml-2 font-mono text-rose-600">
                                (line {tomlValidation.error.line}
                                {tomlValidation.error.column
                                  ? `, column ${tomlValidation.error.column}`
                                  : ""}
                                )
                              </span>
                            ) : null}
                          </div>
                        </div>
                      </div>
                    ) : (
                      <div className="text-xs text-slate-500 dark:text-slate-400">
                        保存前会进行后端 TOML 校验；校验失败不会写入文件。
                      </div>
                    )}
                  </div>
                ) : null}
              </details>
            </div>
          </div>
        )}

        {codexInfo?.error && (
          <div className="mt-4 rounded-lg bg-rose-50 dark:bg-rose-900/30 p-4 text-sm text-rose-600 dark:text-rose-400 flex items-start gap-2">
            <AlertTriangle className="h-5 w-5 shrink-0" />
            <div>
              <span className="font-semibold">检测失败：</span>
              {codexInfo.error}
            </div>
          </div>
        )}
      </Card>
    </div>
  );
}
