import { useCallback, useMemo, useRef, useState } from "react";
import { toast } from "sonner";
import { logToConsole } from "../services/consoleLog";
import type { CliKey } from "../services/providers/providers";
import { useCliProxySetEnabledMutation, useCliProxyStatusAllQuery } from "../query/cliProxy";

const DEFAULT_ENABLED: Record<CliKey, boolean> = {
  claude: true,
  codex: false,
  gemini: false,
};

const DEFAULT_TOGGLING: Record<CliKey, boolean> = {
  claude: false,
  codex: false,
  gemini: false,
};

const DEFAULT_APPLIED_TO_CURRENT_GATEWAY: Record<CliKey, boolean | null> = {
  claude: null,
  codex: null,
  gemini: null,
};

export function useCliProxy() {
  const [toggling, setToggling] = useState<Record<CliKey, boolean>>(DEFAULT_TOGGLING);
  const togglingRef = useRef(toggling);
  togglingRef.current = toggling;

  const statusQuery = useCliProxyStatusAllQuery();
  const setEnabledMutation = useCliProxySetEnabledMutation();
  const loading = Boolean(statusQuery.isLoading);
  const available = loading ? null : statusQuery.data != null;

  const enabled = useMemo<Record<CliKey, boolean>>(() => {
    const next: Record<CliKey, boolean> = { ...DEFAULT_ENABLED };
    const statuses = statusQuery.data ?? null;
    if (!statuses) return next;
    for (const row of statuses) {
      if (row.cli_key in next) {
        next[row.cli_key as CliKey] = Boolean(row.enabled);
      }
    }
    return next;
  }, [statusQuery.data]);

  const appliedToCurrentGateway = useMemo<Record<CliKey, boolean | null>>(() => {
    const next: Record<CliKey, boolean | null> = { ...DEFAULT_APPLIED_TO_CURRENT_GATEWAY };
    const statuses = statusQuery.data ?? null;
    if (!statuses) return next;
    for (const row of statuses) {
      if (row.cli_key in next) {
        next[row.cli_key as CliKey] = row.applied_to_current_gateway ?? null;
      }
    }
    return next;
  }, [statusQuery.data]);

  const refresh = useCallback(() => {
    void statusQuery.refetch();
    return () => {};
  }, [statusQuery]);

  const setCliProxyEnabled = useCallback(
    (cliKey: CliKey, next: boolean) => {
      if (togglingRef.current[cliKey]) return;

      setToggling((cur) => ({ ...cur, [cliKey]: true }));

      void (async () => {
        try {
          const res = await setEnabledMutation.mutateAsync({ cliKey, enabled: next });
          if (!res) {
            return;
          }

          if (res.ok) {
            toast(res.message || (next ? "已开启代理" : "已关闭代理"));
            logToConsole("info", next ? "开启 CLI 代理" : "关闭 CLI 代理", res);
            return;
          }

          toast(res.message ? `操作失败：${res.message}` : "操作失败");
          logToConsole("error", next ? "开启 CLI 代理失败" : "关闭 CLI 代理失败", res);
        } catch (err) {
          toast(`操作失败：${String(err)}`);
          logToConsole("error", "切换 CLI 代理失败", {
            cli: cliKey,
            enabled: next,
            error: String(err),
          });

          // Best-effort rollback: optimistic update may have already applied.
          await statusQuery.refetch();
        } finally {
          setToggling((cur) => ({ ...cur, [cliKey]: false }));
        }
      })();
    },
    [setEnabledMutation, statusQuery]
  );

  return {
    loading,
    available,
    enabled,
    appliedToCurrentGateway,
    toggling,
    refresh,
    setCliProxyEnabled,
  };
}
