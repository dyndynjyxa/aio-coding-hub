// Usage: Used by ProvidersView to create/edit a Provider with toast-based validation.

import { useEffect, useRef, useState } from "react";
import {
  ChevronDown,
  Clock,
  DollarSign,
  CalendarDays,
  CalendarRange,
  Gauge,
  RotateCcw,
  X,
} from "lucide-react";
import { toast } from "sonner";
import { cliLongLabel } from "../../constants/clis";
import { FREE_TAG } from "../../constants/providers";
import { copyText } from "../../services/clipboard";
import { logToConsole } from "../../services/consoleLog";
import {
  providerGetApiKey,
  providerUpsert,
  providerDelete,
  providerOAuthStartFlow,
  providerOAuthRefresh,
  providerOAuthDisconnect,
  providerOAuthStatus,
  providerOAuthFetchLimits,
  type ClaudeModels,
  type CliKey,
  type ProviderSummary,
} from "../../services/providers/providers";
import {
  createProviderEditorDialogSchema,
  type ProviderEditorDialogFormInput,
  type ProviderEditorDialogFormOutput,
} from "../../schemas/providerEditorDialog";
import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";
import { FormField } from "../../ui/FormField";
import { Input } from "../../ui/Input";
import { Switch } from "../../ui/Switch";
import { TabList } from "../../ui/TabList";
import { normalizeBaseUrlRows } from "./baseUrl";
import { formatUnixSeconds } from "../../utils/formatters";
import { BaseUrlEditor } from "./BaseUrlEditor";
import { LimitCard } from "./LimitCard";
import { RadioButtonGroup } from "./RadioButtonGroup";
import type { ProviderEditorInitialValues } from "./providerDuplicate";
import type { BaseUrlRow, ProviderBaseUrlMode } from "./types";
import { validateProviderClaudeModels } from "./validators";
import { useForm } from "react-hook-form";

type DailyResetMode = "fixed" | "rolling";
type ProviderEditorDialogBaseProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSaved: (cliKey: CliKey) => void;
  codexProviders?: ProviderSummary[];
};

export type ProviderEditorDialogProps =
  | (ProviderEditorDialogBaseProps & {
      mode: "create";
      cliKey: CliKey;
      initialValues?: ProviderEditorInitialValues | null;
    })
  | (ProviderEditorDialogBaseProps & {
      mode: "edit";
      provider: ProviderSummary;
    });

function cliNameFromKey(cliKey: CliKey) {
  return cliLongLabel(cliKey);
}

const DEFAULT_FORM_VALUES: ProviderEditorDialogFormInput = {
  name: "",
  api_key: "",
  auth_mode: "api_key",
  cost_multiplier: "1.0",
  limit_5h_usd: "",
  limit_daily_usd: "",
  limit_weekly_usd: "",
  limit_monthly_usd: "",
  limit_total_usd: "",
  daily_reset_mode: "fixed",
  daily_reset_time: "00:00:00",
  enabled: true,
  note: "",
};

function valueOrEmpty(value: number | null | undefined) {
  return value != null ? String(value) : "";
}

function isZeroMultiplier(value: string | null | undefined) {
  if (!value?.trim()) return false;
  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed === 0;
}

function isNonZeroMultiplier(value: string | null | undefined) {
  if (!value?.trim()) return false;
  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed !== 0;
}

function moveFreeTagToFront(tags: string[]) {
  const withoutFreeTag = tags.filter((tag) => tag !== FREE_TAG);
  return [FREE_TAG, ...withoutFreeTag];
}

function areTagsEqual(left: string[], right: string[]) {
  if (left.length !== right.length) return false;
  return left.every((tag, index) => tag === right[index]);
}

function tagBadgeClassName(tag: string) {
  if (tag === FREE_TAG) {
    return "inline-flex items-center gap-1 rounded-full bg-emerald-100 px-2 py-0.5 text-xs font-medium text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-300";
  }
  return "inline-flex items-center gap-1 rounded-full bg-accent/10 px-2 py-0.5 text-xs font-medium text-accent";
}

function tagRemoveButtonClassName(tag: string) {
  if (tag === FREE_TAG) {
    return "inline-flex h-3.5 w-3.5 items-center justify-center rounded-full hover:bg-emerald-200/70 dark:hover:bg-emerald-800/60";
  }
  return "inline-flex h-3.5 w-3.5 items-center justify-center rounded-full hover:bg-accent/20";
}

function buildFormValues(initialValues: ProviderEditorInitialValues | null) {
  if (!initialValues) {
    return { ...DEFAULT_FORM_VALUES };
  }

  return {
    name: initialValues.name,
    api_key: initialValues.api_key,
    auth_mode: initialValues.auth_mode,
    cost_multiplier: String(initialValues.cost_multiplier),
    limit_5h_usd: valueOrEmpty(initialValues.limit_5h_usd),
    limit_daily_usd: valueOrEmpty(initialValues.limit_daily_usd),
    limit_weekly_usd: valueOrEmpty(initialValues.limit_weekly_usd),
    limit_monthly_usd: valueOrEmpty(initialValues.limit_monthly_usd),
    limit_total_usd: valueOrEmpty(initialValues.limit_total_usd),
    daily_reset_mode: initialValues.daily_reset_mode,
    daily_reset_time: initialValues.daily_reset_time,
    enabled: initialValues.enabled,
    note: initialValues.note,
  };
}

function buildBaseUrlRows(
  initialValues: ProviderEditorInitialValues | null,
  newBaseUrlRow: (url?: string) => BaseUrlRow
) {
  const baseUrls = initialValues?.base_urls ?? [];
  if (baseUrls.length > 0) {
    return baseUrls.map((url) => newBaseUrlRow(url));
  }
  if (initialValues?.auth_mode === "oauth") {
    return [] as BaseUrlRow[];
  }
  return [newBaseUrlRow()];
}

function deriveAuthMode(
  provider: ProviderSummary | null | undefined
): "api_key" | "oauth" | "cx2cc" {
  if (!provider) return "api_key";
  if (provider.source_provider_id) return "cx2cc";
  if (provider.auth_mode === "oauth") return "oauth";
  return "api_key";
}

export function ProviderEditorDialog(props: ProviderEditorDialogProps) {
  const { open, onOpenChange, onSaved, codexProviders = [] } = props;

  const mode = props.mode;
  const cliKey = mode === "create" ? props.cliKey : props.provider.cli_key;
  const createInitialValues = mode === "create" ? (props.initialValues ?? null) : null;
  const isDuplicating = mode === "create" && createInitialValues != null;
  const editingProviderId = mode === "edit" ? props.provider.id : null;
  const editProvider = mode === "edit" ? props.provider : null;

  const baseUrlRowSeqRef = useRef(1);
  const newBaseUrlRow = (url = ""): BaseUrlRow => {
    const id = String(baseUrlRowSeqRef.current++);
    return { id, url, ping: { status: "idle" } };
  };

  const [baseUrlMode, setBaseUrlMode] = useState<ProviderBaseUrlMode>("order");
  const [baseUrlRows, setBaseUrlRows] = useState<BaseUrlRow[]>(() => [newBaseUrlRow()]);
  const [pingingAll, setPingingAll] = useState(false);
  const [claudeModels, setClaudeModels] = useState<ClaudeModels>({});
  const [tags, setTags] = useState<string[]>([]);
  const [tagInput, setTagInput] = useState("");
  const [streamIdleTimeoutSeconds, setStreamIdleTimeoutSeconds] = useState("");
  const [saving, setSaving] = useState(false);
  const [fetchingApiKey, setFetchingApiKey] = useState(false);
  const [savedApiKey, setSavedApiKey] = useState<string | null>(null);
  const apiKeyFetchedRef = useRef(false);
  const apiKeyFetchPromiseRef = useRef<Promise<string | null> | null>(null);
  const apiKeyFetchErrorRef = useRef(false);
  const apiKeyRequestSeqRef = useRef(0);

  const [authMode, setAuthMode] = useState<"api_key" | "oauth" | "cx2cc">(
    deriveAuthMode(editProvider)
  );

  const [sourceProviderId, setSourceProviderId] = useState<number | null>(
    editProvider?.source_provider_id ?? null
  );

  const [oauthStatus, setOauthStatus] = useState<{
    connected: boolean;
    provider_type?: string;
    email?: string;
    expires_at?: number;
    has_refresh_token?: boolean;
  } | null>(null);
  const [oauthLoading, setOauthLoading] = useState(false);
  const oauthStatusRequestSeqRef = useRef(0);

  const form = useForm<ProviderEditorDialogFormInput>({
    defaultValues: DEFAULT_FORM_VALUES,
  });
  const editProviderSnapshotRef = useRef<ProviderSummary | null>(null);

  const { register, reset, setValue, watch, formState } = form;
  const enabled = watch("enabled");
  const dailyResetMode = watch("daily_reset_mode");
  const limit5hUsd = watch("limit_5h_usd");
  const limitDailyUsd = watch("limit_daily_usd");
  const limitWeeklyUsd = watch("limit_weekly_usd");
  const limitMonthlyUsd = watch("limit_monthly_usd");
  const limitTotalUsd = watch("limit_total_usd");
  const apiKeyValue = watch("api_key");
  const costMultiplierValue = watch("cost_multiplier");
  const apiKeyDirty = Boolean(formState.dirtyFields.api_key);

  const title =
    mode === "create"
      ? `${cliNameFromKey(cliKey)} · ${isDuplicating ? "复制供应商" : "添加供应商"}`
      : `${cliNameFromKey(props.provider.cli_key)} · 编辑供应商`;
  const description =
    mode === "create"
      ? isDuplicating
        ? "已复制现有 Provider 配置；CLI 已锁定，请确认名称和认证信息后保存。"
        : "已锁定创建 CLI；如需切换请先关闭弹窗。"
      : undefined;

  useEffect(() => {
    if (mode !== "edit" || !open || !editProvider) return;
    editProviderSnapshotRef.current = editProvider;
  }, [editProvider, mode, open]);

  useEffect(() => {
    setFetchingApiKey(false);
    setOauthLoading(false);
    apiKeyFetchPromiseRef.current = null;

    if (!open) {
      setSavedApiKey(null);
      setOauthStatus(null);
      return () => {
        apiKeyRequestSeqRef.current += 1;
        oauthStatusRequestSeqRef.current += 1;
        apiKeyFetchPromiseRef.current = null;
      };
    }

    baseUrlRowSeqRef.current = 1;
    apiKeyFetchedRef.current = false;
    apiKeyFetchPromiseRef.current = null;
    apiKeyFetchErrorRef.current = false;
    setSavedApiKey(null);

    if (mode === "create") {
      const initialSourceProviderId = createInitialValues?.source_provider_id ?? null;
      setBaseUrlMode(createInitialValues?.base_url_mode ?? "order");
      setBaseUrlRows(buildBaseUrlRows(createInitialValues, newBaseUrlRow));
      setPingingAll(false);
      setClaudeModels(createInitialValues?.claude_models ?? {});
      setTags(createInitialValues?.tags ?? []);
      setTagInput("");
      setStreamIdleTimeoutSeconds(valueOrEmpty(createInitialValues?.stream_idle_timeout_seconds));
      setSourceProviderId(initialSourceProviderId);
      setAuthMode(
        initialSourceProviderId != null ? "cx2cc" : (createInitialValues?.auth_mode ?? "api_key")
      );
      setOauthStatus(null);
      reset(buildFormValues(createInitialValues));
      return;
    }

    const snapshot = editProviderSnapshotRef.current;
    if (!snapshot) return;

    const initialAuthMode = deriveAuthMode(snapshot);
    setAuthMode(initialAuthMode);
    setSourceProviderId(snapshot.source_provider_id ?? null);
    setOauthStatus(null);
    setBaseUrlMode(snapshot.base_url_mode);
    setBaseUrlRows(snapshot.base_urls.map((url) => newBaseUrlRow(url)));
    setPingingAll(false);
    setClaudeModels(snapshot.claude_models ?? {});
    setTags(snapshot.tags ?? []);
    setTagInput("");
    setStreamIdleTimeoutSeconds(valueOrEmpty(snapshot.stream_idle_timeout_seconds));
    reset({
      name: snapshot.name,
      api_key: "",
      auth_mode: initialAuthMode === "cx2cc" ? "api_key" : initialAuthMode,
      cost_multiplier: String(snapshot.cost_multiplier ?? 1.0),
      limit_5h_usd: snapshot.limit_5h_usd != null ? String(snapshot.limit_5h_usd) : "",
      limit_daily_usd: snapshot.limit_daily_usd != null ? String(snapshot.limit_daily_usd) : "",
      limit_weekly_usd: snapshot.limit_weekly_usd != null ? String(snapshot.limit_weekly_usd) : "",
      limit_monthly_usd:
        snapshot.limit_monthly_usd != null ? String(snapshot.limit_monthly_usd) : "",
      limit_total_usd: snapshot.limit_total_usd != null ? String(snapshot.limit_total_usd) : "",
      daily_reset_mode: snapshot.daily_reset_mode ?? "fixed",
      daily_reset_time: snapshot.daily_reset_time ?? "00:00:00",
      enabled: snapshot.enabled,
      note: snapshot.note ?? "",
    });
    return () => {
      apiKeyRequestSeqRef.current += 1;
      oauthStatusRequestSeqRef.current += 1;
      apiKeyFetchPromiseRef.current = null;
    };
  }, [cliKey, createInitialValues, editingProviderId, mode, open, reset]);

  useEffect(() => {
    if (!open || mode !== "edit" || !editingProviderId || authMode !== "api_key") return;
    if (apiKeyFetchedRef.current || apiKeyFetchPromiseRef.current) return;

    const requestSeq = ++apiKeyRequestSeqRef.current;
    setFetchingApiKey(true);
    const request = Promise.resolve(providerGetApiKey(editingProviderId))
      .then((key) => {
        if (apiKeyRequestSeqRef.current !== requestSeq) return null;
        const normalized = key?.trim() ? key : null;
        apiKeyFetchedRef.current = true;
        apiKeyFetchErrorRef.current = false;
        setSavedApiKey(normalized);
        setValue("api_key", normalized ?? "", {
          shouldDirty: false,
          shouldTouch: false,
          shouldValidate: false,
        });
        return normalized;
      })
      .catch(() => {
        if (apiKeyRequestSeqRef.current !== requestSeq) return null;
        apiKeyFetchErrorRef.current = true;
        return null;
      })
      .finally(() => {
        if (apiKeyRequestSeqRef.current !== requestSeq) return;
        apiKeyFetchPromiseRef.current = null;
        setFetchingApiKey(false);
      });

    apiKeyFetchPromiseRef.current = request;
  }, [authMode, editingProviderId, mode, open, setValue]);

  useEffect(() => {
    if (!open) return;

    setTags((prev) => {
      const hasFreeTag = prev.includes(FREE_TAG);

      if (isZeroMultiplier(costMultiplierValue)) {
        const next = hasFreeTag ? moveFreeTagToFront(prev) : [FREE_TAG, ...prev];
        return areTagsEqual(prev, next) ? prev : next;
      }

      if (isNonZeroMultiplier(costMultiplierValue) && hasFreeTag) {
        return prev.filter((tag) => tag !== FREE_TAG);
      }

      return prev;
    });
  }, [costMultiplierValue, open]);

  useEffect(() => {
    if (editProvider?.id && editProvider.auth_mode === "oauth") {
      const requestSeq = ++oauthStatusRequestSeqRef.current;
      providerOAuthStatus(editProvider.id)
        .then((status) => {
          if (oauthStatusRequestSeqRef.current !== requestSeq) return;
          setOauthStatus(status);
        })
        .catch((err) => {
          if (oauthStatusRequestSeqRef.current !== requestSeq) return;
          logToConsole("error", "加载 OAuth 状态失败", {
            provider_id: editProvider.id,
            cli_key: editProvider.cli_key,
            error: String(err),
          });
          toast(`加载 OAuth 状态失败：${String(err)}`);
        });
    }
  }, [editProvider?.auth_mode, editProvider?.cli_key, editProvider?.id]);

  function toastFirstSchemaIssue(issues: Array<{ path: Array<PropertyKey>; message: string }>) {
    const orderedFields: Array<keyof ProviderEditorDialogFormInput> = [
      "name",
      ...(mode === "create" ? (["api_key"] as const) : []),
      "cost_multiplier",
      "limit_5h_usd",
      "limit_daily_usd",
      "limit_weekly_usd",
      "limit_monthly_usd",
      "limit_total_usd",
      "daily_reset_time",
    ];

    const messageByField = new Map<string, string>();
    for (const issue of issues) {
      const firstSegment = issue.path[0];
      if (typeof firstSegment !== "string") continue;
      if (!messageByField.has(firstSegment)) {
        messageByField.set(firstSegment, issue.message);
      }
    }

    for (const field of orderedFields) {
      const maybeMessage = messageByField.get(field);
      if (maybeMessage) {
        toast(maybeMessage);
        return;
      }
    }

    const fallback = issues.find((issue) => Boolean(issue.message));
    if (fallback) {
      toast(fallback.message);
    }
  }

  async function ensureSavedApiKey(silent = false) {
    if (savedApiKey?.trim()) {
      return savedApiKey;
    }
    if (mode !== "edit") {
      return null;
    }

    try {
      if (apiKeyFetchPromiseRef.current) {
        const key = await apiKeyFetchPromiseRef.current;
        if (!key && !silent && apiKeyFetchErrorRef.current) {
          toast("读取 API Key 失败");
        }
        return key;
      }

      setFetchingApiKey(true);
      const request = Promise.resolve(providerGetApiKey(props.provider.id))
        .then((nextKey) => {
          const normalized = nextKey?.trim() ? nextKey : null;
          apiKeyFetchedRef.current = true;
          apiKeyFetchErrorRef.current = false;
          setSavedApiKey(normalized);
          setValue("api_key", normalized ?? "", {
            shouldDirty: false,
            shouldTouch: false,
            shouldValidate: false,
          });
          return normalized;
        })
        .catch(() => {
          apiKeyFetchErrorRef.current = true;
          if (!silent) {
            toast("读取 API Key 失败");
          }
          return null;
        })
        .finally(() => {
          apiKeyFetchPromiseRef.current = null;
          setFetchingApiKey(false);
        });
      apiKeyFetchPromiseRef.current = request;
      return await request;
    } catch {
      if (!silent) {
        toast("读取 API Key 失败");
      }
      return null;
    }
  }

  function resolveStreamIdleTimeoutSeconds() {
    const trimmed = streamIdleTimeoutSeconds.trim();
    if (!trimmed) return 0;
    const value = Number(trimmed);
    if (!Number.isFinite(value) || !Number.isInteger(value) || value < 0 || value > 3600) {
      return undefined;
    }
    return value;
  }

  async function save() {
    if (saving) return;
    const isCx2cc = authMode === "cx2cc";

    const formValues = form.getValues();
    const parsed = createProviderEditorDialogSchema({
      mode,
      skipApiKeyCheck: isCx2cc,
    }).safeParse({
      ...formValues,
      auth_mode: isCx2cc ? "api_key" : authMode,
    });
    if (!parsed.success) {
      toastFirstSchemaIssue(parsed.error.issues);
      return;
    }

    const values: ProviderEditorDialogFormOutput = parsed.data;
    const parsedStreamIdleTimeoutSeconds = resolveStreamIdleTimeoutSeconds();
    if (parsedStreamIdleTimeoutSeconds === undefined) {
      toast("流式空闲超时必须为 0-3600 秒");
      return;
    }

    let finalBaseUrls: string[] = [];
    let finalBaseUrlMode = baseUrlMode;

    if (authMode === "oauth") {
      // OAuth providers don't use base URLs — the gateway routes to the
      // provider's official API endpoint based on the OAuth adapter.
      finalBaseUrls = [];

      // Avoid stale UI race: refresh OAuth status once before enforcing save-time gate.
      let effectiveOauthStatus = oauthStatus;
      if (!effectiveOauthStatus?.connected && editingProviderId) {
        try {
          const latestStatus = await providerOAuthStatus(editingProviderId);
          setOauthStatus(latestStatus);
          effectiveOauthStatus = latestStatus;
        } catch (err) {
          logToConsole("warn", "保存前刷新 OAuth 状态失败", {
            cli_key: cliKey,
            provider_id: editingProviderId,
            error: String(err),
          });
        }
      }

      // Validate OAuth is actually connected before saving.
      if (!effectiveOauthStatus?.connected) {
        toast("请先完成 OAuth 登录");
        return;
      }
    } else if (isCx2cc) {
      // CX2CC providers don't need base URLs — inherited from source provider.
      finalBaseUrls = [];
      finalBaseUrlMode = "order";

      if (!sourceProviderId) {
        toast("请选择源 Codex 供应商");
        return;
      }
    } else {
      const normalized = normalizeBaseUrlRows(baseUrlRows);
      if (!normalized.ok) {
        toast(normalized.message);
        return;
      }
      finalBaseUrls = normalized.baseUrls;
    }

    if (cliKey === "claude" && authMode !== "oauth") {
      const modelError = validateProviderClaudeModels(claudeModels);
      if (modelError) {
        toast(modelError);
        return;
      }
    }

    setSaving(true);
    try {
      const apiKeyToSave =
        authMode === "oauth" ? null : mode === "edit" && !apiKeyDirty ? "" : values.api_key;

      const saved = await providerUpsert({
        ...(mode === "edit" ? { provider_id: props.provider.id } : {}),
        cli_key: cliKey,
        name: values.name,
        base_urls: finalBaseUrls,
        base_url_mode: finalBaseUrlMode,
        auth_mode: isCx2cc ? "api_key" : authMode,
        api_key: authMode === "oauth" || isCx2cc ? null : apiKeyToSave,
        enabled: values.enabled,
        cost_multiplier: values.cost_multiplier,
        limit_5h_usd: values.limit_5h_usd,
        limit_daily_usd: values.limit_daily_usd,
        daily_reset_mode: values.daily_reset_mode,
        daily_reset_time: values.daily_reset_time,
        limit_weekly_usd: values.limit_weekly_usd,
        limit_monthly_usd: values.limit_monthly_usd,
        limit_total_usd: values.limit_total_usd,
        tags,
        note: values.note,
        stream_idle_timeout_seconds: parsedStreamIdleTimeoutSeconds,
        ...(cliKey === "claude" && authMode !== "oauth" ? { claude_models: claudeModels } : {}),
        source_provider_id: isCx2cc ? sourceProviderId : null,
        bridge_type: isCx2cc ? "cx2cc" : null,
      });

      if (!saved) {
        return;
      }

      setValue("api_key", "", { shouldDirty: false, shouldValidate: false });
      logToConsole("info", mode === "create" ? "保存 Provider" : "更新 Provider", {
        cli: saved.cli_key,
        provider_id: saved.id,
        name: saved.name,
        base_urls: saved.base_urls,
        base_url_mode: saved.base_url_mode,
        enabled: saved.enabled,
        cost_multiplier: saved.cost_multiplier,
        claude_models: saved.claude_models,
        limit_5h_usd: saved.limit_5h_usd,
        limit_daily_usd: saved.limit_daily_usd,
        daily_reset_mode: saved.daily_reset_mode,
        daily_reset_time: saved.daily_reset_time,
        limit_weekly_usd: saved.limit_weekly_usd,
        limit_monthly_usd: saved.limit_monthly_usd,
        limit_total_usd: saved.limit_total_usd,
        tags: saved.tags,
        note: saved.note,
        stream_idle_timeout_seconds: saved.stream_idle_timeout_seconds,
      });
      toast(mode === "create" ? "Provider 已保存" : "Provider 已更新");

      onSaved(saved.cli_key);
      onOpenChange(false);
    } catch (err) {
      logToConsole("error", mode === "create" ? "保存 Provider 失败" : "更新 Provider 失败", {
        error: String(err),
        cli: cliKey,
        provider_id: mode === "edit" ? props.provider.id : undefined,
      });
      toast(`${mode === "create" ? "保存" : "更新"}失败：${String(err)}`);
    } finally {
      setSaving(false);
    }
  }

  const apiKeyField = register("api_key");

  async function copyApiKey() {
    const actualValue = apiKeyValue || (await ensureSavedApiKey(false)) || "";
    if (!actualValue.trim()) {
      toast("暂无可复制的 API Key");
      return;
    }
    try {
      await copyText(actualValue);
      toast("已复制 API Key");
    } catch {
      toast("复制 API Key 失败");
    }
  }

  const claudeModelCount =
    cliKey === "claude"
      ? Object.values(claudeModels).filter((value) => {
          if (typeof value !== "string") return false;
          return Boolean(value.trim());
        }).length
      : 0;
  const supportsOAuth = cliKey === "codex" || cliKey === "gemini";
  const supportsCx2cc = cliKey === "claude";

  const tagsAndNoteSection = (
    <div className="grid gap-3 sm:grid-cols-2">
      <FormField label="标签" hint="按 Enter 添加标签">
        <div className="flex min-h-10 flex-wrap items-center gap-1.5 rounded-lg border border-slate-200 bg-white px-3 shadow-sm dark:border-slate-600 dark:bg-slate-800 dark:shadow-none">
          {tags.map((tag) => (
            <span key={tag} className={tagBadgeClassName(tag)}>
              {tag}
              <button
                type="button"
                onClick={() => setTags((prev) => prev.filter((t) => t !== tag))}
                className={tagRemoveButtonClassName(tag)}
                disabled={saving}
                aria-label={`移除标签 ${tag}`}
              >
                <X className="h-2.5 w-2.5" />
              </button>
            </span>
          ))}
          <input
            type="text"
            value={tagInput}
            onChange={(e) => setTagInput(e.currentTarget.value)}
            onKeyDown={(e) => {
              if (e.key !== "Enter") return;
              e.preventDefault();
              const trimmed = tagInput.trim();
              if (!trimmed) return;
              if (tags.includes(trimmed)) {
                setTagInput("");
                return;
              }
              setTags((prev) => [...prev, trimmed]);
              setTagInput("");
            }}
            placeholder={tags.length === 0 ? "输入标签后按 Enter" : ""}
            className="min-w-[80px] flex-1 border-none bg-transparent text-sm outline-none placeholder:text-slate-400"
            disabled={saving}
          />
        </div>
      </FormField>

      <FormField label="备注" hint="供应商列表中显示">
        <Input placeholder="可选备注信息" disabled={saving} {...register("note")} />
      </FormField>
    </div>
  );

  async function handleOAuthLogin() {
    setOauthLoading(true);
    let autoSavedProviderId: number | null = null;
    let shouldRollbackAutoSavedProvider = false;

    const rollbackAutoSavedProvider = async () => {
      if (!shouldRollbackAutoSavedProvider || !autoSavedProviderId) return;
      try {
        const deleted = await providerDelete(autoSavedProviderId);
        if (!deleted) {
          logToConsole(
            "warn",
            `OAuth 登录失败后清理临时 Provider 失败：${form.getValues().name || "OAuth Provider"}`,
            {
              cli_key: cliKey,
              provider_id: autoSavedProviderId,
            }
          );
        }
      } catch (cleanupErr) {
        logToConsole(
          "error",
          `OAuth 登录失败后清理临时 Provider 异常：${form.getValues().name || "OAuth Provider"}`,
          {
            cli_key: cliKey,
            provider_id: autoSavedProviderId,
            error: String(cleanupErr),
          }
        );
      }
    };

    try {
      let targetProviderId = editingProviderId;
      const parsedStreamIdleTimeoutSeconds = resolveStreamIdleTimeoutSeconds();
      if (parsedStreamIdleTimeoutSeconds === undefined) {
        toast("流式空闲超时必须为 0-3600 秒");
        return;
      }

      // In create mode, auto-save the provider first to obtain an ID.
      if (!targetProviderId) {
        const formValues = form.getValues();
        if (!formValues.name?.trim()) {
          toast("请先填写 Provider 名称");
          return;
        }
        const saved = await providerUpsert({
          cli_key: cliKey,
          name: formValues.name.trim(),
          base_urls: [],
          base_url_mode: "order",
          auth_mode: "oauth",
          api_key: null,
          enabled: formValues.enabled,
          cost_multiplier: Number(formValues.cost_multiplier) || 1.0,
          limit_5h_usd: formValues.limit_5h_usd ? Number(formValues.limit_5h_usd) : null,
          limit_daily_usd: formValues.limit_daily_usd ? Number(formValues.limit_daily_usd) : null,
          daily_reset_mode: formValues.daily_reset_mode ?? "fixed",
          daily_reset_time: formValues.daily_reset_time ?? "00:00:00",
          limit_weekly_usd: formValues.limit_weekly_usd
            ? Number(formValues.limit_weekly_usd)
            : null,
          limit_monthly_usd: formValues.limit_monthly_usd
            ? Number(formValues.limit_monthly_usd)
            : null,
          limit_total_usd: formValues.limit_total_usd ? Number(formValues.limit_total_usd) : null,
          stream_idle_timeout_seconds: parsedStreamIdleTimeoutSeconds,
        });
        if (!saved) {
          toast("自动保存 Provider 失败");
          return;
        }
        targetProviderId = saved.id;
        autoSavedProviderId = saved.id;
        shouldRollbackAutoSavedProvider = true;
      }

      const result = await providerOAuthStartFlow(cliKey, targetProviderId);
      if (result?.success) {
        shouldRollbackAutoSavedProvider = false;

        let status: Awaited<ReturnType<typeof providerOAuthStatus>> = null;
        try {
          status = await providerOAuthStatus(targetProviderId);
          setOauthStatus(status);
        } catch (statusErr) {
          toast("OAuth 登录成功，但读取连接状态失败，可稍后重试");
          logToConsole(
            "warn",
            `OAuth 登录后读取状态失败：${form.getValues().name || "OAuth Provider"}`,
            {
              cli_key: cliKey,
              provider_id: targetProviderId,
              provider_type: result.provider_type,
              error: String(statusErr),
            }
          );
        }

        let limits: Awaited<ReturnType<typeof providerOAuthFetchLimits>> = null;
        try {
          limits = await providerOAuthFetchLimits(targetProviderId);
          if (!limits) {
            toast("OAuth 登录成功，但获取用量失败，可稍后重试");
            logToConsole(
              "warn",
              `OAuth 登录后获取用量失败：${form.getValues().name || "OAuth Provider"}`,
              {
                cli_key: cliKey,
                provider_id: targetProviderId,
                provider_type: result.provider_type,
                email: status?.email,
              }
            );
          }
        } catch (err) {
          toast("OAuth 登录成功，但获取用量失败，可稍后重试");
          logToConsole(
            "warn",
            `OAuth 登录后获取用量异常：${form.getValues().name || "OAuth Provider"}`,
            {
              cli_key: cliKey,
              provider_id: targetProviderId,
              provider_type: result.provider_type,
              email: status?.email,
              error: String(err),
            }
          );
        }

        toast("OAuth 登录成功");
        logToConsole("info", `OAuth 登录成功：${form.getValues().name || "OAuth Provider"}`, {
          cli_key: cliKey,
          provider_id: targetProviderId,
          provider_type: result.provider_type,
          email: status?.email,
          expires_at: result.expires_at,
          limit_5h: limits?.limit_5h_text,
          limit_weekly: limits?.limit_weekly_text,
        });
        // If we auto-saved (create mode), notify parent and close dialog.
        if (!editingProviderId) {
          onSaved(cliKey);
          onOpenChange(false);
        }
      } else {
        await rollbackAutoSavedProvider();
        toast("OAuth 登录失败");
        logToConsole("warn", `OAuth 登录失败：${form.getValues().name || "OAuth Provider"}`, {
          cli_key: cliKey,
          provider_id: targetProviderId,
        });
      }
    } catch (err) {
      await rollbackAutoSavedProvider();
      toast(`OAuth 登录失败：${String(err)}`);
      logToConsole("error", `OAuth 登录异常：${form.getValues().name || "OAuth Provider"}`, {
        cli_key: cliKey,
        error: String(err),
      });
    } finally {
      setOauthLoading(false);
    }
  }

  async function handleOAuthRefresh() {
    if (!editingProviderId) return;
    setOauthLoading(true);
    try {
      const result = await providerOAuthRefresh(editingProviderId);
      if (result?.success) {
        const status = await providerOAuthStatus(editingProviderId);
        setOauthStatus(status);
        toast("Token 刷新成功");
        logToConsole("info", `OAuth Token 刷新成功：${form.getValues().name}`, {
          provider_id: editingProviderId,
          expires_at: result.expires_at,
        });
      } else {
        toast("Token 刷新失败");
        logToConsole("warn", `OAuth Token 刷新失败：${form.getValues().name}`, {
          provider_id: editingProviderId,
        });
      }
    } catch (err) {
      toast(`Token 刷新失败：${String(err)}`);
      logToConsole("error", `OAuth Token 刷新异常：${form.getValues().name}`, {
        provider_id: editingProviderId,
        error: String(err),
      });
    } finally {
      setOauthLoading(false);
    }
  }

  async function handleOAuthDisconnect() {
    if (!editingProviderId) return;
    setOauthLoading(true);
    try {
      const result = await providerOAuthDisconnect(editingProviderId);
      if (result?.success) {
        setOauthStatus(null);
        toast("已断开 OAuth 连接");
        logToConsole("info", `OAuth 已断开连接：${form.getValues().name}`, {
          provider_id: editingProviderId,
        });
      } else {
        toast("断开 OAuth 连接失败");
        logToConsole("warn", `OAuth 断开连接失败：${form.getValues().name}`, {
          provider_id: editingProviderId,
        });
      }
    } catch (err) {
      toast(`断开 OAuth 连接失败：${String(err)}`);
      logToConsole("error", `OAuth 断开连接异常：${form.getValues().name}`, {
        provider_id: editingProviderId,
        error: String(err),
      });
    } finally {
      setOauthLoading(false);
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(nextOpen) => {
        if (!nextOpen && saving) return;
        onOpenChange(nextOpen);
      }}
      title={title}
      description={description}
      className="max-w-4xl"
    >
      <div className="space-y-4">
        {/* ── Auth mode selector (Codex/Gemini: api_key/oauth; Claude: api_key/cx2cc) ── */}
        {supportsOAuth ? (
          <FormField label="认证方式" hint="选择后下方表单会相应变化">
            <TabList<"api_key" | "oauth">
              ariaLabel="认证方式"
              items={[
                { key: "api_key", label: "API 密钥" },
                { key: "oauth", label: "OAuth 登录" },
              ]}
              value={authMode as "api_key" | "oauth"}
              onChange={(next) => {
                setAuthMode(next);
                setValue("auth_mode", next, { shouldDirty: true });
              }}
            />
          </FormField>
        ) : supportsCx2cc ? (
          <FormField label="认证方式" hint="选择后下方表单会相应变化">
            <TabList<"api_key" | "oauth" | "cx2cc">
              ariaLabel="认证方式"
              items={[
                { key: "api_key", label: "API 密钥" },
                { key: "oauth", label: "OAuth" },
                { key: "cx2cc", label: "CX2CC 转译" },
              ]}
              value={authMode as "api_key" | "oauth" | "cx2cc"}
              onChange={(next) => {
                setAuthMode(next);
                setValue("auth_mode", next === "cx2cc" ? "api_key" : next, { shouldDirty: true });
              }}
            />
          </FormField>
        ) : null}

        {authMode === "oauth" ? (
          /* ── OAuth mode: simplified form ── */
          <>
            <FormField label="名称">
              <Input placeholder="default" {...register("name")} />
            </FormField>

            <FormField label="OAuth 连接">
              <div className="rounded-md border border-slate-200 bg-slate-50 p-3 dark:border-slate-700 dark:bg-slate-800/50">
                {oauthLoading ? (
                  <div className="flex items-center gap-2 text-sm text-slate-500">
                    <span className="animate-spin">⏳</span>
                    <span>处理中...</span>
                  </div>
                ) : oauthStatus?.connected ? (
                  <div className="space-y-2">
                    {oauthStatus.email && (
                      <p className="text-sm text-slate-700 dark:text-slate-300">
                        <span className="font-medium">账号：</span>
                        {oauthStatus.email}
                      </p>
                    )}
                    {oauthStatus.expires_at && (
                      <p className="text-xs text-slate-500 dark:text-slate-400">
                        <span className="font-medium">到期：</span>
                        {formatUnixSeconds(oauthStatus.expires_at)}
                      </p>
                    )}
                    <div className="flex items-center gap-2">
                      <Button
                        onClick={handleOAuthRefresh}
                        variant="secondary"
                        disabled={saving || oauthLoading}
                      >
                        刷新 Token
                      </Button>
                      <Button
                        onClick={handleOAuthDisconnect}
                        variant="secondary"
                        disabled={saving || oauthLoading}
                      >
                        断开连接
                      </Button>
                    </div>
                  </div>
                ) : (
                  <div className="space-y-2">
                    <p className="text-sm text-slate-500 dark:text-slate-400">未连接 OAuth</p>
                    <Button
                      onClick={handleOAuthLogin}
                      variant="primary"
                      disabled={saving || oauthLoading}
                    >
                      OAuth 登录
                    </Button>
                  </div>
                )}
              </div>
            </FormField>

            <FormField label="价格倍率">
              <Input
                type="number"
                min="0.0001"
                step="0.01"
                placeholder="1.0"
                {...register("cost_multiplier")}
              />
            </FormField>
          </>
        ) : authMode === "cx2cc" ? (
          /* ── CX2CC mode: source codex provider selector ── */
          <>
            <div className="grid gap-3 sm:grid-cols-2">
              <FormField label="名称">
                <Input placeholder="default" {...register("name")} />
              </FormField>

              <FormField label="价格倍率">
                <Input
                  type="number"
                  min="0"
                  step="0.01"
                  placeholder="1.0"
                  {...register("cost_multiplier")}
                />
              </FormField>
            </div>

            <FormField label="源 Codex 供应商" hint="CX2CC 将复用此供应商的凭证和 Base URL">
              <select
                className="w-full rounded-md border border-slate-300 bg-white px-3 py-2 text-sm shadow-sm focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent dark:border-slate-600 dark:bg-slate-800 dark:text-slate-200"
                value={sourceProviderId ?? ""}
                onChange={(e) => {
                  const val = e.target.value ? Number(e.target.value) : null;
                  setSourceProviderId(val);
                }}
                disabled={saving}
              >
                <option value="">请选择 Codex 供应商…</option>
                {codexProviders
                  .filter((p) => p.enabled && p.source_provider_id == null)
                  .map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name} ({p.auth_mode === "oauth" ? "OAuth" : "API Key"})
                    </option>
                  ))}
              </select>
              {(() => {
                const selected = sourceProviderId
                  ? (codexProviders.find((p) => p.id === sourceProviderId) ?? null)
                  : null;
                return selected ? (
                  <p className="mt-1 text-xs text-slate-500 dark:text-slate-400">
                    已选择：{selected.name} —{" "}
                    {selected.auth_mode === "oauth" ? "OAuth 认证" : "API Key 认证"}
                    {selected.base_urls.length > 0 ? ` — ${selected.base_urls[0]}` : ""}
                  </p>
                ) : null;
              })()}
            </FormField>

            {tagsAndNoteSection}
          </>
        ) : (
          /* ── API Key mode: full form ── */
          <>
            <div className="grid gap-3 sm:grid-cols-2">
              <FormField label="名称">
                <Input placeholder="default" {...register("name")} />
              </FormField>

              <FormField label="标签" hint="按 Enter 添加标签">
                <div className="flex min-h-10 flex-wrap items-center gap-1.5 rounded-lg border border-slate-200 bg-white px-3 shadow-sm dark:border-slate-600 dark:bg-slate-800 dark:shadow-none">
                  {tags.map((tag) => (
                    <span key={tag} className={tagBadgeClassName(tag)}>
                      {tag}
                      <button
                        type="button"
                        onClick={() => setTags((prev) => prev.filter((t) => t !== tag))}
                        className={tagRemoveButtonClassName(tag)}
                        disabled={saving}
                        aria-label={`移除标签 ${tag}`}
                      >
                        <X className="h-2.5 w-2.5" />
                      </button>
                    </span>
                  ))}
                  <input
                    type="text"
                    value={tagInput}
                    onChange={(e) => setTagInput(e.currentTarget.value)}
                    onKeyDown={(e) => {
                      if (e.key !== "Enter") return;
                      e.preventDefault();
                      const trimmed = tagInput.trim();
                      if (!trimmed) return;
                      if (tags.includes(trimmed)) {
                        setTagInput("");
                        return;
                      }
                      setTags((prev) => [...prev, trimmed]);
                      setTagInput("");
                    }}
                    placeholder={tags.length === 0 ? "输入标签后按 Enter" : ""}
                    className="min-w-[80px] flex-1 border-none bg-transparent text-sm outline-none placeholder:text-slate-400"
                    disabled={saving}
                  />
                </div>
              </FormField>
            </div>

            <FormField label="备注">
              <Input placeholder="可选备注信息" disabled={saving} {...register("note")} />
            </FormField>

            <FormField label="Base URLs">
              <BaseUrlEditor
                rows={baseUrlRows}
                setRows={setBaseUrlRows}
                pingingAll={pingingAll}
                setPingingAll={setPingingAll}
                newRow={newBaseUrlRow}
                placeholder="中转 endpoint（例如：https://example.com/v1）"
                disabled={saving}
                footerStart={
                  <div className="flex items-center gap-2">
                    <span className="shrink-0 text-[11px] font-medium text-slate-500 dark:text-slate-400">
                      URL 选择策略
                    </span>
                    <RadioButtonGroup<ProviderBaseUrlMode>
                      items={[
                        { value: "order", label: "按顺序" },
                        { value: "ping", label: "按 Ping" },
                      ]}
                      ariaLabel="Base URL 选择策略"
                      value={baseUrlMode}
                      onChange={setBaseUrlMode}
                      disabled={saving}
                      size="compact"
                      fullWidth={false}
                    />
                  </div>
                }
              />
            </FormField>

            <div className="grid gap-3 sm:grid-cols-2">
              <FormField label="API Key / Token">
                <div className="flex items-center gap-2">
                  <Input {...apiKeyField} type="text" placeholder="sk-…" autoComplete="off" />
                  <Button
                    type="button"
                    onClick={() => void copyApiKey()}
                    variant="secondary"
                    size="sm"
                    className="h-9 shrink-0"
                    disabled={fetchingApiKey}
                  >
                    复制
                  </Button>
                </div>
              </FormField>

              <FormField label="价格倍率">
                <div className="flex items-center gap-2">
                  <Input
                    type="number"
                    min="0"
                    step="0.01"
                    placeholder="1.0"
                    {...register("cost_multiplier")}
                  />
                  <Button
                    type="button"
                    variant="secondary"
                    size="sm"
                    className={
                      isZeroMultiplier(costMultiplierValue)
                        ? "h-9 shrink-0 border-emerald-200 bg-emerald-50 text-emerald-700 hover:bg-emerald-100 dark:border-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-300 dark:hover:bg-emerald-900/40"
                        : "h-9 shrink-0"
                    }
                    disabled={saving}
                    onClick={() =>
                      setValue("cost_multiplier", "0", {
                        shouldDirty: true,
                        shouldTouch: true,
                        shouldValidate: true,
                      })
                    }
                  >
                    免费
                  </Button>
                </div>
              </FormField>
            </div>
          </>
        )}

        <FormField
          label="流式空闲超时覆盖（秒）"
          hint="留空或 0 表示沿用全局设置；仅对当前 Provider 的流式请求生效。"
        >
          <Input
            type="number"
            min="0"
            max="3600"
            step="1"
            placeholder="0"
            value={streamIdleTimeoutSeconds}
            onChange={(e) => setStreamIdleTimeoutSeconds(e.currentTarget.value)}
            disabled={saving}
          />
        </FormField>

        <details className="group rounded-xl border border-slate-200 bg-gradient-to-br from-slate-50/80 to-white shadow-sm open:ring-2 open:ring-accent/10 transition-all dark:border-slate-700 dark:from-slate-800/80 dark:to-slate-900">
          <summary className="flex cursor-pointer items-center justify-between px-5 py-4 select-none">
            <div className="flex items-center gap-3">
              <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-gradient-to-br from-amber-400 to-orange-500 shadow-sm">
                <DollarSign className="h-4 w-4 text-white" />
              </div>
              <div>
                <span className="text-sm font-semibold text-slate-700 group-open:text-accent dark:text-slate-300">
                  限流配置
                </span>
                <p className="text-xs text-slate-400 dark:text-slate-500">
                  配置不同时间窗口的消费限制以控制成本
                </p>
              </div>
            </div>
            <ChevronDown className="h-4 w-4 text-slate-400 transition-transform group-open:rotate-180" />
          </summary>

          <div className="space-y-6 border-t border-slate-100 px-5 py-5 dark:border-slate-700">
            {/* Time-based limits section */}
            <div>
              <h4 className="mb-3 text-xs font-semibold uppercase tracking-wider text-slate-500 dark:text-slate-400">
                时间维度限制
              </h4>
              <div className="grid gap-4 sm:grid-cols-2">
                <LimitCard
                  icon={<Clock className="h-5 w-5 text-blue-600" />}
                  iconBgClass="bg-blue-50 dark:bg-blue-900/30"
                  label="5 小时消费上限"
                  hint="留空表示不限制"
                  value={limit5hUsd}
                  onChange={(value) => setValue("limit_5h_usd", value, { shouldDirty: true })}
                  placeholder="例如: 10"
                  disabled={saving}
                />
                <LimitCard
                  icon={<DollarSign className="h-5 w-5 text-emerald-600" />}
                  iconBgClass="bg-emerald-50 dark:bg-emerald-900/30"
                  label="每日消费上限"
                  hint="留空表示不限制"
                  value={limitDailyUsd}
                  onChange={(value) => setValue("limit_daily_usd", value, { shouldDirty: true })}
                  placeholder="例如: 100"
                  disabled={saving}
                />
                <LimitCard
                  icon={<CalendarDays className="h-5 w-5 text-violet-600" />}
                  iconBgClass="bg-violet-50 dark:bg-violet-900/30"
                  label="周消费上限"
                  hint="自然周：周一 00:00:00"
                  value={limitWeeklyUsd}
                  onChange={(value) => setValue("limit_weekly_usd", value, { shouldDirty: true })}
                  placeholder="例如: 500"
                  disabled={saving}
                />
                <LimitCard
                  icon={<CalendarRange className="h-5 w-5 text-orange-600" />}
                  iconBgClass="bg-orange-50 dark:bg-orange-900/30"
                  label="月消费上限"
                  hint="自然月：每月 1 号 00:00:00"
                  value={limitMonthlyUsd}
                  onChange={(value) => setValue("limit_monthly_usd", value, { shouldDirty: true })}
                  placeholder="例如: 2000"
                  disabled={saving}
                />
              </div>
            </div>

            {/* Daily reset settings section */}
            <div>
              <h4 className="mb-3 text-xs font-semibold uppercase tracking-wider text-slate-500 dark:text-slate-400">
                每日重置设置
              </h4>
              <div className="rounded-xl border border-slate-200 bg-white p-4 shadow-sm dark:border-slate-700 dark:bg-slate-800">
                <div className="flex items-start gap-3">
                  <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-sky-50 dark:bg-sky-900/30">
                    <RotateCcw className="h-5 w-5 text-sky-600" />
                  </div>
                  <div className="min-w-0 flex-1 space-y-4">
                    <div className="grid gap-4 sm:grid-cols-2">
                      <div>
                        <label className="text-sm font-medium text-slate-700 dark:text-slate-300">
                          每日重置模式
                        </label>
                        <p className="mb-2 text-xs text-slate-400 dark:text-slate-500">
                          rolling 为过去 24 小时窗口
                        </p>
                        <RadioButtonGroup<DailyResetMode>
                          items={[
                            { value: "fixed", label: "固定时间" },
                            { value: "rolling", label: "滚动窗口 (24h)" },
                          ]}
                          ariaLabel="每日重置模式"
                          value={dailyResetMode}
                          onChange={(value) =>
                            setValue("daily_reset_mode", value, { shouldDirty: true })
                          }
                          disabled={saving}
                        />
                      </div>
                      <div>
                        <label className="text-sm font-medium text-slate-700 dark:text-slate-300">
                          每日重置时间
                        </label>
                        <p className="mb-2 text-xs text-slate-400 dark:text-slate-500">
                          {dailyResetMode === "fixed"
                            ? "默认 00:00:00（本机时区）"
                            : "rolling 模式下忽略"}
                        </p>
                        <Input
                          type="time"
                          step="1"
                          disabled={saving || dailyResetMode !== "fixed"}
                          {...register("daily_reset_time")}
                        />
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {/* Other limits section */}
            <div>
              <h4 className="mb-3 text-xs font-semibold uppercase tracking-wider text-slate-500 dark:text-slate-400">
                其他限制
              </h4>
              <div className="grid gap-4 sm:grid-cols-2">
                <LimitCard
                  icon={<Gauge className="h-5 w-5 text-rose-600" />}
                  iconBgClass="bg-rose-50 dark:bg-rose-900/30"
                  label="总消费上限"
                  hint="达到后需手动调整/清除"
                  value={limitTotalUsd}
                  onChange={(value) => setValue("limit_total_usd", value, { shouldDirty: true })}
                  placeholder="例如: 1000"
                  disabled={saving}
                />
              </div>
            </div>
          </div>
        </details>

        {cliKey === "claude" && authMode !== "oauth" ? (
          <details className="group rounded-xl border border-slate-200 bg-white shadow-sm open:ring-2 open:ring-accent/10 transition-all dark:border-slate-700 dark:bg-slate-800">
            <summary className="flex cursor-pointer items-center justify-between px-4 py-3 select-none">
              <div className="flex items-center gap-3">
                <span className="text-sm font-medium text-slate-700 group-open:text-accent dark:text-slate-300">
                  Claude 模型映射
                </span>
                <span className="text-xs font-mono text-slate-500 dark:text-slate-400">
                  已配置 {claudeModelCount}/5
                </span>
              </div>
              <ChevronDown className="h-4 w-4 text-slate-400 transition-transform group-open:rotate-180" />
            </summary>

            <div className="space-y-4 border-t border-slate-100 px-4 py-3 dark:border-slate-700">
              <FormField
                label="主模型"
                hint="默认兜底模型；未命中 haiku/sonnet/opus 且未启用 Thinking 时使用"
              >
                <Input
                  value={claudeModels.main_model ?? ""}
                  onChange={(e) => {
                    const value = e.currentTarget.value;
                    setClaudeModels((prev) => {
                      const oldMain = (prev.main_model ?? "").trim();
                      const syncIfMatch = (field: string | null | undefined) => {
                        const trimmed = (field ?? "").trim();
                        return !trimmed || trimmed === oldMain ? value : field;
                      };
                      return {
                        ...prev,
                        main_model: value,
                        haiku_model: syncIfMatch(prev.haiku_model),
                        sonnet_model: syncIfMatch(prev.sonnet_model),
                        opus_model: syncIfMatch(prev.opus_model),
                      };
                    });
                  }}
                  placeholder="例如: glm-4-plus / minimax-text-01 / kimi-k2"
                  disabled={saving}
                />
              </FormField>

              <FormField
                label="推理模型 (Thinking)"
                hint="当请求中 thinking.type=enabled 时优先使用"
              >
                <Input
                  value={claudeModels.reasoning_model ?? ""}
                  onChange={(e) => {
                    const value = e.currentTarget.value;
                    setClaudeModels((prev) => ({
                      ...prev,
                      reasoning_model: value,
                    }));
                  }}
                  placeholder="例如: kimi-k2-thinking / glm-4-plus-thinking"
                  disabled={saving}
                />
              </FormField>

              <FormField label="Haiku 默认模型" hint="当请求模型名包含 haiku 时使用（子串匹配）">
                <Input
                  value={claudeModels.haiku_model ?? ""}
                  onChange={(e) => {
                    const value = e.currentTarget.value;
                    setClaudeModels((prev) => ({ ...prev, haiku_model: value }));
                  }}
                  placeholder="例如: glm-4-plus-haiku"
                  disabled={saving}
                />
              </FormField>

              <FormField label="Sonnet 默认模型" hint="当请求模型名包含 sonnet 时使用（子串匹配）">
                <Input
                  value={claudeModels.sonnet_model ?? ""}
                  onChange={(e) => {
                    const value = e.currentTarget.value;
                    setClaudeModels((prev) => ({ ...prev, sonnet_model: value }));
                  }}
                  placeholder="例如: glm-4-plus-sonnet"
                  disabled={saving}
                />
              </FormField>

              <FormField label="Opus 默认模型" hint="当请求模型名包含 opus 时使用（子串匹配）">
                <Input
                  value={claudeModels.opus_model ?? ""}
                  onChange={(e) => {
                    const value = e.currentTarget.value;
                    setClaudeModels((prev) => ({ ...prev, opus_model: value }));
                  }}
                  placeholder="例如: glm-4-plus-opus"
                  disabled={saving}
                />
              </FormField>
            </div>
          </details>
        ) : null}

        <div className="flex items-center justify-between border-t border-slate-100 pt-3 dark:border-slate-700">
          <div className="flex items-center gap-2">
            <span className="text-sm text-slate-700 dark:text-slate-300">启用</span>
            <Switch
              checked={enabled}
              onCheckedChange={(checked) => setValue("enabled", checked, { shouldDirty: true })}
              disabled={saving}
            />
          </div>
          <div className="flex items-center gap-2">
            <Button onClick={() => onOpenChange(false)} variant="secondary" disabled={saving}>
              取消
            </Button>
            <Button onClick={save} variant="primary" disabled={saving}>
              {saving ? "保存中…" : "保存"}
            </Button>
          </div>
        </div>
      </div>
    </Dialog>
  );
}
