// Usage: Used by ProvidersView to create/edit a Provider with toast-based validation.

import { useEffect, useMemo, useRef, useState, type Dispatch, type SetStateAction } from "react";
import {
  ChevronDown,
  Clock,
  DollarSign,
  CalendarDays,
  CalendarRange,
  Gauge,
  LogIn,
  LogOut,
  RotateCcw,
  X,
} from "lucide-react";
import { toast } from "sonner";
import { cliLongLabel } from "../../constants/clis";
import { logToConsole } from "../../services/consoleLog";
import {
  oauthStartLogin,
  oauthProviderLogout,
  oauthProviderStatus,
  providerUpsert,
  type AuthType,
  type ClaudeModels,
  type CliKey,
  type OAuthProviderStatus,
  type OAuthProviderType,
  type ProviderSummary,
} from "../../services/providers";
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
import { normalizeBaseUrlRows } from "./baseUrl";
import { BaseUrlEditor } from "./BaseUrlEditor";
import { LimitCard } from "./LimitCard";
import { RadioButtonGroup } from "./RadioButtonGroup";
import type { BaseUrlRow, ProviderBaseUrlMode } from "./types";
import { validateProviderClaudeModels } from "./validators";
import { useForm } from "react-hook-form";

type DailyResetMode = "fixed" | "rolling";

type ProviderEditorDialogBaseProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSaved: (cliKey: CliKey) => void;
};

export type ProviderEditorDialogProps =
  | (ProviderEditorDialogBaseProps & {
      mode: "create";
      cliKey: CliKey;
    })
  | (ProviderEditorDialogBaseProps & {
      mode: "edit";
      provider: ProviderSummary;
    });

function cliNameFromKey(cliKey: CliKey) {
  return cliLongLabel(cliKey);
}

export function ProviderEditorDialog(props: ProviderEditorDialogProps) {
  const { open, onOpenChange, onSaved } = props;

  const mode = props.mode;
  const cliKey = mode === "create" ? props.cliKey : props.provider.cli_key;
  const editingProviderId = mode === "edit" ? props.provider.id : null;

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
  const [saving, setSaving] = useState(false);
  const [oauthStatus, setOauthStatus] = useState<OAuthProviderStatus | null>(null);
  const [oauthLoggingIn, setOauthLoggingIn] = useState(false);

  const schema = useMemo(() => createProviderEditorDialogSchema({ mode }), [mode]);
  const form = useForm<ProviderEditorDialogFormInput>({
    defaultValues: {
      name: "",
      auth_type: "api_key",
      oauth_provider_type: "",
      api_key: "",
      cost_multiplier: "1.0",
      limit_5h_usd: "",
      limit_daily_usd: "",
      limit_weekly_usd: "",
      limit_monthly_usd: "",
      limit_total_usd: "",
      daily_reset_mode: "fixed",
      daily_reset_time: "00:00:00",
      enabled: true,
    },
  });

  const { register, reset, setValue, watch } = form;
  const enabled = watch("enabled");
  const authType = watch("auth_type") as AuthType;
  const dailyResetMode = watch("daily_reset_mode");
  const limit5hUsd = watch("limit_5h_usd");
  const limitDailyUsd = watch("limit_daily_usd");
  const limitWeeklyUsd = watch("limit_weekly_usd");
  const limitMonthlyUsd = watch("limit_monthly_usd");
  const limitTotalUsd = watch("limit_total_usd");

  const title =
    mode === "create"
      ? `${cliNameFromKey(cliKey)} · 添加供应商`
      : `${cliNameFromKey(props.provider.cli_key)} · 编辑供应商`;
  const description = mode === "create" ? "已锁定创建 CLI；如需切换请先关闭弹窗。" : undefined;

  useEffect(() => {
    if (!open) return;

    baseUrlRowSeqRef.current = 1;

    if (mode === "create") {
      setBaseUrlMode("order");
      setBaseUrlRows([newBaseUrlRow()]);
      setPingingAll(false);
      setClaudeModels({});
      setTags([]);
      setTagInput("");
      setOauthStatus(null);
      setOauthLoggingIn(false);
      reset({
        name: "",
        auth_type: "api_key",
        oauth_provider_type: "",
        api_key: "",
        cost_multiplier: "1.0",
        limit_5h_usd: "",
        limit_daily_usd: "",
        limit_weekly_usd: "",
        limit_monthly_usd: "",
        limit_total_usd: "",
        daily_reset_mode: "fixed",
        daily_reset_time: "00:00:00",
        enabled: true,
      });
      return;
    }

    setBaseUrlMode(props.provider.base_url_mode);
    setBaseUrlRows(props.provider.base_urls.map((url) => newBaseUrlRow(url)));
    setPingingAll(false);
    setClaudeModels(props.provider.claude_models ?? {});
    setTags(props.provider.tags ?? []);
    setTagInput("");
    setOauthLoggingIn(false);
    reset({
      name: props.provider.name,
      auth_type: props.provider.auth_type ?? "api_key",
      oauth_provider_type: props.provider.oauth_provider_type ?? "",
      api_key: "",
      cost_multiplier: String(props.provider.cost_multiplier ?? 1.0),
      limit_5h_usd: props.provider.limit_5h_usd != null ? String(props.provider.limit_5h_usd) : "",
      limit_daily_usd:
        props.provider.limit_daily_usd != null ? String(props.provider.limit_daily_usd) : "",
      limit_weekly_usd:
        props.provider.limit_weekly_usd != null ? String(props.provider.limit_weekly_usd) : "",
      limit_monthly_usd:
        props.provider.limit_monthly_usd != null ? String(props.provider.limit_monthly_usd) : "",
      limit_total_usd:
        props.provider.limit_total_usd != null ? String(props.provider.limit_total_usd) : "",
      daily_reset_mode: props.provider.daily_reset_mode ?? "fixed",
      daily_reset_time: props.provider.daily_reset_time ?? "00:00:00",
      enabled: props.provider.enabled,
    });

    // Load OAuth status for edit mode OAuth providers.
    if (props.provider.auth_type === "oauth") {
      oauthProviderStatus(props.provider.id).then((status) => {
        if (status) setOauthStatus(status);
      });
    } else {
      setOauthStatus(null);
    }
    // Only reset form when dialog opens or provider identity changes.
    // Intentionally omitting props.provider fields to avoid resetting user edits
    // when the provider object reference changes from a background query refetch.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [cliKey, editingProviderId, mode, open, reset]);

  const setBaseUrlRowsFromUser: Dispatch<SetStateAction<BaseUrlRow[]>> = (action) => {
    setBaseUrlRows(action);
  };

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

  async function save() {
    if (saving) return;

    const parsed = schema.safeParse(form.getValues());
    if (!parsed.success) {
      toastFirstSchemaIssue(parsed.error.issues);
      return;
    }

    const values: ProviderEditorDialogFormOutput = parsed.data;

    const normalized = normalizeBaseUrlRows(baseUrlRows);
    if (!normalized.ok) {
      toast(normalized.message);
      return;
    }

    if (cliKey === "claude") {
      const modelError = validateProviderClaudeModels(claudeModels);
      if (modelError) {
        toast(modelError);
        return;
      }
    }

    setSaving(true);
    try {
      const saved = await providerUpsert({
        ...(mode === "edit" ? { provider_id: props.provider.id } : {}),
        cli_key: cliKey,
        name: values.name,
        base_urls: normalized.baseUrls,
        base_url_mode: baseUrlMode,
        api_key: values.auth_type === "oauth" ? undefined : values.api_key,
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
        auth_type: values.auth_type as AuthType,
        oauth_provider_type: values.oauth_provider_type as OAuthProviderType | undefined,
        ...(cliKey === "claude" ? { claude_models: claudeModels } : {}),
      });

      if (!saved) {
        toast("仅在 Tauri Desktop 环境可用");
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

  const claudeModelCount =
    cliKey === "claude"
      ? Object.values(claudeModels).filter((value) => {
          if (typeof value !== "string") return false;
          return Boolean(value.trim());
        }).length
      : 0;

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
        <div className="grid gap-3 sm:grid-cols-2">
          <FormField label="名称">
            <Input placeholder="default" {...register("name")} />
          </FormField>

          <FormField label="Base URL 模式">
            <RadioButtonGroup<ProviderBaseUrlMode>
              items={[
                { value: "order", label: "顺序" },
                { value: "ping", label: "Ping" },
              ]}
              ariaLabel="Base URL 模式"
              value={baseUrlMode}
              onChange={setBaseUrlMode}
              disabled={saving}
            />
          </FormField>
        </div>

        <FormField label="标签" hint="按 Enter 添加标签，用于分类筛选">
          <div className="flex flex-wrap items-center gap-1.5 rounded-md border border-slate-200 bg-white px-2 py-1.5 dark:border-slate-700 dark:bg-slate-800">
            {tags.map((tag) => (
              <span
                key={tag}
                className="inline-flex items-center gap-1 rounded-full bg-accent/10 px-2 py-0.5 text-xs font-medium text-accent"
              >
                {tag}
                <button
                  type="button"
                  onClick={() => setTags((prev) => prev.filter((t) => t !== tag))}
                  className="inline-flex h-3.5 w-3.5 items-center justify-center rounded-full hover:bg-accent/20"
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

        <FormField label="Base URLs">
          <BaseUrlEditor
            rows={baseUrlRows}
            setRows={setBaseUrlRowsFromUser}
            pingingAll={pingingAll}
            setPingingAll={setPingingAll}
            newRow={newBaseUrlRow}
            placeholder="中转 endpoint（例如：https://example.com/v1）"
            disabled={saving}
          />
        </FormField>

        <div className="grid gap-3 sm:grid-cols-2">
          <FormField label="认证方式">
            {mode === "create" ? (
              <RadioButtonGroup<AuthType>
                items={[
                  { value: "api_key", label: "API Key" },
                  { value: "oauth", label: "OAuth" },
                ]}
                ariaLabel="认证方式"
                value={authType}
                onChange={(value) => setValue("auth_type", value, { shouldDirty: true })}
                disabled={saving}
              />
            ) : (
              <span className="inline-flex items-center rounded-full bg-slate-100 px-3 py-1 text-xs font-medium text-slate-700 dark:bg-slate-700 dark:text-slate-300">
                {authType === "oauth" ? "OAuth" : "API Key"}
              </span>
            )}
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
        </div>

        {authType === "api_key" ? (
          <FormField
            label="API Key / Token"
            hint={mode === "edit" ? "留空保持不变" : "保存后不回显"}
          >
            <div className="flex items-center gap-2">
              <Input
                type="password"
                placeholder="sk-…"
                autoComplete="off"
                {...register("api_key")}
              />
            </div>
          </FormField>
        ) : (
          <>
            {mode === "create" ? (
              <FormField label="OAuth 类型" hint="选择 OAuth 提供商">
                <select
                  className="w-full rounded-md border border-slate-200 bg-white px-3 py-2 text-sm dark:border-slate-700 dark:bg-slate-800"
                  {...register("oauth_provider_type")}
                  disabled={saving}
                >
                  <option value="">-- 请选择 --</option>
                  <option value="claude_oauth">Claude</option>
                  <option value="codex_oauth">Codex</option>
                  <option value="gemini_oauth">Gemini</option>
                </select>
              </FormField>
            ) : null}

            {mode === "edit" && editingProviderId != null ? (
              <div className="rounded-xl border border-slate-200 bg-white p-4 shadow-sm dark:border-slate-700 dark:bg-slate-800">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <div className="text-sm font-medium text-slate-700 dark:text-slate-300">
                      OAuth 状态
                    </div>
                    {oauthStatus?.has_token ? (
                      <span className="rounded-full bg-emerald-50 px-2 py-0.5 text-[10px] font-medium text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400">
                        已认证
                      </span>
                    ) : (
                      <span className="rounded-full bg-slate-100 px-2 py-0.5 text-[10px] font-medium text-slate-500 dark:bg-slate-700 dark:text-slate-400">
                        未登录
                      </span>
                    )}
                    {oauthStatus?.quota_exceeded ? (
                      <span className="rounded-full bg-amber-50 px-2 py-0.5 text-[10px] font-medium text-amber-700 dark:bg-amber-900/30 dark:text-amber-400">
                        配额超限
                      </span>
                    ) : null}
                  </div>
                  <div className="flex gap-2">
                    {oauthStatus?.has_token ? (
                      <Button
                        variant="danger"
                        size="sm"
                        disabled={saving || oauthLoggingIn}
                        onClick={async () => {
                          const result = await oauthProviderLogout(editingProviderId);
                          if (result) setOauthStatus(result);
                        }}
                      >
                        <LogOut className="h-4 w-4" />
                        登出
                      </Button>
                    ) : (
                      <Button
                        variant="primary"
                        size="sm"
                        disabled={saving || oauthLoggingIn}
                        onClick={async () => {
                          setOauthLoggingIn(true);
                          try {
                            const result = await oauthStartLogin(editingProviderId);
                            if (result) setOauthStatus(result);
                          } catch (err) {
                            toast(`OAuth 登录失败: ${String(err)}`);
                          } finally {
                            setOauthLoggingIn(false);
                          }
                        }}
                      >
                        <LogIn className="h-4 w-4" />
                        {oauthLoggingIn ? "登录中…" : "浏览器登录"}
                      </Button>
                    )}
                  </div>
                </div>
                {oauthStatus?.last_error ? (
                  <p className="mt-2 text-xs text-rose-600 dark:text-rose-400">
                    {oauthStatus.last_error}
                  </p>
                ) : null}
                {oauthStatus?.expires_at ? (
                  <p className="mt-1 text-xs text-slate-400">
                    Token 过期时间: {new Date(oauthStatus.expires_at * 1000).toLocaleString()}
                  </p>
                ) : null}
              </div>
            ) : null}
          </>
        )}

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

        {cliKey === "claude" ? (
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
                    setClaudeModels((prev) => ({ ...prev, main_model: value }));
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
