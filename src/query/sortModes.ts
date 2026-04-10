import {
  keepPreviousData,
  useMutation,
  useQuery,
  useQueryClient,
  type QueryClient,
} from "@tanstack/react-query";
import type { CliKey } from "../services/providers/providers";
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
} from "../services/providers/sortModes";
import { sortModesKeys } from "./keys";

function invalidateSortModesQueries(
  queryClient: QueryClient,
  options: { includeActiveList?: boolean } = {}
) {
  void queryClient.invalidateQueries({ queryKey: sortModesKeys.list() });
  if (options.includeActiveList) {
    void queryClient.invalidateQueries({ queryKey: sortModesKeys.activeList() });
  }
}

export function sortModeProvidersQueryKey(modeId: number, cliKey: CliKey) {
  return [...sortModesKeys.all, "providers", cliKey, modeId] as const;
}

function invalidateSortModeProvidersQuery(
  queryClient: QueryClient,
  input: { modeId: number; cliKey: CliKey }
) {
  void queryClient.invalidateQueries({
    queryKey: sortModeProvidersQueryKey(input.modeId, input.cliKey),
  });
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
  return useQuery({
    queryKey:
      input.modeId == null
        ? [...sortModesKeys.all, "providers", input.cliKey, null]
        : sortModeProvidersQueryKey(input.modeId, input.cliKey),
    queryFn: () => {
      if (input.modeId == null) {
        return Promise.resolve<SortModeProviderRow[] | null>(null);
      }
      return sortModeProvidersList({ mode_id: input.modeId, cli_key: input.cliKey });
    },
    enabled: input.modeId != null && (options.enabled ?? true),
    retry: false,
  });
}

export function useSortModeActiveSetMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { cliKey: CliKey; modeId: number | null }) =>
      sortModeActiveSet({ cli_key: input.cliKey, mode_id: input.modeId }),
    onMutate: (input) => {
      void queryClient.cancelQueries({ queryKey: sortModesKeys.activeList() });

      const previous =
        queryClient.getQueryData<SortModeActiveRow[] | null>(sortModesKeys.activeList()) ?? null;

      if (previous) {
        const next = previous.map((row) =>
          row.cli_key === input.cliKey ? { ...row, mode_id: input.modeId } : row
        );
        queryClient.setQueryData(sortModesKeys.activeList(), next);
      }

      return { previous };
    },
    onSuccess: (res, _input, ctx) => {
      if (!res) {
        if (ctx?.previous) {
          queryClient.setQueryData(sortModesKeys.activeList(), ctx.previous);
        }
        return;
      }

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
      invalidateSortModesQueries(queryClient);
    },
  });
}

export function useSortModeRenameMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { modeId: number; name: string }) =>
      sortModeRename({ mode_id: input.modeId, name: input.name }),
    onSettled: () => {
      invalidateSortModesQueries(queryClient);
    },
  });
}

export function useSortModeDeleteMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { modeId: number }) => sortModeDelete({ mode_id: input.modeId }),
    onSettled: () => {
      invalidateSortModesQueries(queryClient, { includeActiveList: true });
    },
  });
}

export function useSortModeProvidersSetOrderMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { modeId: number; cliKey: CliKey; orderedProviderIds: number[] }) =>
      sortModeProvidersSetOrder({
        mode_id: input.modeId,
        cli_key: input.cliKey,
        ordered_provider_ids: input.orderedProviderIds,
      }),
    onSettled: (_data, _error, input) => {
      invalidateSortModeProvidersQuery(queryClient, {
        modeId: input.modeId,
        cliKey: input.cliKey,
      });
    },
  });
}

export function useSortModeProviderSetEnabledMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { modeId: number; cliKey: CliKey; providerId: number; enabled: boolean }) =>
      sortModeProviderSetEnabled({
        mode_id: input.modeId,
        cli_key: input.cliKey,
        provider_id: input.providerId,
        enabled: input.enabled,
      }),
    onSettled: (_data, _error, input) => {
      invalidateSortModeProvidersQuery(queryClient, {
        modeId: input.modeId,
        cliKey: input.cliKey,
      });
    },
  });
}
