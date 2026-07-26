import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { validateProviderCliKey, type CliKey } from "../services/providers/providers";
import { FeValidationError } from "../utils/errors";
import {
  sortModeActiveList,
  sortModeActiveSet,
  sortModeCreate,
  sortModeDelete,
  sortModeProviderSetEnabled,
  sortModeProvidersList,
  sortModeProvidersSetOrder,
  sortModeRename,
  sortModesList,
  type SortModeActiveRow,
  type SortModeProviderRow,
  validateSortModeId,
} from "../services/providers/sortModes";
import { sortModesKeys } from "./keys";

export function sortModeProvidersQueryKey(modeId: number, cliKey: CliKey) {
  return [
    ...sortModesKeys.all,
    "providers",
    validateProviderCliKey(cliKey),
    validateSortModeId(modeId),
  ] as const;
}

export function useSortModesListQuery(options: { enabled?: boolean } = {}) {
  return useQuery({
    queryKey: sortModesKeys.list(),
    queryFn: () => sortModesList(),
    enabled: options.enabled ?? true,
    placeholderData: keepPreviousData,
    retry: false,
  });
}

export function useSortModeActiveListQuery(options: { enabled?: boolean } = {}) {
  return useQuery({
    queryKey: sortModesKeys.activeList(),
    queryFn: () => sortModeActiveList(),
    enabled: options.enabled ?? true,
    placeholderData: keepPreviousData,
    retry: false,
  });
}

export function useSortModeProvidersListQuery(
  input: { modeId: number | null; cliKey: CliKey },
  options: { enabled?: boolean } = {}
) {
  const cliKey = validateProviderCliKey(input.cliKey);
  const modeId = input.modeId == null ? null : validateSortModeId(input.modeId);

  return useQuery({
    queryKey:
      modeId == null
        ? [...sortModesKeys.all, "providers", cliKey, null]
        : sortModeProvidersQueryKey(modeId, cliKey),
    queryFn: () => {
      if (modeId == null) {
        return Promise.resolve<SortModeProviderRow[] | null>(null);
      }
      return sortModeProvidersList({ mode_id: modeId, cli_key: cliKey });
    },
    enabled: modeId != null && (options.enabled ?? true),
    retry: false,
  });
}

export function useSortModeActiveSetMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { cliKey: CliKey; modeId: number | null }) => {
      const modeId = input.modeId == null ? null : validateSortModeId(input.modeId);
      return sortModeActiveSet({ cli_key: validateProviderCliKey(input.cliKey), mode_id: modeId });
    },
    onMutate: (input) => {
      const cliKey = validateProviderCliKey(input.cliKey);
      const modeId = input.modeId == null ? null : validateSortModeId(input.modeId);
      void queryClient.cancelQueries({ queryKey: sortModesKeys.activeList() });

      const previous =
        queryClient.getQueryData<SortModeActiveRow[] | null>(sortModesKeys.activeList()) ?? null;

      if (previous) {
        const next = previous.map((row) =>
          row.cli_key === cliKey ? { ...row, mode_id: modeId } : row
        );
        queryClient.setQueryData(sortModesKeys.activeList(), next);
      }

      return { previous };
    },
    onSuccess: (res) => {
      queryClient.setQueryData<SortModeActiveRow[] | null>(sortModesKeys.activeList(), (prev) => {
        if (!prev) return prev;
        return prev.map((row) => (row.cli_key === res.cli_key ? res : row));
      });
    },
    onError: (_err, _input, ctx) => {
      if (ctx?.previous) {
        queryClient.setQueryData(sortModesKeys.activeList(), ctx.previous);
      }
    },
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey: sortModesKeys.activeList() });
    },
  });
}

export function useSortModeCreateMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { name: string }) => sortModeCreate({ name: input.name }),
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey: sortModesKeys.list() });
    },
  });
}

export function useSortModeRenameMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { modeId: number; name: string }) =>
      sortModeRename({ mode_id: validateSortModeId(input.modeId), name: input.name }),
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey: sortModesKeys.list() });
    },
  });
}

export function useSortModeDeleteMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { modeId: number }) =>
      sortModeDelete({ mode_id: validateSortModeId(input.modeId) }),
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey: sortModesKeys.list() });
      void queryClient.invalidateQueries({ queryKey: sortModesKeys.activeList() });
    },
  });
}

export function useSortModeProvidersSetOrderMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { modeId: number; cliKey: CliKey; orderedProviderIds: number[] }) =>
      sortModeProvidersSetOrder({
        mode_id: validateSortModeId(input.modeId),
        cli_key: validateProviderCliKey(input.cliKey),
        ordered_provider_ids: input.orderedProviderIds,
      }),
    onSettled: (_data, _error, input) => {
      try {
        const cliKey = validateProviderCliKey(input.cliKey);
        const modeId = validateSortModeId(input.modeId);
        void queryClient.invalidateQueries({
          queryKey: sortModeProvidersQueryKey(modeId, cliKey),
        });
      } catch (error) {
        if (error instanceof FeValidationError) return;
        throw error;
      }
    },
  });
}

export function useSortModeProviderSetEnabledMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { modeId: number; cliKey: CliKey; providerId: number; enabled: boolean }) =>
      sortModeProviderSetEnabled({
        mode_id: validateSortModeId(input.modeId),
        cli_key: validateProviderCliKey(input.cliKey),
        provider_id: input.providerId,
        enabled: input.enabled,
      }),
    onSettled: (_data, _error, input) => {
      try {
        const cliKey = validateProviderCliKey(input.cliKey);
        const modeId = validateSortModeId(input.modeId);
        void queryClient.invalidateQueries({
          queryKey: sortModeProvidersQueryKey(modeId, cliKey),
        });
      } catch (error) {
        if (error instanceof FeValidationError) return;
        throw error;
      }
    },
  });
}
