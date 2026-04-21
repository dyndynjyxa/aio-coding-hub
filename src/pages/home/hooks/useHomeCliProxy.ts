// Usage:
// - Wraps useCliProxy with env-conflict checking logic before enabling a CLI proxy.
// - Manages the pending confirmation dialog state for environment variable conflicts.

import { useCallback, useMemo, useState } from "react";
import { logToConsole } from "../../../services/consoleLog";
import type { CliKey } from "../../../services/providers/providers";
import { envConflictsCheck, type EnvConflict } from "../../../services/cli/envConflicts";
import { useCliProxy } from "../../../hooks/useCliProxy";

export type PendingCliProxyEnablePrompt = {
  cliKey: CliKey;
  conflicts: EnvConflict[];
};

export type HomeCliProxyState = {
  cliProxyLoading: boolean;
  cliProxyAvailable: boolean | null;
  cliProxyEnabled: Record<CliKey, boolean>;
  cliProxyAppliedToCurrentGateway: Record<CliKey, boolean | null>;
  cliProxyToggling: Record<CliKey, boolean>;
  pendingCliProxyEnablePrompt: PendingCliProxyEnablePrompt | null;
  setPendingCliProxyEnablePrompt: (v: PendingCliProxyEnablePrompt | null) => void;
  requestCliProxyEnabledSwitch: (cliKey: CliKey, next: boolean) => void;
  confirmPendingCliProxyEnable: () => void;
};

export function useHomeCliProxy(): HomeCliProxyState {
  const cliProxy = useCliProxy();

  const [pendingCliProxyEnablePrompt, setPendingCliProxyEnablePrompt] =
    useState<PendingCliProxyEnablePrompt | null>(null);
  const [checkingCliProxyCliKey, setCheckingCliProxyCliKey] = useState<CliKey | null>(null);

  const { setCliProxyEnabled } = cliProxy;
  const cliProxyToggling = useMemo<Record<CliKey, boolean>>(() => {
    if (!checkingCliProxyCliKey) return cliProxy.toggling;
    return { ...cliProxy.toggling, [checkingCliProxyCliKey]: true };
  }, [checkingCliProxyCliKey, cliProxy.toggling]);

  const requestCliProxyEnabledSwitch = useCallback(
    (cliKey: CliKey, next: boolean) => {
      if (next === false) {
        setCliProxyEnabled(cliKey, false);
        return;
      }

      if (checkingCliProxyCliKey === cliKey) return;
      setCheckingCliProxyCliKey(cliKey);

      void (async () => {
        try {
          const conflicts = await envConflictsCheck(cliKey);
          if (!conflicts || conflicts.length === 0) {
            setCliProxyEnabled(cliKey, true);
            return;
          }
          setPendingCliProxyEnablePrompt({ cliKey, conflicts });
        } catch (err) {
          logToConsole("error", "检查环境变量冲突失败，仍尝试开启 CLI 代理", {
            cli: cliKey,
            error: String(err),
          });
          setCliProxyEnabled(cliKey, true);
        } finally {
          setCheckingCliProxyCliKey(null);
        }
      })();
    },
    [checkingCliProxyCliKey, setCliProxyEnabled]
  );

  const confirmPendingCliProxyEnable = useCallback(() => {
    const pending = pendingCliProxyEnablePrompt;
    if (!pending) return;
    setPendingCliProxyEnablePrompt(null);
    setCliProxyEnabled(pending.cliKey, true);
  }, [pendingCliProxyEnablePrompt, setCliProxyEnabled]);

  return {
    cliProxyLoading: cliProxy.loading,
    cliProxyAvailable: cliProxy.available,
    cliProxyEnabled: cliProxy.enabled,
    cliProxyAppliedToCurrentGateway: cliProxy.appliedToCurrentGateway,
    cliProxyToggling,
    pendingCliProxyEnablePrompt,
    setPendingCliProxyEnablePrompt,
    requestCliProxyEnabledSwitch,
    confirmPendingCliProxyEnable,
  };
}
