import {
  keepPreviousData,
  useInfiniteQuery,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import {
  cliSessionsFolderLookupByIds,
  cliSessionsMessagesGet,
  cliSessionsProjectsList,
  cliSessionsSessionDelete,
  cliSessionsSessionsList,
  type CliSessionsFolderLookupEntry,
  type CliSessionsFolderLookupInput,
  type CliSessionsSessionSummary,
  type CliSessionsSource,
} from "../services/cli/cliSessions";
import { cliSessionsKeys } from "./keys";

export function useCliSessionsProjectsListQuery(source: CliSessionsSource, wslDistro?: string) {
  return useQuery({
    queryKey: cliSessionsKeys.projectsList(source, wslDistro),
    queryFn: () => cliSessionsProjectsList(source, wslDistro),
    enabled: true,
    placeholderData: keepPreviousData,
  });
}

export function useCliSessionsSessionsListQuery(
  source: CliSessionsSource,
  projectId: string,
  options?: { enabled?: boolean; wslDistro?: string }
) {
  const wslDistro = options?.wslDistro;
  return useQuery({
    queryKey: cliSessionsKeys.sessionsList(source, projectId, wslDistro),
    queryFn: () => cliSessionsSessionsList(source, projectId, wslDistro),
    enabled: Boolean(projectId.trim()) && (options?.enabled ?? true),
    placeholderData: keepPreviousData,
  });
}

export function useCliSessionsFolderLookupByIdsQuery(
  items: CliSessionsFolderLookupInput[],
  options?: { enabled?: boolean; wslDistro?: string }
) {
  const wslDistro = options?.wslDistro;
  const lookupKeys = items.map((item) => `${item.source}:${item.session_id}`);
  return useQuery<CliSessionsFolderLookupEntry[]>({
    queryKey: cliSessionsKeys.folderLookup(lookupKeys, wslDistro),
    queryFn: async () => (await cliSessionsFolderLookupByIds(items, wslDistro)) ?? [],
    enabled: items.length > 0 && (options?.enabled ?? true),
  });
}

export function useCliSessionsMessagesInfiniteQuery(
  source: CliSessionsSource,
  filePath: string,
  options?: { enabled?: boolean; fromEnd?: boolean; wslDistro?: string }
) {
  const fromEnd = options?.fromEnd ?? true;
  const wslDistro = options?.wslDistro;
  return useInfiniteQuery({
    queryKey: cliSessionsKeys.messages(source, filePath, fromEnd, wslDistro),
    queryFn: ({ pageParam = 0 }) =>
      cliSessionsMessagesGet({
        source,
        file_path: filePath,
        page: pageParam,
        page_size: 50,
        from_end: fromEnd,
        wsl_distro: wslDistro,
      }),
    enabled: Boolean(filePath.trim()) && (options?.enabled ?? true),
    getNextPageParam: (lastPage) => (lastPage?.has_more ? lastPage.page + 1 : undefined),
    initialPageParam: 0,
  });
}

export function useCliSessionsSessionDeleteMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: {
      source: CliSessionsSource;
      filePaths: string[];
      projectId: string;
      wslDistro?: string;
    }) =>
      cliSessionsSessionDelete({
        source: input.source,
        file_paths: input.filePaths,
        wsl_distro: input.wslDistro,
      }),
    onSuccess: (failedList, input) => {
      if (!failedList) return;
      const deletedPaths = new Set(
        input.filePaths.filter((fp) => !failedList.some((f) => f.startsWith(fp)))
      );
      if (deletedPaths.size === 0) return;
      const key = cliSessionsKeys.sessionsList(input.source, input.projectId, input.wslDistro);
      queryClient.setQueryData<CliSessionsSessionSummary[] | null>(key, (prev) => {
        if (!prev) return prev;
        return prev.filter((s) => !deletedPaths.has(s.file_path));
      });
    },
    onSettled: (_res, _err, input) => {
      if (!input) return;
      queryClient.invalidateQueries({
        queryKey: cliSessionsKeys.sessionsList(input.source, input.projectId, input.wslDistro),
      });
      queryClient.invalidateQueries({
        queryKey: cliSessionsKeys.projectsList(input.source, input.wslDistro),
      });
    },
  });
}
