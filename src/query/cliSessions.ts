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
  normalizeCliSessionsDeleteFilePaths,
  normalizeCliSessionsFilePath,
  normalizeCliSessionsFolderLookupItems,
  normalizeCliSessionsProjectId,
  normalizeCliSessionsWslDistro,
  type CliSessionsFolderLookupEntry,
  type CliSessionsFolderLookupInput,
  type CliSessionsSessionSummary,
  type CliSessionsSource,
} from "../services/cli/cliSessions";
import { cliSessionsKeys } from "./keys";

function normalizeCliSessionsMutationCacheInput(input: { projectId: string; wslDistro?: string }) {
  return {
    projectId: normalizeCliSessionsProjectId(input.projectId),
    wslDistro: normalizeCliSessionsWslDistro(input.wslDistro) ?? undefined,
  };
}

function normalizeOptionalCliSessionsProjectId(projectId: string): string {
  return projectId.trim() ? normalizeCliSessionsProjectId(projectId) : "";
}

function normalizeOptionalCliSessionsFilePath(filePath: string): string {
  return filePath.trim() ? normalizeCliSessionsFilePath(filePath) : "";
}

export function useCliSessionsProjectsListQuery(source: CliSessionsSource, wslDistro?: string) {
  const normalizedWslDistro = normalizeCliSessionsWslDistro(wslDistro) ?? undefined;
  return useQuery({
    queryKey: cliSessionsKeys.projectsList(source, normalizedWslDistro),
    queryFn: () => cliSessionsProjectsList(source, normalizedWslDistro),
    enabled: true,
    placeholderData: keepPreviousData,
  });
}

export function useCliSessionsSessionsListQuery(
  source: CliSessionsSource,
  projectId: string,
  options?: { enabled?: boolean; wslDistro?: string }
) {
  const wslDistro = normalizeCliSessionsWslDistro(options?.wslDistro) ?? undefined;
  const normalizedProjectId = normalizeOptionalCliSessionsProjectId(projectId);
  return useQuery({
    queryKey: cliSessionsKeys.sessionsList(source, normalizedProjectId, wslDistro),
    queryFn: () => cliSessionsSessionsList(source, normalizedProjectId, wslDistro),
    enabled: Boolean(normalizedProjectId) && (options?.enabled ?? true),
    placeholderData: keepPreviousData,
  });
}

export function useCliSessionsFolderLookupByIdsQuery(
  items: CliSessionsFolderLookupInput[],
  options?: { enabled?: boolean; wslDistro?: string }
) {
  const wslDistro = normalizeCliSessionsWslDistro(options?.wslDistro) ?? undefined;
  const normalizedItems = normalizeCliSessionsFolderLookupItems(items);
  const lookupKeys = normalizedItems.map((item) => `${item.source}:${item.session_id}`);
  return useQuery<CliSessionsFolderLookupEntry[]>({
    queryKey: cliSessionsKeys.folderLookup(lookupKeys, wslDistro),
    queryFn: async () => (await cliSessionsFolderLookupByIds(normalizedItems, wslDistro)) ?? [],
    enabled: normalizedItems.length > 0 && (options?.enabled ?? true),
  });
}

export function useCliSessionsMessagesInfiniteQuery(
  source: CliSessionsSource,
  filePath: string,
  options?: { enabled?: boolean; fromEnd?: boolean; wslDistro?: string }
) {
  const fromEnd = options?.fromEnd ?? true;
  const wslDistro = normalizeCliSessionsWslDistro(options?.wslDistro) ?? undefined;
  const normalizedFilePath = normalizeOptionalCliSessionsFilePath(filePath);
  return useInfiniteQuery({
    queryKey: cliSessionsKeys.messages(source, normalizedFilePath, fromEnd, wslDistro),
    queryFn: ({ pageParam = 0 }) =>
      cliSessionsMessagesGet({
        source,
        filePath: normalizedFilePath,
        page: pageParam,
        pageSize: 50,
        fromEnd,
        wslDistro,
      }),
    enabled: Boolean(normalizedFilePath) && (options?.enabled ?? true),
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
    }) => {
      const wslDistro = normalizeCliSessionsWslDistro(input.wslDistro) ?? undefined;
      const filePaths = normalizeCliSessionsDeleteFilePaths(input.filePaths);
      return cliSessionsSessionDelete({
        source: input.source,
        filePaths,
        wslDistro,
      });
    },
    onSuccess: (failedList, input) => {
      if (!failedList) return;
      let normalizedInput: ReturnType<typeof normalizeCliSessionsMutationCacheInput>;
      let filePaths: string[];
      try {
        normalizedInput = normalizeCliSessionsMutationCacheInput(input);
        filePaths = normalizeCliSessionsDeleteFilePaths(input.filePaths);
      } catch {
        return;
      }
      const deletedPaths = new Set(
        filePaths.filter((fp) => !failedList.some((failedPath) => failedPath.startsWith(fp)))
      );
      if (deletedPaths.size === 0) return;
      const key = cliSessionsKeys.sessionsList(
        input.source,
        normalizedInput.projectId,
        normalizedInput.wslDistro
      );
      queryClient.setQueryData<CliSessionsSessionSummary[] | null>(key, (prev) => {
        if (!prev) return prev;
        return prev.filter((s) => !deletedPaths.has(s.file_path));
      });
    },
    onSettled: (_res, _err, input) => {
      if (!input) return;
      let normalizedInput: ReturnType<typeof normalizeCliSessionsMutationCacheInput>;
      try {
        normalizedInput = normalizeCliSessionsMutationCacheInput(input);
      } catch {
        return;
      }
      queryClient.invalidateQueries({
        queryKey: cliSessionsKeys.sessionsList(
          input.source,
          normalizedInput.projectId,
          normalizedInput.wslDistro
        ),
      });
      queryClient.invalidateQueries({
        queryKey: cliSessionsKeys.projectsList(input.source, normalizedInput.wslDistro),
      });
    },
  });
}
