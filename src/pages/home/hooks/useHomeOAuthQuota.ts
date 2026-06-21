import { useCallback, useMemo, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import type { HomeOAuthQuotaRow } from "../../../components/home/homeOAuthQuotaTypes";
import {
  readProviderOAuthLimitsCache,
  refreshProviderOAuthLimits,
  resetProviderOAuthCodexQuota,
  useProvidersListQuery,
} from "../../../query/providers";
import type { RequestLogSummary } from "../../../services/gateway/requestLogs";
import type { CliKey, ProviderSummary } from "../../../services/providers/providers";

type UseHomeOAuthQuotaOptions = {
  cliPriorityOrder: CliKey[];
  requestLogs?: RequestLogSummary[];
  enabled?: boolean;
};

export type UseHomeOAuthQuotaResult = {
  oauthQuotaRows: HomeOAuthQuotaRow[];
  oauthQuotaVisible: boolean;
  oauthQuotaRefreshing: boolean;
  oauthQuotaHasRefreshed: boolean;
  refreshOAuthQuota: () => Promise<void>;
  refreshOAuthQuotaRow: (providerId: number) => Promise<void>;
  resetOAuthQuotaRow: (providerId: number) => Promise<void>;
};

type OAuthProviderSummary = {
  providerId: number;
  cliKey: CliKey;
  providerName: string;
  enabled: boolean;
};

function readOAuthProviders(rows: ProviderSummary[] | null | undefined): OAuthProviderSummary[] {
  if (!rows?.length) return [];
  return rows
    .filter((row) => row.auth_mode === "oauth")
    .map((row) => ({
      providerId: row.id,
      cliKey: row.cli_key,
      providerName: row.name,
      enabled: row.enabled,
    }));
}

function formatRefreshError(error: unknown): string {
  if (error instanceof Error && error.message.trim()) return error.message;
  if (typeof error === "string" && error.trim()) return error;
  return "读取 OAuth 配额失败";
}

function formatResetError(error: unknown): string {
  const message = formatRefreshError(error);
  if (message === "读取 OAuth 配额失败") return "重置失败";
  return `重置失败：${message}`;
}

function removeProviderErrors(
  current: Record<number, string>,
  providerIds: number[]
): Record<number, string> {
  if (providerIds.length === 0) return current;
  const next = { ...current };
  providerIds.forEach((providerId) => {
    delete next[providerId];
  });
  return next;
}

function readProviderIdsFromRequestLog(log: RequestLogSummary): number[] {
  const providerIds = new Set<number>();

  for (const hop of log.route) {
    if (!Number.isSafeInteger(hop.provider_id) || hop.provider_id <= 0) continue;
    if (hop.skipped) continue;
    providerIds.add(hop.provider_id);
  }

  if (Number.isSafeInteger(log.final_provider_id) && log.final_provider_id > 0) {
    providerIds.add(log.final_provider_id);
  }

  if (
    providerIds.size === 0 &&
    Number.isSafeInteger(log.start_provider_id) &&
    log.start_provider_id > 0
  ) {
    providerIds.add(log.start_provider_id);
  }

  return Array.from(providerIds);
}

function readRequestLogTimestampMs(log: RequestLogSummary): number {
  if (log.created_at_ms != null && Number.isFinite(log.created_at_ms)) {
    return log.created_at_ms;
  }
  return log.created_at * 1000;
}

export function useHomeOAuthQuota({
  cliPriorityOrder,
  requestLogs = [],
  enabled = true,
}: UseHomeOAuthQuotaOptions): UseHomeOAuthQuotaResult {
  const queryClient = useQueryClient();
  const claudeProvidersQuery = useProvidersListQuery("claude", { enabled });
  const codexProvidersQuery = useProvidersListQuery("codex", { enabled });
  const geminiProvidersQuery = useProvidersListQuery("gemini", { enabled });
  const [refreshingProviderIds, setRefreshingProviderIds] = useState<Set<number>>(new Set());
  const [resettingProviderIds, setResettingProviderIds] = useState<Set<number>>(new Set());
  const [providerErrors, setProviderErrors] = useState<Record<number, string>>({});
  const [resetErrors, setResetErrors] = useState<Record<number, string>>({});
  const [oauthQuotaHasRefreshed, setOauthQuotaHasRefreshed] = useState(false);
  const recentUsedAtByProvider = useMemo(() => {
    const timestamps = new Map<number, number>();

    for (const log of requestLogs) {
      const timestampMs = readRequestLogTimestampMs(log);
      for (const providerId of readProviderIdsFromRequestLog(log)) {
        const previous = timestamps.get(providerId) ?? 0;
        if (timestampMs > previous) {
          timestamps.set(providerId, timestampMs);
        }
      }
    }

    return timestamps;
  }, [requestLogs]);

  const oauthProviders = useMemo(() => {
    const providersByCli: Record<CliKey, OAuthProviderSummary[]> = {
      claude: readOAuthProviders(claudeProvidersQuery.data),
      codex: readOAuthProviders(codexProvidersQuery.data),
      gemini: readOAuthProviders(geminiProvidersQuery.data),
    };

    const orderedProviders = cliPriorityOrder.flatMap((cliKey) => providersByCli[cliKey] ?? []);

    return orderedProviders
      .map((provider, index) => ({
        provider,
        index,
        lastUsedAt: recentUsedAtByProvider.get(provider.providerId) ?? null,
      }))
      .sort((left, right) => {
        if (
          left.lastUsedAt != null &&
          right.lastUsedAt != null &&
          left.lastUsedAt !== right.lastUsedAt
        ) {
          return right.lastUsedAt - left.lastUsedAt;
        }
        if (left.lastUsedAt != null) return -1;
        if (right.lastUsedAt != null) return 1;
        return left.index - right.index;
      })
      .map((item) => item.provider);
  }, [
    claudeProvidersQuery.data,
    cliPriorityOrder,
    codexProvidersQuery.data,
    geminiProvidersQuery.data,
    recentUsedAtByProvider,
  ]);

  const oauthQuotaRows = useMemo<HomeOAuthQuotaRow[]>(() => {
    return oauthProviders.map((provider) => {
      const error = providerErrors[provider.providerId] ?? null;
      const resetError = resetErrors[provider.providerId] ?? null;
      const resetting = resettingProviderIds.has(provider.providerId);
      const limits = readProviderOAuthLimitsCache(queryClient, provider.providerId);

      if (error) {
        return {
          ...provider,
          state: "error",
          limits,
          error,
          resetting,
          resetError,
        };
      }

      if (refreshingProviderIds.has(provider.providerId)) {
        return {
          ...provider,
          state: "loading",
          limits,
          error: null,
          resetting,
          resetError,
        };
      }

      if (limits) {
        return {
          ...provider,
          state: "success",
          limits,
          error: null,
          resetting,
          resetError,
        };
      }

      return {
        ...provider,
        state: "idle",
        limits: null,
        error: null,
        resetting,
        resetError,
      };
    });
  }, [
    oauthProviders,
    providerErrors,
    queryClient,
    refreshingProviderIds,
    resetErrors,
    resettingProviderIds,
  ]);

  const refreshOAuthProviders = useCallback(
    async (providers: OAuthProviderSummary[]) => {
      if (!providers.length) return;

      const providerIds = providers.map((provider) => provider.providerId);
      setRefreshingProviderIds((current) => {
        const next = new Set(current);
        providerIds.forEach((providerId) => next.add(providerId));
        return next;
      });
      setProviderErrors((current) => removeProviderErrors(current, providerIds));
      setResetErrors((current) => removeProviderErrors(current, providerIds));
      setOauthQuotaHasRefreshed(true);

      const settled = await Promise.allSettled(
        providers.map(async (provider) => {
          await refreshProviderOAuthLimits(queryClient, provider.providerId, {
            resetCircuitAfterRefresh: true,
          });
          return provider.providerId;
        })
      );

      const nextErrors: Record<number, string> = {};

      settled.forEach((result, index) => {
        if (result.status === "fulfilled") return;
        const providerId = providers[index]?.providerId;
        if (providerId == null) return;
        nextErrors[providerId] = formatRefreshError(result.reason);
      });

      setProviderErrors((current) => ({
        ...removeProviderErrors(current, providerIds),
        ...nextErrors,
      }));
      setRefreshingProviderIds((current) => {
        const next = new Set(current);
        providerIds.forEach((providerId) => next.delete(providerId));
        return next;
      });
    },
    [queryClient]
  );

  const refreshOAuthQuota = useCallback(async () => {
    const enabledProviders = oauthProviders.filter((provider) => provider.enabled);
    if (!enabledProviders.length) return;
    await refreshOAuthProviders(enabledProviders);
  }, [oauthProviders, refreshOAuthProviders]);

  const refreshOAuthQuotaRow = useCallback(
    async (providerId: number) => {
      const target = oauthProviders.find((provider) => provider.providerId === providerId);
      if (!target) return;
      await refreshOAuthProviders([target]);
    },
    [oauthProviders, refreshOAuthProviders]
  );

  const resetOAuthQuotaRow = useCallback(
    async (providerId: number) => {
      const target = oauthProviders.find((provider) => provider.providerId === providerId);
      if (!target || target.cliKey !== "codex") return;

      setResettingProviderIds((current) => {
        const next = new Set(current);
        next.add(target.providerId);
        return next;
      });
      setResetErrors((current) => removeProviderErrors(current, [target.providerId]));
      setOauthQuotaHasRefreshed(true);

      try {
        const result = await resetProviderOAuthCodexQuota(queryClient, target.providerId, {
          resetCircuitAfterRefresh: true,
        });
        if (result.refresh_error) {
          setResetErrors((current) => ({
            ...current,
            [target.providerId]: `已重置，但刷新用量失败：${result.refresh_error}`,
          }));
        }
      } catch (error) {
        setResetErrors((current) => ({
          ...current,
          [target.providerId]: formatResetError(error),
        }));
      } finally {
        setResettingProviderIds((current) => {
          const next = new Set(current);
          next.delete(target.providerId);
          return next;
        });
      }
    },
    [oauthProviders, queryClient]
  );

  return {
    oauthQuotaRows,
    oauthQuotaVisible: oauthProviders.length > 0,
    oauthQuotaRefreshing: refreshingProviderIds.size > 0,
    oauthQuotaHasRefreshed,
    refreshOAuthQuota,
    refreshOAuthQuotaRow,
    resetOAuthQuotaRow,
  };
}
