import { useCallback, useRef, useState } from "react";
import { toast } from "sonner";
import type { MutableRefObject } from "react";
import type { AppAboutInfo } from "../../services/app/appAbout";
import { logToConsole } from "../../services/consoleLog";
import {
  gatewayCheckPortAvailable,
  type GatewayStatus,
} from "../../services/gateway/gateway";
import type {
  SettingsMutationResult,
  SettingsSetInput,
} from "../../services/settings/settings";
import { SETTINGS_READONLY_MESSAGE } from "../../query/settings";
import { presentSettingsMutationFeedback } from "./settingsPersistenceFeedback";
import {
  buildPersistedSettingsSnapshot,
  buildPersistedSettingsMutationInput,
  diffPersistedSettings,
  replacePersistedSettingsKeys,
  validatePersistedSettings,
  type PersistKey,
  type PersistedSettings,
  type PersistedSettingsPatch,
} from "./settingsPersistenceModel";

function isSettingsReadFailure(err: unknown) {
  const text = String(err);
  return text.includes("SETTINGS_RECOVERY_REQUIRED") || text.includes("invalid settings.json");
}

type UseSettingsPersistRunnerInput = {
  gateway: GatewayStatus | null;
  about: AppAboutInfo | null;
  settingsReady: boolean;
  settingsWriteBlocked: boolean;
  settingsReadErrorMessage: string | null;
  persistedSettingsRef: MutableRefObject<PersistedSettings>;
  desiredSettingsRef: MutableRefObject<PersistedSettings>;
  setSettingsReadErrorMessage: (message: string | null) => void;
  reportSettingsReadFailure: (error: unknown) => void;
  settingsSetMutation: {
    mutateAsync: (input: SettingsSetInput) => Promise<SettingsMutationResult | null>;
  };
  reconcileSettledKeys: (
    desiredSnapshot: PersistedSettings,
    persistedSnapshot: PersistedSettings,
    settledKeys: PersistKey[]
  ) => void;
  revertKeys: (keys: PersistKey[], source: PersistedSettings) => void;
  setField: <TKey extends PersistKey>(
    key: TKey,
    value: PersistedSettings[TKey]
  ) => void;
};

export function useSettingsPersistRunner(input: UseSettingsPersistRunnerInput) {
  const {
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
  } = input;
  const [settingsSaving, setSettingsSaving] = useState(false);
  const persistQueueRef = useRef<{
    inFlight: boolean;
    pending: PersistedSettings | null;
  }>({ inFlight: false, pending: null });

  const revertSettledKeys = useCallback(
    (desiredSnapshot: PersistedSettings, keysToConsider: PersistKey[]) => {
      const desiredNow = desiredSettingsRef.current;
      const settledKeys = keysToConsider.filter((key) => desiredNow[key] === desiredSnapshot[key]);
      if (settledKeys.length === 0) {
        return;
      }

      desiredSettingsRef.current = replacePersistedSettingsKeys(
        desiredNow,
        persistedSettingsRef.current,
        settledKeys
      );
      revertKeys(settledKeys, persistedSettingsRef.current);

      if (persistQueueRef.current.pending) {
        persistQueueRef.current.pending = desiredSettingsRef.current;
      }
    },
    [desiredSettingsRef, persistedSettingsRef, revertKeys]
  );

  const blockPersist = useCallback(
    (keys: PersistKey[]) => {
      toast(settingsReadErrorMessage ?? SETTINGS_READONLY_MESSAGE);
      persistQueueRef.current.pending = null;
      desiredSettingsRef.current = replacePersistedSettingsKeys(
        desiredSettingsRef.current,
        persistedSettingsRef.current,
        keys
      );
      revertKeys(keys, persistedSettingsRef.current);
    },
    [desiredSettingsRef, persistedSettingsRef, revertKeys, settingsReadErrorMessage]
  );

  const persistSettings = useCallback(
    async (desiredSnapshot: PersistedSettings) => {
      const before = persistedSettingsRef.current;
      let desired = desiredSnapshot;
      let changedKeys = diffPersistedSettings(before, desired);
      if (changedKeys.length === 0) {
        return;
      }

      const validationError = validatePersistedSettings(desired, changedKeys);
      if (validationError) {
        toast(validationError);
        revertSettledKeys(desired, changedKeys);
        return;
      }

      if (
        changedKeys.includes("preferred_port") &&
        !(gateway?.running && gateway.port === desired.preferred_port)
      ) {
        if (desiredSettingsRef.current.preferred_port !== desired.preferred_port) {
          return;
        }

        let available: boolean | null;
        try {
          available = await gatewayCheckPortAvailable(desired.preferred_port);
        } catch (err) {
          revertSettledKeys(desired, ["preferred_port"]);
          if (isSettingsReadFailure(err)) {
            setSettingsReadErrorMessage(SETTINGS_READONLY_MESSAGE);
            toast(SETTINGS_READONLY_MESSAGE);
            return;
          }
          toast("检查端口可用性失败：请稍后重试");
          return;
        }

        if (available === false) {
          if (desiredSettingsRef.current.preferred_port === desired.preferred_port) {
            toast(`端口 ${desired.preferred_port} 已被占用，请换一个端口`);
            revertSettledKeys(desired, ["preferred_port"]);
            desired = replacePersistedSettingsKeys(desired, before, ["preferred_port"]);
          } else {
            return;
          }
        }
      }

      changedKeys = diffPersistedSettings(before, desired);
      if (changedKeys.length === 0) {
        return;
      }

      let nextResult: SettingsMutationResult | null;
      try {
        nextResult = await settingsSetMutation.mutateAsync(
          buildPersistedSettingsMutationInput(desired)
        );
      } catch (err) {
        if (isSettingsReadFailure(err)) {
          reportSettingsReadFailure(err);
          setSettingsReadErrorMessage(SETTINGS_READONLY_MESSAGE);
          revertSettledKeys(desired, changedKeys);
          return;
        }

        logToConsole("error", "更新设置失败", { error: String(err) });
        toast("更新设置失败：请稍后重试");
        revertSettledKeys(desired, changedKeys);
        return;
      }

      if (!nextResult) {
        revertSettledKeys(desired, changedKeys);
        return;
      }

      const after = buildPersistedSettingsSnapshot(nextResult.settings, desired);
      persistedSettingsRef.current = after;

      const desiredNow = desiredSettingsRef.current;
      const settledKeys = changedKeys.filter((key) => desiredNow[key] === desired[key]);
      if (settledKeys.length > 0) {
        desiredSettingsRef.current = replacePersistedSettingsKeys(desiredNow, after, settledKeys);
        reconcileSettledKeys(desired, after, settledKeys);
      }

      presentSettingsMutationFeedback({
        before,
        desired,
        after,
        settledKeys,
        result: nextResult,
        gateway,
        about,
      });
    },
    [
      about,
      desiredSettingsRef,
      gateway,
      persistedSettingsRef,
      reconcileSettledKeys,
      reportSettingsReadFailure,
      revertSettledKeys,
      setSettingsReadErrorMessage,
      settingsSetMutation,
    ]
  );

  const enqueuePersist = useCallback(
    (desiredSnapshot: PersistedSettings) => {
      if (!settingsReady || settingsWriteBlocked) {
        return;
      }

      const queue = persistQueueRef.current;
      if (queue.inFlight) {
        queue.pending = desiredSnapshot;
        return;
      }

      queue.inFlight = true;
      setSettingsSaving(true);
      void persistSettings(desiredSnapshot).finally(() => {
        const next = queue.pending;
        queue.pending = null;
        queue.inFlight = false;
        if (next) {
          enqueuePersist(next);
          return;
        }
        setSettingsSaving(false);
      });
    },
    [persistSettings, settingsReady, settingsWriteBlocked]
  );

  const requestPersist = useCallback(
    (patch: PersistedSettingsPatch) => {
      if (!settingsReady) {
        return;
      }

      const changedKeys = Object.keys(patch) as PersistKey[];
      if (settingsWriteBlocked) {
        blockPersist(changedKeys);
        return;
      }

      const nextDesired = { ...desiredSettingsRef.current, ...patch };
      desiredSettingsRef.current = nextDesired;
      enqueuePersist(nextDesired);
    },
    [blockPersist, desiredSettingsRef, enqueuePersist, settingsReady, settingsWriteBlocked]
  );

  const commitNumberField = useCallback(
    (options: {
      key: "preferred_port" | "log_retention_days";
      next: number;
      min: number;
      max: number;
      invalidMessage: string;
    }) => {
      if (!settingsReady) {
        return;
      }

      if (settingsWriteBlocked) {
        blockPersist([options.key]);
        return;
      }

      const normalized = Math.floor(options.next);
      if (!Number.isFinite(normalized) || normalized < options.min || normalized > options.max) {
        toast(options.invalidMessage);
        revertKeys([options.key], desiredSettingsRef.current);
        return;
      }

      setField(options.key, normalized as PersistedSettings[typeof options.key]);
      requestPersist({ [options.key]: normalized } as PersistedSettingsPatch);
    },
    [
      blockPersist,
      desiredSettingsRef,
      requestPersist,
      revertKeys,
      setField,
      settingsReady,
      settingsWriteBlocked,
    ]
  );

  return {
    settingsSaving,
    requestPersist,
    commitNumberField,
  };
}
