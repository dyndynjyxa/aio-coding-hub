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
import { useProvidersListQuery } from "../../../query/providers";
import { useCliEnvConflictsQuery } from "../../../query/cliProxy";
import { useCliProxyControls } from "../../../hooks/useCliProxyControls";
import { formatActionFailureToast, formatUnknownError } from "../../../utils/errors";
import type { CliManagerGrokTabProps } from "./GrokTab";

const EMPTY_GROK_PREFERENCES: GrokProxyPreferences = {
  model_id: "",
  api_backend: "responses",
};

export function useGrokTabDataModel({ enabled }: { enabled: boolean }) {
  const infoQuery = useCliManagerGrokInfoQuery({ enabled });
  const configQuery = useCliManagerGrokConfigQuery({ enabled });
  const configSetMutation = useCliManagerGrokConfigSetMutation();
  const providersQuery = useProvidersListQuery("grok", { enabled });
  const envConflictsQuery = useCliEnvConflictsQuery("grok", { enabled });
  const cliProxyControls = useCliProxyControls();

  const [preferencesDraft, setPreferencesDraft] =
    useState<GrokProxyPreferences>(EMPTY_GROK_PREFERENCES);
  const preferencesDirtyRef = useRef(false);
  const preferencesDraftRevisionRef = useRef(0);
  const effectiveModelId = configQuery.data?.effective_preferences.model_id;
  const effectiveApiBackend = configQuery.data?.effective_preferences.api_backend;

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
    });
  }, [configQuery.isError, effectiveApiBackend, effectiveModelId]);

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

  async function persistPreferences() {
    if (configSetMutation.isPending || !grokConfig || grokConfigError) return;
    const modelId = preferencesDraft.model_id.trim();
    if (!modelId) return;

    const nextPreferences: GrokProxyPreferences = {
      model_id: modelId,
      api_backend: preferencesDraft.api_backend,
    };
    const submittedRevision = preferencesDraftRevisionRef.current;

    try {
      const updated = await configSetMutation.mutateAsync(nextPreferences);
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

  async function refreshGrok() {
    await Promise.all([
      infoQuery.refetch(),
      configQuery.refetch(),
      providersQuery.refetch(),
      envConflictsQuery.refetch(),
    ]);
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

  const pendingCliProxyEnablePrompt =
    cliProxyControls.pendingCliProxyEnablePrompt?.cliKey === "grok"
      ? cliProxyControls.pendingCliProxyEnablePrompt
      : null;

  return {
    grokAvailable,
    grokLoading: infoQuery.isFetching,
    grokInfo,
    grokConfigLoading: configQuery.isFetching,
    grokConfigSaving: configSetMutation.isPending,
    grokConfig,
    grokConfigError,
    preferencesDraft,
    providers: providersQuery.data ?? null,
    providersLoading: providersQuery.isFetching,
    envConflicts: envConflictsQuery.data ?? null,
    envConflictsLoading: envConflictsQuery.isFetching,
    envConflictsError,
    cliProxyLoading: cliProxyControls.cliProxyLoading,
    cliProxyAvailable: cliProxyControls.cliProxyAvailable,
    cliProxyEnabled: cliProxyControls.cliProxyEnabled.grok,
    cliProxyAppliedToCurrentGateway: cliProxyControls.cliProxyAppliedToCurrentGateway.grok,
    cliProxyToggling: cliProxyControls.cliProxyToggling.grok,
    pendingCliProxyEnablePrompt,
    refreshGrok,
    openGrokConfigDir,
    setModelIdDraft,
    setApiBackendDraft,
    persistPreferences,
    requestCliProxyEnabledSwitch: (next: boolean) =>
      cliProxyControls.requestCliProxyEnabledSwitch("grok", next),
    clearPendingCliProxyEnablePrompt: () => cliProxyControls.setPendingCliProxyEnablePrompt(null),
    confirmPendingCliProxyEnable: cliProxyControls.confirmPendingCliProxyEnable,
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
