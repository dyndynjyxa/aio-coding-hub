import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { QueryClient } from "@tanstack/react-query";
import {
  providerClaudeTerminalLaunchCommand,
  providerUpsert,
  providerDuplicate,
  providerDelete,
  providerOAuthStatus,
  providerOAuthFetchLimits,
  providerSetEnabled,
  providersList,
  providersReorder,
  providerTestAvailability,
  type CliKey,
  type OAuthLimitsResult,
  type ProviderAvailabilityResult,
  type ProviderUpsertInput,
  type ProviderSummary,
  validateProviderCliKey,
  validateProviderId,
} from "../services/providers/providers";
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
  providerId: number
): Promise<OAuthLimitsResult> {
  const normalizedProviderId = validateProviderId(providerId);
  const next = normalizeProviderOAuthLimitsResult(
    await providerOAuthFetchLimits(normalizedProviderId)
  );
  queryClient.setQueryData(oauthLimitsKeys.detail(normalizedProviderId), next);
  return next;
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
    mutationFn: (input: { cliKey: CliKey; providerId: number }) => providerDelete(input.providerId),
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

export function useProviderDuplicateMutation() {
  const queryClient = useQueryClient();

  return useMutation<ProviderSummary | null, Error, { providerId: number }>({
    mutationFn: (input: { providerId: number }) => providerDuplicate(input.providerId),
    onSuccess: (duplicated) => {
      if (!duplicated) return;
      queryClient.setQueryData<ProviderSummary[] | null>(
        providersKeys.list(duplicated.cli_key),
        (prev) => {
          if (!prev) return [duplicated];
          if (prev.some((row) => row.id === duplicated.id)) return prev;
          return [...prev, duplicated];
        }
      );
      queryClient.invalidateQueries({ queryKey: providersKeys.list(duplicated.cli_key) });
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
