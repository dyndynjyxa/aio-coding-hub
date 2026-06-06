import { useCallback } from "react";
import type { AppAboutInfo } from "../../services/app/appAbout";
import type { CliKey } from "../../services/providers/providers";
import type { GatewayStatus } from "../../services/gateway/gateway";
import { useSettingsQuery, useSettingsSetMutation } from "../../query/settings";
import { type PersistedSettings } from "./settingsPersistenceModel";
import { useSettingsFormController } from "./useSettingsFormController";
import { useSettingsPersistenceReadState } from "./useSettingsPersistenceReadState";
import { useSettingsPersistRunner } from "./useSettingsPersistRunner";

export function useSettingsPersistence(options: {
  gateway: GatewayStatus | null;
  about: AppAboutInfo | null;
}) {
  const { gateway, about } = options;

  const settingsQuery = useSettingsQuery();
  const settingsSetMutation = useSettingsSetMutation();
  const { draft, applySnapshot, setField, revertKeys, reconcileSettledKeys } =
    useSettingsFormController();
  const {
    settingsReady,
    settingsReadErrorMessage,
    settingsWriteBlocked,
    setSettingsReadErrorMessage,
    reportSettingsReadFailure,
    persistedSettingsRef,
    desiredSettingsRef,
  } = useSettingsPersistenceReadState({
    settingsQuery: {
      data: settingsQuery.data ?? null,
      isLoading: settingsQuery.isLoading,
      isError: settingsQuery.isError,
      error: settingsQuery.error,
      dataUpdatedAt: settingsQuery.dataUpdatedAt,
    },
    applySnapshot,
  });

  const { settingsSaving, requestPersist, commitNumberField } = useSettingsPersistRunner({
    gateway,
    about,
    settingsReady,
    settingsWriteBlocked,
    settingsReadErrorMessage,
    persistedSettingsRef,
    desiredSettingsRef,
    setSettingsReadErrorMessage,
    reportSettingsReadFailure,
    settingsSetMutation,
    reconcileSettledKeys,
    revertKeys,
    setField,
  });

  const setPort = useCallback(
    (next: number) => {
      setField("preferred_port", next);
    },
    [setField]
  );
  const setShowHomeHeatmap = useCallback(
    (next: boolean) => {
      setField("show_home_heatmap", next);
    },
    [setField]
  );
  const setShowHomeUsage = useCallback(
    (next: boolean) => {
      setField("show_home_usage", next);
    },
    [setField]
  );
  const setHomeUsagePeriod = useCallback(
    (next: PersistedSettings["home_usage_period"]) => {
      setField("home_usage_period", next);
    },
    [setField]
  );
  const setCliPriorityOrder = useCallback(
    (next: CliKey[]) => {
      setField("cli_priority_order", next);
    },
    [setField]
  );
  const setAutoStart = useCallback(
    (next: boolean) => {
      setField("auto_start", next);
    },
    [setField]
  );
  const setStartMinimized = useCallback(
    (next: boolean) => {
      setField("start_minimized", next);
    },
    [setField]
  );
  const setTrayEnabled = useCallback(
    (next: boolean) => {
      setField("tray_enabled", next);
    },
    [setField]
  );
  const setLogRetentionDays = useCallback(
    (next: number) => {
      setField("log_retention_days", next);
    },
    [setField]
  );
  const setEnableDebugLog = useCallback(
    (next: boolean) => {
      setField("enable_debug_log", next);
    },
    [setField]
  );
  const setEnableAssociationAudit = useCallback(
    (next: boolean) => {
      setField("enable_association_audit", next);
    },
    [setField]
  );
  const setAssociationAuditProviderId = useCallback(
    (next: number | null) => {
      setField("association_audit_provider_id", next);
    },
    [setField]
  );
  const setAssociationAuditModel = useCallback(
    (next: string) => {
      setField("association_audit_model", next);
    },
    [setField]
  );
  const setAssociationAuditMode = useCallback(
    (next: PersistedSettings["association_audit_mode"]) => {
      setField("association_audit_mode", next);
    },
    [setField]
  );
  const setAssociationAuditSampleRate = useCallback(
    (next: number) => {
      setField("association_audit_sample_rate", next);
    },
    [setField]
  );
  const setAssociationAuditTimeoutSeconds = useCallback(
    (next: number) => {
      setField("association_audit_timeout_seconds", next);
    },
    [setField]
  );
  const setAssociationAuditMaxInputChars = useCallback(
    (next: number) => {
      setField("association_audit_max_input_chars", next);
    },
    [setField]
  );
  const setAssociationAuditMaxOutputChars = useCallback(
    (next: number) => {
      setField("association_audit_max_output_chars", next);
    },
    [setField]
  );

  return {
    settingsReady,
    settingsReadErrorMessage,
    settingsWriteBlocked,
    settingsSaving,

    port: draft.preferred_port,
    setPort,
    showHomeHeatmap: draft.show_home_heatmap,
    setShowHomeHeatmap,
    showHomeUsage: draft.show_home_usage,
    setShowHomeUsage,
    homeUsagePeriod: draft.home_usage_period,
    setHomeUsagePeriod,
    cliPriorityOrder: draft.cli_priority_order,
    setCliPriorityOrder,
    autoStart: draft.auto_start,
    setAutoStart,
    startMinimized: draft.start_minimized,
    setStartMinimized,
    trayEnabled: draft.tray_enabled,
    setTrayEnabled,
    logRetentionDays: draft.log_retention_days,
    setLogRetentionDays,
    enableDebugLog: draft.enable_debug_log,
    setEnableDebugLog,
    enableAssociationAudit: draft.enable_association_audit,
    setEnableAssociationAudit,
    associationAuditProviderId: draft.association_audit_provider_id,
    setAssociationAuditProviderId,
    associationAuditModel: draft.association_audit_model,
    setAssociationAuditModel,
    associationAuditMode: draft.association_audit_mode,
    setAssociationAuditMode,
    associationAuditSampleRate: draft.association_audit_sample_rate,
    setAssociationAuditSampleRate,
    associationAuditTimeoutSeconds: draft.association_audit_timeout_seconds,
    setAssociationAuditTimeoutSeconds,
    associationAuditMaxInputChars: draft.association_audit_max_input_chars,
    setAssociationAuditMaxInputChars,
    associationAuditMaxOutputChars: draft.association_audit_max_output_chars,
    setAssociationAuditMaxOutputChars,

    requestPersist,
    commitNumberField,
  };
}
