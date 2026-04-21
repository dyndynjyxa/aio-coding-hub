import { useCallback, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useForm } from "react-hook-form";
import type { ClaudeModels, ProviderSummary } from "../../services/providers/providers";
import type { ProviderEditorDialogFormInput } from "../../schemas/providerEditorDialog";
import type { BaseUrlRow, ProviderBaseUrlMode } from "./types";
import type { ProviderEditorDialogProps } from "./ProviderEditorDialog";
import type {
  CopyApiKeyActionContext,
  OAuthActionContext,
  OAuthStatusValue,
  ProviderEditorPayloadContext,
  SaveActionContext,
} from "./providerEditorActionContext";
import {
  fetchProviderOAuthStatus,
  useProviderDeleteMutation,
  useProviderOAuthStatusQuery,
  useProviderUpsertMutation,
} from "../../query/providers";
import { useGatewayStatusQuery } from "../../query/gateway";
import { useSettingsQuery } from "../../query/settings";
import {
  DEFAULT_FORM_VALUES,
  CX2CC_GLOBAL_SOURCE_VALUE,
  deriveAuthMode,
  deriveCx2ccSourceValue,
  cliNameFromKey,
} from "./providerEditorUtils";
import {
  copyApiKey as copyApiKeyAction,
} from "./useProviderEditorActions";
import {
  handleOAuthLogin as oauthLoginAction,
  handleOAuthRefresh as oauthRefreshAction,
  handleOAuthDisconnect as oauthDisconnectAction,
} from "./providerEditorOAuthActions";
import { runProviderEditorSave } from "./providerEditorSaveRunner";
import { useProviderEditorEffects } from "./useProviderEditorEffects";

export function useProviderEditorForm(props: ProviderEditorDialogProps) {
  const { open, onOpenChange, onSaved, codexProviders = [] } = props;

  const mode = props.mode;
  const cliKey = mode === "create" ? props.cliKey : props.provider.cli_key;
  const createInitialValues = mode === "create" ? (props.initialValues ?? null) : null;
  const isDuplicating = mode === "create" && createInitialValues != null;
  const editingProviderId = mode === "edit" ? props.provider.id : null;
  const editProvider = mode === "edit" ? props.provider : null;

  const baseUrlRowSeqRef = useRef(1);
  const newBaseUrlRow = useCallback((url = ""): BaseUrlRow => {
    const id = String(baseUrlRowSeqRef.current++);
    return { id, url, ping: { status: "idle" } };
  }, []);

  const [baseUrlMode, setBaseUrlMode] = useState<ProviderBaseUrlMode>("order");
  const [baseUrlRows, setBaseUrlRows] = useState<BaseUrlRow[]>(() => [newBaseUrlRow()]);
  const [pingingAll, setPingingAll] = useState(false);
  const [claudeModels, setClaudeModels] = useState<ClaudeModels>({});
  const [tags, setTags] = useState<string[]>([]);
  const [tagInput, setTagInput] = useState("");
  const [streamIdleTimeoutSeconds, setStreamIdleTimeoutSeconds] = useState("");
  const [saving, setSaving] = useState(false);
  const [copyingApiKey, setCopyingApiKey] = useState(false);

  const [authMode, setAuthMode] = useState<"api_key" | "oauth" | "cx2cc">(
    deriveAuthMode(editProvider)
  );
  const [cx2ccSourceValue, setCx2ccSourceValue] = useState<string>(
    deriveCx2ccSourceValue(editProvider)
  );
  const [oauthStatus, setOauthStatus] = useState<OAuthStatusValue>(null);
  const [oauthLoading, setOauthLoading] = useState(false);
  const [cx2ccFallbackModels, setCx2ccFallbackModels] = useState<{
    main: string; haiku: string; sonnet: string; opus: string;
  } | null>(null);
  const [codexGatewayBaseOrigin, setCodexGatewayBaseOrigin] = useState<string | null>(null);
  const oauthStatusRequestSeqRef = useRef(0);
  const queryClient = useQueryClient();
  const providerUpsertMutation = useProviderUpsertMutation();
  const providerDeleteMutation = useProviderDeleteMutation();
  const claudeMetaEnabled = open && cliKey === "claude";
  const settingsQuery = useSettingsQuery({ enabled: claudeMetaEnabled });
  const gatewayStatusQuery = useGatewayStatusQuery({ enabled: claudeMetaEnabled });
  const oauthStatusQuery = useProviderOAuthStatusQuery(editingProviderId, {
    enabled: open && editProvider?.auth_mode === "oauth",
  });

  const form = useForm<ProviderEditorDialogFormInput>({ defaultValues: DEFAULT_FORM_VALUES });
  const editProviderSnapshotRef = useRef<ProviderSummary | null>(null);

  const { register, reset, setValue, watch } = form;
  const enabled = watch("enabled");
  const dailyResetMode = watch("daily_reset_mode");
  const limit5hUsd = watch("limit_5h_usd");
  const limitDailyUsd = watch("limit_daily_usd");
  const limitWeeklyUsd = watch("limit_weekly_usd");
  const limitMonthlyUsd = watch("limit_monthly_usd");
  const limitTotalUsd = watch("limit_total_usd");
  const apiKeyValue = watch("api_key");
  const costMultiplierValue = watch("cost_multiplier");
  const apiKeyConfigured = editProvider?.api_key_configured === true;
  const isCodexGatewaySource = cx2ccSourceValue === CX2CC_GLOBAL_SOURCE_VALUE;
  const sourceProviderId =
    cx2ccSourceValue && cx2ccSourceValue !== CX2CC_GLOBAL_SOURCE_VALUE
      ? Number(cx2ccSourceValue)
      : null;
  const selectedCx2ccSourceProvider = sourceProviderId
    ? (codexProviders.find((provider) => provider.id === sourceProviderId) ?? null)
    : null;
  const codexGatewayBaseUrl = codexGatewayBaseOrigin
    ? `${codexGatewayBaseOrigin.replace(/\/$/, "")}/v1`
    : "当前网关 /v1";

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

  const refreshOauthStatus = useCallback(
    (providerId?: number | null) => {
      return fetchProviderOAuthStatus(queryClient, providerId ?? editingProviderId);
    },
    [editingProviderId, queryClient]
  );

  useProviderEditorEffects({
    open, mode, cliKey, editProvider, editingProviderId, createInitialValues,
    authMode, costMultiplierValue, isCodexGatewaySource, selectedCx2ccSourceProvider,
    reset, setValue,
    editProviderSnapshotRef, baseUrlRowSeqRef, oauthStatusRequestSeqRef,
    newBaseUrlRow,
    setBaseUrlMode, setBaseUrlRows, setPingingAll, setClaudeModels, setTags, setTagInput,
    setStreamIdleTimeoutSeconds, setAuthMode, setCx2ccSourceValue, setOauthStatus,
    setOauthLoading, setCx2ccFallbackModels, setCodexGatewayBaseOrigin,
    settingsSnapshot: settingsQuery.data ?? null,
    gatewayStatusSnapshot: gatewayStatusQuery.data ?? null,
    oauthStatusSnapshot: oauthStatusQuery.data,
    oauthStatusError: oauthStatusQuery.error,
  });

  const apiKeyFieldReg = register("api_key");

  const claudeModelCount =
    cliKey === "claude"
      ? Object.values(claudeModels).filter((value) => {
          if (typeof value !== "string") return false;
          return Boolean(value.trim());
        }).length
      : 0;
  const supportsOAuth = cliKey === "codex" || cliKey === "gemini";
  const supportsCx2cc = cliKey === "claude";

  const buildPayloadContext = useCallback((): ProviderEditorPayloadContext => ({
    mode,
    cliKey,
    editingProviderId,
    authMode,
    baseUrlMode,
    baseUrlRows,
    tags,
    claudeModels,
    streamIdleTimeoutSeconds,
    apiKeyConfigured,
    isCodexGatewaySource,
    sourceProviderId,
    selectedCx2ccSourceProvider,
    formValues: form.getValues(),
  }), [
    mode,
    cliKey,
    editingProviderId,
    authMode,
    baseUrlMode,
    baseUrlRows,
    tags,
    claudeModels,
    streamIdleTimeoutSeconds,
    apiKeyConfigured,
    isCodexGatewaySource,
    sourceProviderId,
    selectedCx2ccSourceProvider,
    form,
  ]);

  const buildCopyApiKeyContext = useCallback((): CopyApiKeyActionContext => ({
    mode,
    cliKey,
    editingProviderId,
    editProvider,
    open,
    onOpenChange,
    onSaved,
    copyingApiKey,
    setCopyingApiKey,
    apiKeyConfigured,
    apiKeyValue,
  }), [
    mode,
    cliKey,
    editingProviderId,
    editProvider,
    open,
    onOpenChange,
    onSaved,
    copyingApiKey,
    apiKeyConfigured,
    apiKeyValue,
  ]);

  const buildSaveContext = useCallback((): SaveActionContext => ({
    editProvider,
    open,
    onOpenChange,
    onSaved,
    ...buildPayloadContext(),
    saving,
    setSaving,
    form: { getValues: form.getValues, setValue: form.setValue },
    oauthStatus,
    setOauthStatus,
    refreshOauthStatus,
    persistProvider: (input) => providerUpsertMutation.mutateAsync({ input }),
  }), [
    mode,
    cliKey,
    editingProviderId,
    editProvider,
    open,
    onOpenChange,
    onSaved,
    buildPayloadContext,
    saving,
    form.getValues,
    form.setValue,
    oauthStatus,
    refreshOauthStatus,
    providerUpsertMutation,
  ]);

  const buildOAuthContext = useCallback((): OAuthActionContext => ({
    editProvider,
    open,
    onOpenChange,
    onSaved,
    ...buildPayloadContext(),
    form: { getValues: form.getValues, setValue: form.setValue },
    oauthStatus,
    setOauthStatus,
    refreshOauthStatus,
    setOauthLoading,
    persistProvider: (input) => providerUpsertMutation.mutateAsync({ input }),
    removeProvider: (providerId) => providerDeleteMutation.mutateAsync({ cliKey, providerId }),
  }), [
    mode,
    cliKey,
    editingProviderId,
    editProvider,
    open,
    onOpenChange,
    onSaved,
    buildPayloadContext,
    form.getValues,
    form.setValue,
    oauthStatus,
    refreshOauthStatus,
    providerUpsertMutation,
    providerDeleteMutation,
  ]);

  return {
    mode, cliKey, open, onOpenChange,
    saving, title, description,
    authMode, setAuthMode, supportsOAuth, supportsCx2cc,
    register, setValue, watch,
    enabled, dailyResetMode,
    limit5hUsd, limitDailyUsd, limitWeeklyUsd, limitMonthlyUsd, limitTotalUsd,
    costMultiplierValue,
    apiKeyField: apiKeyFieldReg, apiKeyValue, apiKeyConfigured, copyingApiKey,
    tags, setTags, tagInput, setTagInput,
    baseUrlMode, setBaseUrlMode, baseUrlRows, setBaseUrlRows,
    pingingAll, setPingingAll, newBaseUrlRow,
    claudeModels, setClaudeModels, claudeModelCount,
    streamIdleTimeoutSeconds, setStreamIdleTimeoutSeconds,
    oauthStatus, oauthLoading,
    cx2ccSourceValue, setCx2ccSourceValue,
    isCodexGatewaySource, selectedCx2ccSourceProvider,
    codexGatewayBaseUrl, cx2ccFallbackModels, codexProviders,
    save: () => runProviderEditorSave(buildSaveContext()),
    copyApiKey: () => copyApiKeyAction(buildCopyApiKeyContext()),
    handleOAuthLogin: () => oauthLoginAction(buildOAuthContext()),
    handleOAuthRefresh: () => oauthRefreshAction(buildOAuthContext()),
    handleOAuthDisconnect: () => oauthDisconnectAction(buildOAuthContext()),
  };
}

export type UseProviderEditorFormReturn = ReturnType<typeof useProviderEditorForm>;
