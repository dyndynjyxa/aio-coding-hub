import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import type { GrokApiBackend, GrokProxyPreferences } from "../../../services/cli/cliManager";
import { openDesktopPath } from "../../../services/desktop/opener";
import { logToConsole } from "../../../services/consoleLog";
import {
  useCliManagerGrokConfigQuery,
  useCliManagerGrokConfigSetMutation,
  useCliManagerGrokInfoQuery,
} from "../../../query/cliManager";
import { useCliEnvConflictsQuery } from "../../../query/cliProxy";
import { formatActionFailureToast, formatUnknownError } from "../../../utils/errors";
import type { CliManagerGrokTabProps } from "./GrokTab";

const EMPTY_GROK_PREFERENCES: GrokProxyPreferences = {
  model_id: "",
  api_backend: "responses",
  context_window: null,
  telemetry: null,
  supports_backend_search: null,
};

function normalizeContextWindow(value: number | null | undefined): number | null {
  return value != null && Number.isSafeInteger(value) && value > 0 ? value : null;
}

function buildGrokPreferences(
  current: GrokProxyPreferences,
  changes: Partial<GrokProxyPreferences>
): GrokProxyPreferences {
  const next = { ...current, ...changes };
  return {
    model_id: next.model_id,
    api_backend: next.api_backend,
    context_window: normalizeContextWindow(next.context_window),
    telemetry: next.telemetry ?? null,
    supports_backend_search: next.supports_backend_search ?? null,
  };
}

export function useGrokTabDataModel({ enabled }: { enabled: boolean }) {
  const infoQuery = useCliManagerGrokInfoQuery({ enabled });
  const configQuery = useCliManagerGrokConfigQuery({ enabled });
  const configSetMutation = useCliManagerGrokConfigSetMutation();
  const envConflictsQuery = useCliEnvConflictsQuery("grok", { enabled });

  const [preferencesDraft, setPreferencesDraft] =
    useState<GrokProxyPreferences>(EMPTY_GROK_PREFERENCES);
  const preferencesDirtyRef = useRef(false);
  const preferencesDraftRevisionRef = useRef(0);
  const effectiveModelId = configQuery.data?.effective_preferences.model_id;
  const effectiveApiBackend = configQuery.data?.effective_preferences.api_backend;
  const effectiveContextWindow = configQuery.data?.effective_preferences.context_window ?? null;
  const effectiveTelemetry = configQuery.data?.effective_preferences.telemetry ?? null;
  const effectiveSupportsBackendSearch =
    configQuery.data?.effective_preferences.supports_backend_search ?? null;

  useEffect(() => {
    if (preferencesDirtyRef.current) return;
    if (configQuery.isError) {
      setPreferencesDraft(EMPTY_GROK_PREFERENCES);
      return;
    }
    if (!effectiveModelId || !effectiveApiBackend) return;
    setPreferencesDraft({
      model_id: effectiveModelId,
      api_backend: effectiveApiBackend,
      context_window: effectiveContextWindow,
      telemetry: effectiveTelemetry,
      supports_backend_search: effectiveSupportsBackendSearch,
    });
  }, [
    configQuery.isError,
    effectiveApiBackend,
    effectiveContextWindow,
    effectiveModelId,
    effectiveSupportsBackendSearch,
    effectiveTelemetry,
  ]);

  async function doPersist(next: GrokProxyPreferences) {
    const modelId = next.model_id.trim();
    if (!modelId) {
      toast("模型 ID 不能为空");
      return;
    }
    if (configSetMutation.isPending || !grokConfig || grokConfigError) return;

    const normalized = buildGrokPreferences(next, { model_id: modelId });

    const submittedRevision = preferencesDraftRevisionRef.current;

    try {
      const updated = await configSetMutation.mutateAsync(normalized);
      if (!updated) return;
      if (preferencesDraftRevisionRef.current === submittedRevision) {
        preferencesDirtyRef.current = false;
        setPreferencesDraft(updated.effective_preferences);
      }
      toast("已保存 Grok 网关偏好");
    } catch (error) {
      const formatted = formatActionFailureToast("保存 Grok 网关偏好", error);
      logToConsole("error", "保存 Grok 网关偏好失败", {
        error: formatted.raw,
        error_code: formatted.error_code ?? undefined,
      });
      toast(formatted.toast);
    }
  }

  async function persistModelId(modelId: string) {
    preferencesDirtyRef.current = true;
    const next = buildGrokPreferences(preferencesDraft, { model_id: modelId });
    preferencesDraftRevisionRef.current += 1;
    setPreferencesDraft(next);
    await doPersist(next);
  }

  async function persistApiBackend(apiBackend: GrokApiBackend) {
    preferencesDirtyRef.current = true;
    const next = buildGrokPreferences(preferencesDraft, { api_backend: apiBackend });
    preferencesDraftRevisionRef.current += 1;
    setPreferencesDraft(next);
    await doPersist(next);
  }

  async function persistContextWindow(contextWindow: number | null) {
    preferencesDirtyRef.current = true;
    const next = buildGrokPreferences(preferencesDraft, { context_window: contextWindow });
    preferencesDraftRevisionRef.current += 1;
    setPreferencesDraft(next);
    await doPersist(next);
  }

  async function persistTelemetry(telemetry: boolean | null) {
    preferencesDirtyRef.current = true;
    const next = buildGrokPreferences(preferencesDraft, { telemetry });
    preferencesDraftRevisionRef.current += 1;
    setPreferencesDraft(next);
    await doPersist(next);
  }

  async function persistSupportsBackendSearch(supportsBackendSearch: boolean | null) {
    preferencesDirtyRef.current = true;
    const next = buildGrokPreferences(preferencesDraft, {
      supports_backend_search: supportsBackendSearch,
    });
    preferencesDraftRevisionRef.current += 1;
    setPreferencesDraft(next);
    await doPersist(next);
  }

  const grokInfo = infoQuery.data ?? null;
  const grokConfig = configQuery.data ?? null;
  const grokAvailable: CliManagerGrokTabProps["grokAvailable"] =
    infoQuery.isFetching && !grokInfo
      ? "checking"
      : grokInfo?.found === true
        ? "available"
        : "unavailable";
  const grokConfigError = configQuery.isError ? formatUnknownError(configQuery.error) : null;
  const envConflictsError = envConflictsQuery.isError
    ? formatUnknownError(envConflictsQuery.error)
    : null;

  function setModelIdDraft(modelId: string) {
    preferencesDirtyRef.current = true;
    preferencesDraftRevisionRef.current += 1;
    setPreferencesDraft((current) => ({ ...current, model_id: modelId }));
  }

  function setApiBackendDraft(apiBackend: GrokApiBackend) {
    preferencesDirtyRef.current = true;
    preferencesDraftRevisionRef.current += 1;
    setPreferencesDraft((current) => ({ ...current, api_backend: apiBackend }));
  }

  function setContextWindowDraft(contextWindow: number | null) {
    preferencesDirtyRef.current = true;
    preferencesDraftRevisionRef.current += 1;
    const normalized = normalizeContextWindow(contextWindow);
    setPreferencesDraft((current) => ({ ...current, context_window: normalized }));
  }

  function setTelemetryDraft(telemetry: boolean | null) {
    preferencesDirtyRef.current = true;
    preferencesDraftRevisionRef.current += 1;
    setPreferencesDraft((current) => ({ ...current, telemetry }));
  }

  function setSupportsBackendSearchDraft(supportsBackendSearch: boolean | null) {
    preferencesDirtyRef.current = true;
    preferencesDraftRevisionRef.current += 1;
    setPreferencesDraft((current) => ({
      ...current,
      supports_backend_search: supportsBackendSearch,
    }));
  }

  async function refreshGrok() {
    await Promise.all([infoQuery.refetch(), configQuery.refetch(), envConflictsQuery.refetch()]);
  }

  async function openGrokConfigDir() {
    if (grokInfo?.found !== true || !grokConfig) return;
    const configDir = configDirectory(grokConfig.config_path);
    if (!configDir) return;

    try {
      await openDesktopPath(configDir);
    } catch (error) {
      logToConsole("error", "打开 Grok 配置目录失败", {
        error: formatUnknownError(error),
      });
      toast("打开 Grok 配置目录失败：请查看控制台日志");
    }
  }

  return {
    grokAvailable,
    grokLoading: infoQuery.isFetching,
    grokInfo,
    grokConfigLoading: configQuery.isFetching,
    grokConfigSaving: configSetMutation.isPending,
    grokConfig,
    grokConfigError,
    preferencesDraft,
    envConflicts: envConflictsQuery.data ?? null,
    envConflictsLoading: envConflictsQuery.isFetching,
    envConflictsError,
    refreshGrok,
    openGrokConfigDir,
    setModelIdDraft,
    setApiBackendDraft,
    setContextWindowDraft,
    setTelemetryDraft,
    setSupportsBackendSearchDraft,
    persistModelId,
    persistApiBackend,
    persistContextWindow,
    persistTelemetry,
    persistSupportsBackendSearch,
  } satisfies CliManagerGrokTabProps;
}

function configDirectory(configPath: string) {
  const normalized = configPath.trim().replace(/[\\/]+$/, "");
  const separatorIndex = Math.max(normalized.lastIndexOf("/"), normalized.lastIndexOf("\\"));
  if (separatorIndex < 0) return null;
  if (separatorIndex === 0) return normalized.slice(0, 1);

  const directory = normalized.slice(0, separatorIndex);
  if (/^[A-Za-z]:$/.test(directory)) {
    return normalized.slice(0, separatorIndex + 1);
  }
  return directory;
}
