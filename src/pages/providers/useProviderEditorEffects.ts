import { useEffect, useRef } from "react";
import { toast } from "sonner";
import { logToConsole } from "../../services/consoleLog";
import {
  type ProviderOAuthStatusResult,
  type ClaudeModels,
  type ProviderSummary,
} from "../../services/providers/providers";
import type { GatewayStatus } from "../../services/gateway/gateway";
import type { AppSettings } from "../../services/settings/settings";
import type { ProviderEditorDialogFormInput } from "../../schemas/providerEditorDialog";
import type { BaseUrlRow, ProviderBaseUrlMode } from "./types";
import type { ProviderEditorInitialValues } from "./providerDuplicate";
import type { UseFormReset } from "react-hook-form";
import {
  valueOrEmpty,
  normalizeTagsForCostMultiplier,
  buildFormValues,
  buildBaseUrlRows,
  deriveAuthMode,
  deriveCx2ccSourceValue,
  withCx2ccDefaultModel,
} from "./providerEditorUtils";

export type EffectDeps = {
  open: boolean;
  mode: "create" | "edit";
  cliKey: string;
  editProvider: ProviderSummary | null;
  editingProviderId: number | null;
  createInitialValues: ProviderEditorInitialValues | null;
  authMode: "api_key" | "oauth" | "cx2cc";
  reset: UseFormReset<ProviderEditorDialogFormInput>;
  editProviderSnapshotRef: React.MutableRefObject<ProviderSummary | null>;
  baseUrlRowSeqRef: React.MutableRefObject<number>;
  cancelActiveOAuthLoginAttempt: (resetUi?: boolean) => void;
  newBaseUrlRow: (url?: string) => BaseUrlRow;
  setBaseUrlMode: (v: ProviderBaseUrlMode) => void;
  setBaseUrlRows: (v: BaseUrlRow[]) => void;
  setPingingAll: (v: boolean) => void;
  setClaudeModels: (v: ClaudeModels) => void;
  setTags: React.Dispatch<React.SetStateAction<string[]>>;
  setTagInput: (v: string) => void;
  setStreamIdleTimeoutSeconds: (v: string) => void;
  setAuthMode: (v: "api_key" | "oauth" | "cx2cc") => void;
  setCx2ccSourceValue: (v: string) => void;
  setOauthStatus: (v: ProviderOAuthStatusResult | null) => void;
  setOauthLoading: (v: boolean) => void;
  setCx2ccFallbackModels: (
    v: {
      main: string;
      haiku: string;
      sonnet: string;
      opus: string;
    } | null
  ) => void;
  setCodexGatewayBaseOrigin: (v: string | null) => void;
  settingsSnapshot: AppSettings | null;
  gatewayStatusSnapshot: GatewayStatus | null;
  oauthStatusSnapshot: ProviderOAuthStatusResult | null | undefined;
  oauthStatusError: unknown;
};

export function useProviderEditorEffects(d: EffectDeps) {
  const {
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
    settingsSnapshot,
    gatewayStatusSnapshot,
    oauthStatusSnapshot,
    oauthStatusError,
  } = d;
  const oauthStatusErrorRef = useRef<string | null>(null);

  useEffect(() => {
    if (mode !== "edit" || !open || !editProvider) return;
    editProviderSnapshotRef.current = editProvider;
  }, [editProvider, editProviderSnapshotRef, mode, open]);

  useEffect(() => {
    setOauthLoading(false);

    if (!open) {
      cancelActiveOAuthLoginAttempt();
      setOauthStatus(null);
      return () => {
        cancelActiveOAuthLoginAttempt(false);
      };
    }

    cancelActiveOAuthLoginAttempt();

    baseUrlRowSeqRef.current = 1;

    if (mode === "create") {
      setBaseUrlMode(createInitialValues?.base_url_mode ?? "order");
      setBaseUrlRows(buildBaseUrlRows(createInitialValues, newBaseUrlRow));
      setPingingAll(false);
      const initialCx2ccSourceValue = deriveCx2ccSourceValue(createInitialValues);
      setClaudeModels(
        initialCx2ccSourceValue
          ? withCx2ccDefaultModel(createInitialValues?.claude_models ?? {})
          : (createInitialValues?.claude_models ?? {})
      );
      setTags(
        normalizeTagsForCostMultiplier(
          createInitialValues?.tags ?? [],
          String(createInitialValues?.cost_multiplier ?? 1.0)
        )
      );
      setTagInput("");
      setStreamIdleTimeoutSeconds(valueOrEmpty(createInitialValues?.stream_idle_timeout_seconds));
      setCx2ccSourceValue(initialCx2ccSourceValue);
      setAuthMode(
        initialCx2ccSourceValue ? "cx2cc" : (createInitialValues?.auth_mode ?? "api_key")
      );
      setOauthStatus(null);
      reset(buildFormValues(createInitialValues));
      return () => {
        cancelActiveOAuthLoginAttempt(false);
      };
    }

    const snapshot = editProviderSnapshotRef.current;
    if (!snapshot) {
      return () => {
        cancelActiveOAuthLoginAttempt(false);
      };
    }

    const initialAuthMode = deriveAuthMode(snapshot);
    const initialCx2ccSourceValue = deriveCx2ccSourceValue(snapshot);
    setAuthMode(initialAuthMode);
    setCx2ccSourceValue(initialCx2ccSourceValue);
    setOauthStatus(null);
    setBaseUrlMode(snapshot.base_url_mode);
    setBaseUrlRows(snapshot.base_urls.map((url) => newBaseUrlRow(url)));
    setPingingAll(false);
    setClaudeModels(
      initialAuthMode === "cx2cc"
        ? withCx2ccDefaultModel(snapshot.claude_models ?? {})
        : (snapshot.claude_models ?? {})
    );
    setTags(
      normalizeTagsForCostMultiplier(snapshot.tags ?? [], String(snapshot.cost_multiplier ?? 1.0))
    );
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
      cancelActiveOAuthLoginAttempt(false);
    };
  }, [
    baseUrlRowSeqRef,
    cancelActiveOAuthLoginAttempt,
    cliKey,
    createInitialValues,
    editProviderSnapshotRef,
    editingProviderId,
    mode,
    newBaseUrlRow,
    open,
    reset,
    setAuthMode,
    setBaseUrlMode,
    setBaseUrlRows,
    setClaudeModels,
    setCx2ccSourceValue,
    setOauthLoading,
    setOauthStatus,
    setPingingAll,
    setStreamIdleTimeoutSeconds,
    setTagInput,
    setTags,
  ]);

  useEffect(() => {
    if (!open || authMode === "oauth") return;
    cancelActiveOAuthLoginAttempt();
  }, [authMode, cancelActiveOAuthLoginAttempt, open]);

  useEffect(() => {
    if (!open || cliKey !== "claude") return;

    if (settingsSnapshot) {
      setCx2ccFallbackModels({
        main: settingsSnapshot.cx2cc_fallback_model_main.trim(),
        haiku: settingsSnapshot.cx2cc_fallback_model_haiku.trim(),
        sonnet: settingsSnapshot.cx2cc_fallback_model_sonnet.trim(),
        opus: settingsSnapshot.cx2cc_fallback_model_opus.trim(),
      });
      setCodexGatewayBaseOrigin(
        gatewayStatusSnapshot?.base_url?.trim() ||
          `http://127.0.0.1:${settingsSnapshot.preferred_port}`
      );
      return;
    }

    setCx2ccFallbackModels(null);
    setCodexGatewayBaseOrigin(gatewayStatusSnapshot?.base_url?.trim() || null);
  }, [
    cliKey,
    gatewayStatusSnapshot?.base_url,
    open,
    setCodexGatewayBaseOrigin,
    setCx2ccFallbackModels,
    settingsSnapshot,
  ]);

  useEffect(() => {
    if (!open || editProvider?.auth_mode !== "oauth") return;
    if (oauthStatusSnapshot === undefined) return;
    oauthStatusErrorRef.current = null;
    setOauthStatus(oauthStatusSnapshot);
  }, [editProvider?.auth_mode, oauthStatusSnapshot, open, setOauthStatus]);

  useEffect(() => {
    if (!open || editProvider?.auth_mode !== "oauth" || !oauthStatusError) return;
    const errorText = String(oauthStatusError);
    if (oauthStatusErrorRef.current === errorText) return;
    oauthStatusErrorRef.current = errorText;
    logToConsole("error", "加载 OAuth 状态失败", {
      provider_id: editProvider.id,
      cli_key: editProvider.cli_key,
      error: errorText,
    });
    toast(`加载 OAuth 状态失败：${errorText}`);
  }, [editProvider?.auth_mode, editProvider?.cli_key, editProvider?.id, oauthStatusError, open]);
}
