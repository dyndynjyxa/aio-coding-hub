import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { logToConsole } from "../../services/consoleLog";
import type { AppSettings } from "../../services/settings/settings";
import {
  getSettingsReadProtection,
  SETTINGS_READONLY_MESSAGE,
} from "../../query/settings";
import {
  buildPersistedSettingsSnapshot,
  diffPersistedSettings,
  type PersistedSettings,
  DEFAULT_PERSISTED_SETTINGS,
} from "./settingsPersistenceModel";

type SettingsQueryState = {
  data: AppSettings | null | undefined;
  isLoading: boolean;
  isError: boolean;
  error: unknown;
  dataUpdatedAt?: number;
};

type UseSettingsPersistenceReadStateInput = {
  settingsQuery: SettingsQueryState;
  applySnapshot: (next: PersistedSettings) => void;
};

export function useSettingsPersistenceReadState(
  input: UseSettingsPersistenceReadStateInput
) {
  const { settingsQuery, applySnapshot } = input;
  const {
    data: settingsData,
    dataUpdatedAt,
    error: settingsError,
    isError: settingsIsError,
    isLoading: settingsLoading,
  } = settingsQuery;
  const [settingsReady, setSettingsReady] = useState(false);
  const [settingsReadErrorMessage, setSettingsReadErrorMessage] = useState<string | null>(null);

  const persistedSettingsRef = useRef<PersistedSettings>(DEFAULT_PERSISTED_SETTINGS);
  const desiredSettingsRef = useRef<PersistedSettings>(DEFAULT_PERSISTED_SETTINGS);
  const readFailureReportedRef = useRef<string | null>(null);
  const lastAppliedDataUpdatedAtRef = useRef<number | null>(null);
  const settingsWriteBlocked = settingsReadErrorMessage !== null;

  const reportSettingsReadFailure = useCallback((error: unknown) => {
    const errorText = String(error);
    if (readFailureReportedRef.current === errorText) {
      return;
    }

    logToConsole("error", "读取设置失败", { error: errorText });
    toast(SETTINGS_READONLY_MESSAGE);
    readFailureReportedRef.current = errorText;
  }, []);

  useEffect(() => {
    if (settingsLoading) {
      return;
    }

    const readProtection = getSettingsReadProtection({
      data: settingsData,
      isError: settingsIsError,
    });

    if (settingsData) {
      const nextSettings = buildPersistedSettingsSnapshot(settingsData);
      const nextUpdatedAt = dataUpdatedAt ?? 0;
      const hasFreshQueryData =
        lastAppliedDataUpdatedAtRef.current == null ||
        nextUpdatedAt > lastAppliedDataUpdatedAtRef.current;

      if (settingsWriteBlocked && !hasFreshQueryData) {
        setSettingsReady(true);
        return;
      }

      const shouldSyncForm =
        !settingsReady ||
        hasFreshQueryData ||
        diffPersistedSettings(persistedSettingsRef.current, nextSettings).length > 0;

      persistedSettingsRef.current = nextSettings;
      desiredSettingsRef.current = nextSettings;
      lastAppliedDataUpdatedAtRef.current = nextUpdatedAt;

      if (shouldSyncForm) {
        applySnapshot(nextSettings);
      }

      if (readProtection.settingsWriteBlocked) {
        reportSettingsReadFailure(settingsError);
        setSettingsReadErrorMessage(readProtection.settingsReadErrorMessage);
        setSettingsReady(true);
        return;
      }

      readFailureReportedRef.current = null;
      setSettingsReadErrorMessage(null);
      setSettingsReady(true);
      return;
    }

    if (readProtection.settingsWriteBlocked) {
      reportSettingsReadFailure(settingsError);
      setSettingsReadErrorMessage(readProtection.settingsReadErrorMessage);
      setSettingsReady(true);
      return;
    }

    readFailureReportedRef.current = null;
    setSettingsReadErrorMessage(null);
    if (!settingsReady) {
      setSettingsReady(true);
    }
  }, [
    applySnapshot,
    dataUpdatedAt,
    reportSettingsReadFailure,
    settingsData,
    settingsError,
    settingsIsError,
    settingsLoading,
    settingsReady,
    settingsWriteBlocked,
  ]);

  return {
    settingsReady,
    settingsReadErrorMessage,
    settingsWriteBlocked,
    setSettingsReadErrorMessage,
    reportSettingsReadFailure,
    persistedSettingsRef,
    desiredSettingsRef,
  };
}
