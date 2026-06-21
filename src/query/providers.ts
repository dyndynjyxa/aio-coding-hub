import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { QueryClient } from "@tanstack/react-query";
import {
  defaultRouteProvidersList,
  defaultRouteProvidersSetOrder,
  providerClaudeTerminalLaunchCommand,
  providerUpsert,
  providerDuplicate,
  providerDelete,
  providerOAuthStatus,
  providerOAuthFetchLimits,
  providerOAuthResetCodexQuota,
  providerSetEnabled,
  providersList,
  providersReorder,
  providerTestAvailability,
  type CliKey,
  type OAuthLimitsResult,
  type ProviderOAuthResetCodexQuotaResult,
  type ProviderAvailabilityResult,
  type ProviderRouteRow,
  type ProviderUpsertInput,
  type ProviderSummary,
  validateProviderCliKey,
  validateProviderId,
} from "../services/providers/providers";
import { logToConsole } from "../services/consoleLog";
import { gatewayCircuitResetProvider } from "../services/gateway/gateway";
import { formatUnknownError } from "../utils/errors";
import { gatewayKeys, oauthLimitsKeys, providersKeys } from "./keys";

export function useProvidersListQuery(cliKey: CliKey, options?: { enabled?: boolean }) {
  const normalizedCliKey = validateProviderCliKey(cliKey);

  return useQuery({
    queryKey: providersKeys.list(normalizedCliKey),
    queryFn: () => providersList(normalizedCliKey),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function useDefaultRouteProvidersQuery(cliKey: CliKey, options?: { enabled?: boolean }) {
  const normalizedCliKey = validateProviderCliKey(cliKey);

  return useQuery({
    queryKey: providersKeys.defaultRoute(normalizedCliKey),
    queryFn: () => defaultRouteProvidersList(normalizedCliKey),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
    retry: false,
  });
}

export function useProviderOAuthStatusQuery(
  providerId: number | null,
  options?: { enabled?: boolean }
) {
  const normalizedProviderId = providerId == null ? null : validateProviderId(providerId);

  return useQuery({
    queryKey: providersKeys.oauthStatus(normalizedProviderId),
    queryFn: () => {
      if (normalizedProviderId == null) return null;
      return providerOAuthStatus(normalizedProviderId);
    },
    enabled: (options?.enabled ?? true) && normalizedProviderId != null,
    placeholderData: keepPreviousData,
  });
}

export async function fetchProviderOAuthStatus(
  queryClient: ReturnType<typeof useQueryClient>,
  providerId: number | null
) {
  if (providerId == null) return null;
  const normalizedProviderId = validateProviderId(providerId);
  return queryClient.fetchQuery({
    queryKey: providersKeys.oauthStatus(normalizedProviderId),
    queryFn: () => providerOAuthStatus(normalizedProviderId),
  });
}

const EMPTY_OAUTH_LIMITS_RESULT: OAuthLimitsResult = {
  limit_short_label: null,
  limit_5h_text: null,
  limit_weekly_text: null,
  limit_5h_reset_at: null,
  limit_weekly_reset_at: null,
  reset_credit_available_count: null,
};

export function normalizeProviderOAuthLimitsResult(
  result: OAuthLimitsResult | null | undefined
): OAuthLimitsResult {
  if (!result) return EMPTY_OAUTH_LIMITS_RESULT;
  return result;
}

export function readProviderOAuthLimitsCache(
  queryClient: QueryClient,
  providerId: number
): OAuthLimitsResult | null {
  const normalizedProviderId = validateProviderId(providerId);
  const state = queryClient.getQueryState<OAuthLimitsResult>(
    oauthLimitsKeys.detail(normalizedProviderId)
  );
  return state?.data ?? null;
}

export async function refreshProviderOAuthLimits(
  queryClient: QueryClient,
  providerId: number,
  options?: { resetCircuitAfterRefresh?: boolean }
): Promise<OAuthLimitsResult> {
  const normalizedProviderId = validateProviderId(providerId);
  const next = normalizeProviderOAuthLimitsResult(
    await providerOAuthFetchLimits(normalizedProviderId)
  );
  queryClient.setQueryData(oauthLimitsKeys.detail(normalizedProviderId), next);
  try {
    if (options?.resetCircuitAfterRefresh) {
      try {
        await gatewayCircuitResetProvider(normalizedProviderId);
      } catch (error) {
        logToConsole("warn", "OAuth 配额刷新成功，但重置熔断器失败", {
          provider_id: normalizedProviderId,
          error: formatUnknownError(error),
        });
      }
    }
  } finally {
    void queryClient.invalidateQueries({ queryKey: gatewayKeys.circuits() });
  }
  return next;
}

export async function resetProviderOAuthCodexQuota(
  queryClient: QueryClient,
  providerId: number,
  options?: { resetCircuitAfterRefresh?: boolean }
): Promise<ProviderOAuthResetCodexQuotaResult> {
  const normalizedProviderId = validateProviderId(providerId);
  const result = await providerOAuthResetCodexQuota(normalizedProviderId);
  const refreshedLimits = result.refreshed_limits;

  if (refreshedLimits) {
    const next = normalizeProviderOAuthLimitsResult(refreshedLimits);
    queryClient.setQueryData(oauthLimitsKeys.detail(normalizedProviderId), next);
    try {
      if (options?.resetCircuitAfterRefresh) {
        try {
          await gatewayCircuitResetProvider(normalizedProviderId);
        } catch (error) {
          logToConsole("warn", "Codex 重置成功，但重置熔断器失败", {
            provider_id: normalizedProviderId,
            error: formatUnknownError(error),
          });
        }
      }
    } finally {
      void queryClient.invalidateQueries({ queryKey: gatewayKeys.circuits() });
    }
  }

  return result;
}

export function useProviderSetEnabledMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { providerId: number; enabled: boolean }) =>
      providerSetEnabled(input.providerId, input.enabled),
    onSuccess: (updated) => {
      if (!updated) return;

      queryClient.setQueryData<ProviderSummary[] | null>(
        providersKeys.list(updated.cli_key),
        (prev) => {
          if (!prev) return prev;
          return prev.map((row) => (row.id === updated.id ? updated : row));
        }
      );
    },
  });
}

export function useProviderUpsertMutation() {
  const queryClient = useQueryClient();

  return useMutation<ProviderSummary, Error, { input: ProviderUpsertInput }>({
    mutationFn: (input: { input: ProviderUpsertInput }) => providerUpsert(input.input),
    onSuccess: (saved) => {
      queryClient.setQueryData<ProviderSummary[] | null>(
        providersKeys.list(saved.cli_key),
        (prev) => {
          if (!prev) {
            return [saved];
          }

          const existingIndex = prev.findIndex((row) => row.id === saved.id);
          if (existingIndex === -1) {
            return [...prev, saved];
          }

          return prev.map((row) => (row.id === saved.id ? saved : row));
        }
      );

      void queryClient.invalidateQueries({ queryKey: providersKeys.list(saved.cli_key) });
      void queryClient.invalidateQueries({ queryKey: gatewayKeys.circuitStatus(saved.cli_key) });
    },
  });
}

export function useProviderDeleteMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { cliKey: CliKey; providerId: number; clearUsageStats?: boolean }) =>
      providerDelete(input.providerId, { clearUsageStats: input.clearUsageStats === true }),
    onSuccess: (ok, input) => {
      if (!ok) return;
      const cliKey = validateProviderCliKey(input.cliKey);

      queryClient.setQueryData<ProviderSummary[] | null>(providersKeys.list(cliKey), (prev) => {
        if (!prev) return prev;
        return prev.filter((row) => row.id !== input.providerId);
      });
    },
  });
}

export function useProvidersReorderMutation() {
  const queryClient = useQueryClient();

  return useMutation<
    ProviderSummary[] | null,
    Error,
    {
      cliKey: CliKey;
      orderedProviderIds: number[];
      optimisticProviders?: ProviderSummary[];
    },
    { previousProviders: ProviderSummary[] | null | undefined }
  >({
    mutationFn: (input: {
      cliKey: CliKey;
      orderedProviderIds: number[];
      optimisticProviders?: ProviderSummary[];
    }) => providersReorder(validateProviderCliKey(input.cliKey), input.orderedProviderIds),
    onMutate: async (input) => {
      const cliKey = validateProviderCliKey(input.cliKey);
      await queryClient.cancelQueries({ queryKey: providersKeys.list(cliKey) });
      const previousProviders = queryClient.getQueryData<ProviderSummary[] | null>(
        providersKeys.list(cliKey)
      );
      if (input.optimisticProviders) {
        queryClient.setQueryData(providersKeys.list(cliKey), input.optimisticProviders);
      }
      return { previousProviders };
    },
    onError: (_error, input, context) => {
      if (context?.previousProviders !== undefined) {
        const cliKey = validateProviderCliKey(input.cliKey);
        queryClient.setQueryData(providersKeys.list(cliKey), context.previousProviders);
      }
    },
    onSuccess: (next, input) => {
      if (!next) return;
      const cliKey = validateProviderCliKey(input.cliKey);
      queryClient.setQueryData(providersKeys.list(cliKey), next);
    },
  });
}

export function useDefaultRouteProvidersSetOrderMutation() {
  const queryClient = useQueryClient();

  return useMutation<
    ProviderRouteRow[] | null,
    Error,
    {
      cliKey: CliKey;
      orderedProviderIds: number[];
      optimisticRows?: ProviderRouteRow[];
    },
    { previousRows: ProviderRouteRow[] | null | undefined }
  >({
    mutationFn: (input) =>
      defaultRouteProvidersSetOrder(validateProviderCliKey(input.cliKey), input.orderedProviderIds),
    onMutate: async (input) => {
      const cliKey = validateProviderCliKey(input.cliKey);
      await queryClient.cancelQueries({ queryKey: providersKeys.defaultRoute(cliKey) });
      const previousRows = queryClient.getQueryData<ProviderRouteRow[] | null>(
        providersKeys.defaultRoute(cliKey)
      );
      if (input.optimisticRows) {
        queryClient.setQueryData(providersKeys.defaultRoute(cliKey), input.optimisticRows);
      }
      return { previousRows };
    },
    onError: (_error, input, context) => {
      if (context?.previousRows !== undefined) {
        const cliKey = validateProviderCliKey(input.cliKey);
        queryClient.setQueryData(providersKeys.defaultRoute(cliKey), context.previousRows);
      }
    },
    onSuccess: (next, input) => {
      if (!next) return;
      const cliKey = validateProviderCliKey(input.cliKey);
      queryClient.setQueryData(providersKeys.defaultRoute(cliKey), next);
    },
    onSettled: (_data, _error, input) => {
      const cliKey = validateProviderCliKey(input.cliKey);
      void queryClient.invalidateQueries({ queryKey: providersKeys.defaultRoute(cliKey) });
    },
  });
}

export function useProviderDuplicateMutation() {
  const queryClient = useQueryClient();

  return useMutation<
    ProviderSummary | null,
    Error,
    { providerId: number },
    { sourceProviderId: number }
  >({
    mutationFn: (input: { providerId: number }) => providerDuplicate(input.providerId),
    onMutate: (input) => {
      return { sourceProviderId: input.providerId };
    },
    onSuccess: async (duplicated, _input, context) => {
      if (!duplicated) return;

      const cliKey = duplicated.cli_key;
      const sourceId = context?.sourceProviderId;

      // Insert the duplicated provider right after the source in the cache
      queryClient.setQueryData<ProviderSummary[] | null>(providersKeys.list(cliKey), (prev) => {
        if (!prev) return [duplicated];

        const rows = prev.filter((row) => row.id !== duplicated.id);

        if (sourceId != null) {
          const sourceIndex = rows.findIndex((row) => row.id === sourceId);
          if (sourceIndex !== -1) {
            const next = [...rows];
            next.splice(sourceIndex + 1, 0, duplicated);
            return next;
          }
        }

        return [...rows, duplicated];
      });

      // Persist the new order to the backend
      const currentList = queryClient.getQueryData<ProviderSummary[] | null>(
        providersKeys.list(cliKey)
      );
      if (currentList && currentList.length > 1) {
        try {
          const reordered = await providersReorder(
            cliKey as CliKey,
            currentList.map((p) => p.id)
          );
          if (reordered) {
            queryClient.setQueryData(providersKeys.list(cliKey), reordered);
          }
        } catch (error) {
          await queryClient.invalidateQueries({ queryKey: providersKeys.list(cliKey) });
          throw error;
        }
      } else {
        await queryClient.invalidateQueries({ queryKey: providersKeys.list(cliKey) });
      }
    },
  });
}

export function useProviderClaudeTerminalLaunchCommandMutation() {
  return useMutation({
    mutationFn: (input: { providerId: number }) =>
      providerClaudeTerminalLaunchCommand(input.providerId),
  });
}

export function useOAuthLimitsQuery(providerId: number, enabled: boolean) {
  const normalizedProviderId = validateProviderId(providerId);

  return useQuery({
    queryKey: oauthLimitsKeys.detail(normalizedProviderId),
    queryFn: async (): Promise<OAuthLimitsResult> => {
      return normalizeProviderOAuthLimitsResult(
        await providerOAuthFetchLimits(normalizedProviderId)
      );
    },
    enabled,
    staleTime: 180_000,
    refetchInterval: 180_000,
  });
}

export function useProviderTestAvailabilityMutation() {
  return useMutation<ProviderAvailabilityResult | null, Error, { providerId: number }>({
    mutationFn: (input) => providerTestAvailability(input.providerId),
  });
}
