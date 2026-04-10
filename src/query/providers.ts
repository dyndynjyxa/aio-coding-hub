import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  providerClaudeTerminalLaunchCommand,
  providerDelete,
  providerSetEnabled,
  providersList,
  providersReorder,
  type CliKey,
  type ProviderSummary,
} from "../services/providers/providers";
import { providersKeys } from "./keys";

export function useProvidersListQuery(cliKey: CliKey, options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: providersKeys.list(cliKey),
    queryFn: () => providersList(cliKey),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
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

  return useMutation({
    mutationFn: (input: { cliKey: CliKey; orderedProviderIds: number[] }) =>
      providersReorder(input.cliKey, input.orderedProviderIds),
    onSuccess: (next, input) => {
      if (!next) return;
      queryClient.setQueryData(providersKeys.list(input.cliKey), next);
    },
  });
}

export function useProviderClaudeTerminalLaunchCommandMutation() {
  return useMutation({
    mutationFn: (input: { providerId: number }) =>
      providerClaudeTerminalLaunchCommand(input.providerId),
  });
}
