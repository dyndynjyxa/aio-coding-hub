import { useCallback, useMemo, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useForm } from "react-hook-form";
import type { ActiveUiContribution, JsonValue } from "../../generated/bindings";
import type {
  ClaudeModels,
  ProviderExtensionValuesInput,
  ProviderOAuthDeviceCodeStartResult,
  ProviderSummary,
} from "../../services/providers/providers";
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
  normalizeTagsForCostMultiplier,
  withCx2ccDefaultModel,
} from "./providerEditorUtils";
import { copyApiKey as copyApiKeyAction } from "./useProviderEditorActions";
import {
  handleOAuthLogin as oauthLoginAction,
  handleOAuthDeviceLogin as oauthDeviceLoginAction,
  handleOAuthRefresh as oauthRefreshAction,
  handleOAuthDisconnect as oauthDisconnectAction,
} from "./providerEditorOAuthActions";
import { runProviderEditorSave } from "./providerEditorSaveRunner";
import { useProviderEditorEffects } from "./useProviderEditorEffects";
import { providerOAuthCancelDeviceFlow } from "../../services/providers/providers";
import { logToConsole } from "../../services/consoleLog";
import { useContributionsForSlot } from "../../plugins/contributions/useActiveContributions";
import { contributionKey, type ContributionValues } from "../../plugins/contributions/types";

type StoredProviderExtensionValues = ProviderSummary["extension_values"][number];

function isContributionValues(value: JsonValue | null | undefined): value is ContributionValues {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function extensionValueKey(pluginId: string, namespace: string) {
  return `${pluginId}\u0000${namespace}`;
}

function resolveExtensionNamespace(
  contribution: ActiveUiContribution,
  existingValues: StoredProviderExtensionValues[]
) {
  const declaredNamespace = contribution.providerExtensionNamespace;
  if (declaredNamespace) {
    const exactExisting = existingValues.find(
      (value) => value.pluginId === contribution.pluginId && value.namespace === declaredNamespace
    );
    if (exactExisting) return exactExisting.namespace;
  }

  return (
    existingValues.find((value) => value.pluginId === contribution.pluginId)?.namespace ??
    declaredNamespace ??
    contribution.pluginId
  );
}

function deriveExtensionValuesByContribution(
  contributions: ActiveUiContribution[],
  existingValues: StoredProviderExtensionValues[]
) {
  const next: Record<string, ContributionValues> = {};
  const valuesByPluginAndNamespace = new Map<string, StoredProviderExtensionValues>();
  const firstValueByPlugin = new Map<string, StoredProviderExtensionValues>();

  for (const value of existingValues) {
    valuesByPluginAndNamespace.set(extensionValueKey(value.pluginId, value.namespace), value);
    if (!firstValueByPlugin.has(value.pluginId)) {
      firstValueByPlugin.set(value.pluginId, value);
    }
  }

  for (const contribution of contributions) {
    const namespace = resolveExtensionNamespace(contribution, existingValues);
    const existing =
      valuesByPluginAndNamespace.get(extensionValueKey(contribution.pluginId, namespace)) ??
      firstValueByPlugin.get(contribution.pluginId);
    next[contributionKey(contribution)] = isContributionValues(existing?.values)
      ? { ...existing.values }
      : {};
  }

  return next;
}

function buildExtensionValuesInput(
  contributions: ActiveUiContribution[],
  valuesByContributionKey: Record<string, ContributionValues>,
  existingValues: StoredProviderExtensionValues[]
): ProviderExtensionValuesInput[] | null {
  if (contributions.length === 0) return null;

  const activeRows = new Map<string, ProviderExtensionValuesInput>();
  const activeKeys = new Set<string>();

  for (const contribution of contributions) {
    const namespace = resolveExtensionNamespace(contribution, existingValues);
    const rowKey = extensionValueKey(contribution.pluginId, namespace);
    activeKeys.add(rowKey);
    const existingRow = activeRows.get(rowKey);
    const nextValues = valuesByContributionKey[contributionKey(contribution)] ?? {};

    activeRows.set(rowKey, {
      pluginId: contribution.pluginId,
      namespace,
      values: {
        ...(isContributionValues(existingRow?.values) ? existingRow.values : {}),
        ...nextValues,
      },
    });
  }

  const preservedRows: ProviderExtensionValuesInput[] = [];
  for (const value of existingValues) {
    if (activeKeys.has(extensionValueKey(value.pluginId, value.namespace))) continue;
    preservedRows.push({
      pluginId: value.pluginId,
      namespace: value.namespace,
      values: value.values,
    });
  }

  return [...preservedRows, ...activeRows.values()];
}

type ExtensionValuesState = {
  resetKey: string;
  valuesByContributionKey: Record<string, ContributionValues>;
};

function buildExtensionValuesResetKey({
  open,
  mode,
  editingProviderId,
  contributionResetKey,
  existingExtensionValuesResetKey,
}: {
  open: boolean;
  mode: ProviderEditorDialogProps["mode"];
  editingProviderId: number | null;
  contributionResetKey: string;
  existingExtensionValuesResetKey: string;
}) {
  if (!open) return "closed";
  return [
    mode,
    editingProviderId ?? "new",
    contributionResetKey,
    mode === "edit" ? existingExtensionValuesResetKey : "",
  ].join(":");
}

function buildExtensionValuesState({
  resetKey,
  mode,
  providerEditorContributions,
  existingExtensionValues,
}: {
  resetKey: string;
  mode: ProviderEditorDialogProps["mode"];
  providerEditorContributions: ActiveUiContribution[];
  existingExtensionValues: StoredProviderExtensionValues[];
}): ExtensionValuesState {
  return {
    resetKey,
    valuesByContributionKey:
      resetKey === "closed"
        ? {}
        : deriveExtensionValuesByContribution(
            providerEditorContributions,
            mode === "edit" ? existingExtensionValues : []
          ),
  };
}

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

  const [authMode, setAuthMode] = useState<"api_key" | "oauth" | "cx2cc">(() =>
    deriveAuthMode(editProvider)
  );
  const [cx2ccSourceValue, setCx2ccSourceValue] = useState<string>(() =>
    deriveCx2ccSourceValue(editProvider)
  );
  const [oauthStatus, setOauthStatus] = useState<OAuthStatusValue>(null);
  const [oauthLoading, setOauthLoading] = useState(false);
  const [oauthDeviceFlow, setOauthDeviceFlow] = useState<ProviderOAuthDeviceCodeStartResult | null>(
    null
  );
  const [oauthDevicePolling, setOauthDevicePolling] = useState(false);
  const [oauthDeviceError, setOauthDeviceError] = useState<string | null>(null);
  const [cx2ccFallbackModels, setCx2ccFallbackModels] = useState<{
    main: string;
    haiku: string;
    sonnet: string;
    opus: string;
  } | null>(null);
  const [codexGatewayBaseOrigin, setCodexGatewayBaseOrigin] = useState<string | null>(null);
  const oauthLoginAttemptSeqRef = useRef(0);
  const activeOAuthDeviceFlowRef = useRef<string | null>(null);
  const queryClient = useQueryClient();
  const providerUpsertMutation = useProviderUpsertMutation();
  const providerDeleteMutation = useProviderDeleteMutation();
  const { contributions: providerEditorContributions } = useContributionsForSlot(
    "providers.editor.sections"
  );
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

  const syncFreeTagForCostMultiplier = useCallback((value: string) => {
    setTags((prev) => normalizeTagsForCostMultiplier(prev, value));
  }, []);

  const setCostMultiplierValue = useCallback(
    (value: string, options?: Parameters<typeof setValue>[2]) => {
      setValue("cost_multiplier", value, options);
      syncFreeTagForCostMultiplier(value);
    },
    [setValue, syncFreeTagForCostMultiplier]
  );

  const resolveCx2ccInheritedMultiplier = useCallback(
    (sourceValue: string) => {
      if (sourceValue === CX2CC_GLOBAL_SOURCE_VALUE) return "0";
      const sourceProvider = codexProviders.find((provider) => String(provider.id) === sourceValue);
      return String(sourceProvider?.cost_multiplier ?? 1.0);
    },
    [codexProviders]
  );

  const setAuthModeFromUi = useCallback(
    (next: "api_key" | "oauth" | "cx2cc") => {
      setAuthMode(next);
      if (next === "cx2cc") {
        setClaudeModels((prev) => withCx2ccDefaultModel(prev));
        setCostMultiplierValue(resolveCx2ccInheritedMultiplier(cx2ccSourceValue), {
          shouldDirty: true,
          shouldTouch: false,
          shouldValidate: false,
        });
      }
    },
    [cx2ccSourceValue, resolveCx2ccInheritedMultiplier, setCostMultiplierValue]
  );

  const setCx2ccSourceValueFromUi = useCallback(
    (value: string) => {
      setCx2ccSourceValue(value);
      if (authMode === "cx2cc") {
        setCostMultiplierValue(resolveCx2ccInheritedMultiplier(value), {
          shouldDirty: true,
          shouldTouch: false,
          shouldValidate: false,
        });
      }
    },
    [authMode, resolveCx2ccInheritedMultiplier, setCostMultiplierValue]
  );

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

  const editProviderExtensionValues = editProvider?.extension_values;
  const existingExtensionValues = useMemo(
    () => editProviderExtensionValues ?? [],
    [editProviderExtensionValues]
  );
  const contributionResetKey = providerEditorContributions
    .map((contribution) => `${contribution.pluginId}:${contribution.contributionId}`)
    .join("|");
  const existingExtensionValuesResetKey = useMemo(
    () =>
      JSON.stringify(
        existingExtensionValues.map((value) => [value.pluginId, value.namespace, value.values])
      ),
    [existingExtensionValues]
  );
  const extensionValuesResetKey = buildExtensionValuesResetKey({
    open,
    mode,
    editingProviderId,
    contributionResetKey,
    existingExtensionValuesResetKey,
  });
  const [extensionValuesState, setExtensionValuesState] = useState<ExtensionValuesState>(() =>
    buildExtensionValuesState({
      resetKey: extensionValuesResetKey,
      mode,
      providerEditorContributions,
      existingExtensionValues,
    })
  );
  let effectiveExtensionValuesState = extensionValuesState;

  if (extensionValuesState.resetKey !== extensionValuesResetKey) {
    effectiveExtensionValuesState = buildExtensionValuesState({
      resetKey: extensionValuesResetKey,
      mode,
      providerEditorContributions,
      existingExtensionValues,
    });
    setExtensionValuesState(effectiveExtensionValuesState);
  }
  const extensionValuesByContributionKey = effectiveExtensionValuesState.valuesByContributionKey;

  const setExtensionValue = useCallback(
    (contribution: ActiveUiContribution, fieldKey: string, value: JsonValue) => {
      const key = contributionKey(contribution);
      setExtensionValuesState((prev) => ({
        ...prev,
        valuesByContributionKey: {
          ...prev.valuesByContributionKey,
          [key]: {
            ...(prev.valuesByContributionKey[key] ?? {}),
            [fieldKey]: value,
          },
        },
      }));
    },
    []
  );

  const refreshOauthStatus = useCallback(
    (providerId?: number | null) => {
      return fetchProviderOAuthStatus(queryClient, providerId ?? editingProviderId);
    },
    [editingProviderId, queryClient]
  );

  const cancelOAuthDeviceFlow = useCallback((flowId: string) => {
    void providerOAuthCancelDeviceFlow(flowId).catch((err) => {
      logToConsole("warn", "取消设备码登录失败", { error: String(err) });
    });
  }, []);

  const clearActiveOAuthDeviceFlow = useCallback((flowId: string) => {
    if (activeOAuthDeviceFlowRef.current === flowId) {
      activeOAuthDeviceFlowRef.current = null;
    }
  }, []);

  const cancelActiveOAuthLoginAttempt = useCallback(
    (resetUi = true) => {
      oauthLoginAttemptSeqRef.current += 1;
      const activeFlowId = activeOAuthDeviceFlowRef.current;
      activeOAuthDeviceFlowRef.current = null;
      if (activeFlowId) {
        cancelOAuthDeviceFlow(activeFlowId);
      }
      if (!resetUi) return;
      setOauthDevicePolling(false);
      setOauthDeviceFlow(null);
      setOauthDeviceError(null);
      setOauthLoading(false);
    },
    [cancelOAuthDeviceFlow]
  );

  const beginOAuthLoginAttempt = useCallback(() => {
    cancelActiveOAuthLoginAttempt();
    oauthLoginAttemptSeqRef.current += 1;
    return oauthLoginAttemptSeqRef.current;
  }, [cancelActiveOAuthLoginAttempt]);

  const isOAuthLoginAttemptCurrent = useCallback((attemptId: number) => {
    return oauthLoginAttemptSeqRef.current === attemptId;
  }, []);

  const setActiveOAuthDeviceFlow = useCallback((attemptId: number, flowId: string) => {
    if (oauthLoginAttemptSeqRef.current === attemptId) {
      activeOAuthDeviceFlowRef.current = flowId;
    }
  }, []);

  const requestOpenChange = useCallback(
    (nextOpen: boolean) => {
      if (!nextOpen) {
        cancelActiveOAuthLoginAttempt();
      }
      onOpenChange(nextOpen);
    },
    [cancelActiveOAuthLoginAttempt, onOpenChange]
  );

  useProviderEditorEffects({
    open,
    mode,
    cliKey,
    editProvider,
    editingProviderId,
    createInitialValues,
    authMode,
    reset,
    editProviderSnapshotRef,
    baseUrlRowSeqRef,
    cancelActiveOAuthLoginAttempt,
    newBaseUrlRow,
    setBaseUrlMode,
    setBaseUrlRows,
    setPingingAll,
    setClaudeModels,
    setTags,
    setTagInput,
    setStreamIdleTimeoutSeconds,
    setAuthMode,
    setCx2ccSourceValue,
    setOauthStatus,
    setOauthLoading,
    setCx2ccFallbackModels,
    setCodexGatewayBaseOrigin,
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
  const supportsOAuth = cliKey === "codex" || cliKey === "gemini" || cliKey === "grok";
  const supportsCx2cc = cliKey === "claude";

  const buildPayloadContext = useCallback(
    (): ProviderEditorPayloadContext => ({
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
      extensionValues: buildExtensionValuesInput(
        providerEditorContributions,
        extensionValuesByContributionKey,
        mode === "edit" ? existingExtensionValues : []
      ),
    }),
    [
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
      providerEditorContributions,
      extensionValuesByContributionKey,
      existingExtensionValues,
    ]
  );

  const buildCopyApiKeyContext = useCallback(
    (): CopyApiKeyActionContext => ({
      mode,
      cliKey,
      editingProviderId,
      editProvider,
      open,
      onOpenChange: requestOpenChange,
      onSaved,
      copyingApiKey,
      setCopyingApiKey,
      apiKeyConfigured,
      apiKeyValue,
    }),
    [
      mode,
      cliKey,
      editingProviderId,
      editProvider,
      open,
      requestOpenChange,
      onSaved,
      copyingApiKey,
      apiKeyConfigured,
      apiKeyValue,
    ]
  );

  const buildSaveContext = useCallback(
    (): SaveActionContext => ({
      editProvider,
      open,
      onOpenChange: requestOpenChange,
      onSaved,
      ...buildPayloadContext(),
      saving,
      setSaving,
      form: { getValues: form.getValues, setValue: form.setValue },
      oauthStatus,
      setOauthStatus,
      refreshOauthStatus,
      persistProvider: (input) => providerUpsertMutation.mutateAsync({ input }),
    }),
    [
      editProvider,
      open,
      requestOpenChange,
      onSaved,
      buildPayloadContext,
      saving,
      form.getValues,
      form.setValue,
      oauthStatus,
      refreshOauthStatus,
      providerUpsertMutation,
    ]
  );

  const buildOAuthContext = useCallback(
    (): OAuthActionContext => ({
      editProvider,
      open,
      onOpenChange: requestOpenChange,
      onSaved,
      ...buildPayloadContext(),
      form: { getValues: form.getValues, setValue: form.setValue },
      oauthStatus,
      setOauthStatus,
      refreshOauthStatus,
      setOauthLoading,
      oauthDeviceFlow,
      setOauthDeviceFlow,
      oauthDevicePolling,
      setOauthDevicePolling,
      oauthDeviceError,
      setOauthDeviceError,
      persistProvider: (input) => providerUpsertMutation.mutateAsync({ input }),
      removeProvider: (providerId) => providerDeleteMutation.mutateAsync({ cliKey, providerId }),
      beginOAuthLoginAttempt,
      isOAuthLoginAttemptCurrent,
      cancelOAuthDeviceFlow,
      setActiveOAuthDeviceFlow,
      clearActiveOAuthDeviceFlow,
    }),
    [
      cliKey,
      editProvider,
      open,
      requestOpenChange,
      onSaved,
      buildPayloadContext,
      form.getValues,
      form.setValue,
      oauthStatus,
      oauthDeviceFlow,
      oauthDevicePolling,
      oauthDeviceError,
      refreshOauthStatus,
      providerUpsertMutation,
      providerDeleteMutation,
      beginOAuthLoginAttempt,
      isOAuthLoginAttemptCurrent,
      cancelOAuthDeviceFlow,
      setActiveOAuthDeviceFlow,
      clearActiveOAuthDeviceFlow,
    ]
  );

  return {
    mode,
    cliKey,
    open,
    onOpenChange: requestOpenChange,
    saving,
    title,
    description,
    authMode,
    setAuthMode: setAuthModeFromUi,
    supportsOAuth,
    supportsCx2cc,
    register,
    setValue,
    watch,
    enabled,
    dailyResetMode,
    limit5hUsd,
    limitDailyUsd,
    limitWeeklyUsd,
    limitMonthlyUsd,
    limitTotalUsd,
    costMultiplierValue,
    setCostMultiplierValue,
    syncFreeTagForCostMultiplier,
    apiKeyField: apiKeyFieldReg,
    apiKeyValue,
    apiKeyConfigured,
    copyingApiKey,
    tags,
    setTags,
    tagInput,
    setTagInput,
    baseUrlMode,
    setBaseUrlMode,
    baseUrlRows,
    setBaseUrlRows,
    pingingAll,
    setPingingAll,
    newBaseUrlRow,
    claudeModels,
    setClaudeModels,
    claudeModelCount,
    streamIdleTimeoutSeconds,
    setStreamIdleTimeoutSeconds,
    oauthStatus,
    oauthLoading,
    oauthDeviceFlow,
    oauthDevicePolling,
    oauthDeviceError,
    cx2ccSourceValue,
    setCx2ccSourceValue: setCx2ccSourceValueFromUi,
    isCodexGatewaySource,
    selectedCx2ccSourceProvider,
    codexGatewayBaseUrl,
    cx2ccFallbackModels,
    codexProviders,
    extensionValuesByContributionKey,
    setExtensionValue,
    save: () => runProviderEditorSave(buildSaveContext()),
    copyApiKey: () => copyApiKeyAction(buildCopyApiKeyContext()),
    handleOAuthLogin: () => oauthLoginAction(buildOAuthContext()),
    handleOAuthDeviceLogin: () => oauthDeviceLoginAction(buildOAuthContext()),
    handleOAuthRefresh: () => oauthRefreshAction(buildOAuthContext()),
    handleOAuthDisconnect: () => oauthDisconnectAction(buildOAuthContext()),
  };
}

export type UseProviderEditorFormReturn = ReturnType<typeof useProviderEditorForm>;
