import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
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
  type CliKey,
  type OAuthLimitsResult,
  type ProviderUpsertInput,
  type ProviderSummary,
} from "../services/providers/providers";
import { gatewayKeys, oauthLimitsKeys, providersKeys } from "./keys";

export function useProvidersListQuery(cliKey: CliKey, options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: providersKeys.list(cliKey),
    queryFn: () => providersList(cliKey),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function useProviderOAuthStatusQuery(
  providerId: number | null,
  options?: { enabled?: boolean }
) {
  return useQuery({
    queryKey: providersKeys.oauthStatus(providerId),
    queryFn: () => {
      if (providerId == null) return null;
      return providerOAuthStatus(providerId);
    },
    enabled: (options?.enabled ?? true) && providerId != null,
    placeholderData: keepPreviousData,
  });
}

export async function fetchProviderOAuthStatus(
  queryClient: ReturnType<typeof useQueryClient>,
  providerId: number | null
) {
  if (providerId == null) return null;
  return queryClient.fetchQuery({
    queryKey: providersKeys.oauthStatus(providerId),
    queryFn: () => providerOAuthStatus(providerId),
  });
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

  return useMutation<ProviderSummary | null, Error, { input: ProviderUpsertInput }>({
    mutationFn: (input: { input: ProviderUpsertInput }) => providerUpsert(input.input),
    onSuccess: (saved) => {
      if (!saved) return;

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

      queryClient.setQueryData<ProviderSummary[] | null>(
        providersKeys.list(input.cliKey),
        (prev) => {
          if (!prev) return prev;
          return prev.filter((row) => row.id !== input.providerId);
        }
      );
    },
  });
}

export function useProvidersReorderMutation() {
  const queryClient = useQueryClient();

  return useMutation<ProviderSummary[] | null, Error, {
    cliKey: CliKey;
    orderedProviderIds: number[];
    optimisticProviders?: ProviderSummary[];
  }, { previousProviders: ProviderSummary[] | null | undefined }>({
    mutationFn: (input: {
      cliKey: CliKey;
      orderedProviderIds: number[];
      optimisticProviders?: ProviderSummary[];
    }) =>
      providersReorder(input.cliKey, input.orderedProviderIds),
    onMutate: async (input) => {
      await queryClient.cancelQueries({ queryKey: providersKeys.list(input.cliKey) });
      const previousProviders = queryClient.getQueryData<ProviderSummary[] | null>(
        providersKeys.list(input.cliKey)
      );
      if (input.optimisticProviders) {
        queryClient.setQueryData(providersKeys.list(input.cliKey), input.optimisticProviders);
      }
      return { previousProviders };
    },
    onError: (_error, input, context) => {
      if (context?.previousProviders !== undefined) {
        queryClient.setQueryData(providersKeys.list(input.cliKey), context.previousProviders);
      }
    },
    onSuccess: (next, input) => {
      if (!next) return;
      queryClient.setQueryData(providersKeys.list(input.cliKey), next);
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
  return useQuery({
    queryKey: oauthLimitsKeys.detail(providerId),
    queryFn: async (): Promise<OAuthLimitsResult> => {
      const result = await providerOAuthFetchLimits(providerId);
      if (!result) {
        return {
          limit_short_label: null,
          limit_5h_text: null,
          limit_weekly_text: null,
          limit_5h_reset_at: null,
          limit_weekly_reset_at: null,
        };
      }
      return result;
    },
    enabled,
    staleTime: 180_000,
    refetchInterval: 180_000,
  });
}
