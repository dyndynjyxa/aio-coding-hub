import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  cliManagerClaudeInfoGet,
  cliManagerClaudeSettingsGet,
  cliManagerClaudeSettingsSet,
  cliManagerClaudeHooksGet,
  cliManagerClaudeHooksSet,
  cliManagerCodexConfigGet,
  cliManagerCodexConfigSet,
  cliManagerCodexConfigTomlGet,
  cliManagerCodexConfigTomlSet,
  cliManagerCodexInfoGet,
  cliManagerCodexModelCatalogGet,
  cliManagerGeminiConfigGet,
  cliManagerGeminiConfigSet,
  cliManagerGeminiInfoGet,
  cliManagerGrokConfigGet,
  cliManagerGrokConfigSet,
  cliManagerGrokInfoGet,
  type ClaudeCliInfo,
  type ClaudeHooksSetInput,
  type ClaudeHooksState,
  type ClaudeSettingsPatch,
  type ClaudeSettingsState,
  type CodexConfigPatch,
  type CodexConfigState,
  type CodexModelCatalogState,
  type GeminiConfigPatch,
  type GeminiConfigState,
  type GrokConfigState,
  type GrokProxyPreferences,
  type SimpleCliInfo,
} from "../services/cli/cliManager";
import { cliManagerKeys } from "./keys";

const CODEX_CONFIG_MUTATION_SCOPE = "codex-config";
const CODEX_MODEL_CATALOG_STALE_TIME = 5 * 60 * 1000;

export type CodexModelCatalogQuerySnapshot = {
  configPath?: string | null;
  executablePath?: string | null;
  cliVersion?: string | null;
};

function hasCodexModelCatalogSnapshot(snapshot?: CodexModelCatalogQuerySnapshot) {
  return Boolean(snapshot?.configPath && snapshot?.executablePath);
}

function codexModelCatalogQueryOptions(snapshot?: CodexModelCatalogQuerySnapshot) {
  return {
    queryKey: cliManagerKeys.codexModelCatalog(snapshot),
    queryFn: () => cliManagerCodexModelCatalogGet(),
    staleTime: CODEX_MODEL_CATALOG_STALE_TIME,
    retry: false,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
  };
}

export function useCliManagerClaudeInfoQuery(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: cliManagerKeys.claudeInfo(),
    queryFn: () => cliManagerClaudeInfoGet(),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function useCliManagerClaudeSettingsQuery(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: cliManagerKeys.claudeSettings(),
    queryFn: () => cliManagerClaudeSettingsGet(),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function useCliManagerCodexInfoQuery(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: cliManagerKeys.codexInfo(),
    queryFn: () => cliManagerCodexInfoGet(),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function useCliManagerCodexModelCatalogQuery(options?: {
  enabled?: boolean;
  snapshot?: CodexModelCatalogQuerySnapshot;
}) {
  const snapshot = options?.snapshot;
  const hasSnapshot = hasCodexModelCatalogSnapshot(snapshot);
  const enabled = (options?.enabled ?? true) && hasSnapshot;
  return useQuery<CodexModelCatalogState | null>({
    ...codexModelCatalogQueryOptions(snapshot),
    enabled: (query) => enabled && query.state.status !== "error",
  });
}

export function useCliManagerCodexModelCatalogRefresh() {
  const queryClient = useQueryClient();
  return async (snapshot: CodexModelCatalogQuerySnapshot) => {
    if (!hasCodexModelCatalogSnapshot(snapshot)) return;

    const queryOptions = codexModelCatalogQueryOptions(snapshot);
    await queryClient.invalidateQueries({
      queryKey: queryOptions.queryKey,
      exact: true,
      refetchType: "none",
    });
    await queryClient.prefetchQuery(queryOptions);
  };
}

export function useCliManagerCodexConfigQuery(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: cliManagerKeys.codexConfig(),
    queryFn: () => cliManagerCodexConfigGet(),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function useCliManagerCodexConfigTomlQuery(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: cliManagerKeys.codexConfigToml(),
    queryFn: () => cliManagerCodexConfigTomlGet(),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function useCliManagerGeminiInfoQuery(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: cliManagerKeys.geminiInfo(),
    queryFn: () => cliManagerGeminiInfoGet(),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function useCliManagerGeminiConfigQuery(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: cliManagerKeys.geminiConfig(),
    queryFn: () => cliManagerGeminiConfigGet(),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function useCliManagerGrokInfoQuery(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: cliManagerKeys.grokInfo(),
    queryFn: () => cliManagerGrokInfoGet(),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function useCliManagerGrokConfigQuery(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: cliManagerKeys.grokConfig(),
    queryFn: () => cliManagerGrokConfigGet(),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function useCliManagerGrokConfigSetMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (preferences: GrokProxyPreferences) => cliManagerGrokConfigSet(preferences),
    onSuccess: (next) => {
      if (!next) return;
      queryClient.setQueryData<GrokConfigState | null>(cliManagerKeys.grokConfig(), next);
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: cliManagerKeys.grokConfig() });
    },
  });
}

export function useCliManagerClaudeSettingsSetMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (patch: ClaudeSettingsPatch) => cliManagerClaudeSettingsSet(patch),
    onSuccess: (next) => {
      if (!next) return;
      queryClient.setQueryData<ClaudeSettingsState | null>(cliManagerKeys.claudeSettings(), next);
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: cliManagerKeys.claudeSettings() });
    },
  });
}

export function useCliManagerCodexConfigSetMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    scope: { id: CODEX_CONFIG_MUTATION_SCOPE },
    mutationFn: (patch: CodexConfigPatch) => cliManagerCodexConfigSet(patch),
    onSuccess: (next) => {
      if (!next) return;
      queryClient.setQueryData<CodexConfigState | null>(cliManagerKeys.codexConfig(), next);
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: cliManagerKeys.codexConfig() });
      queryClient.invalidateQueries({ queryKey: cliManagerKeys.codexConfigToml() });
    },
  });
}

export function useCliManagerCodexConfigTomlSetMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    scope: { id: CODEX_CONFIG_MUTATION_SCOPE },
    mutationFn: (input: { toml: string }) => cliManagerCodexConfigTomlSet(input.toml),
    onSuccess: (next) => {
      if (!next) return;
      queryClient.setQueryData<CodexConfigState | null>(cliManagerKeys.codexConfig(), next);
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: cliManagerKeys.codexConfig() });
      queryClient.invalidateQueries({ queryKey: cliManagerKeys.codexConfigToml() });
    },
  });
}

export function useCliManagerGeminiConfigSetMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (patch: GeminiConfigPatch) => cliManagerGeminiConfigSet(patch),
    onSuccess: (next) => {
      if (!next) return;
      queryClient.setQueryData<GeminiConfigState | null>(cliManagerKeys.geminiConfig(), next);
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: cliManagerKeys.geminiConfig() });
    },
  });
}

export function useCliManagerClaudeHooksQuery(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: cliManagerKeys.claudeHooks(),
    queryFn: () => cliManagerClaudeHooksGet(),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function useCliManagerClaudeHooksSetMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: ClaudeHooksSetInput) => cliManagerClaudeHooksSet(input),
    onSuccess: (next) => {
      if (!next) return;
      queryClient.setQueryData<ClaudeHooksState | null>(cliManagerKeys.claudeHooks(), next);
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: cliManagerKeys.claudeHooks() });
    },
  });
}

export function pickCliAvailable(info: SimpleCliInfo | ClaudeCliInfo | null) {
  if (!info) return "unavailable" as const;
  return info.found ? ("available" as const) : ("unavailable" as const);
}
