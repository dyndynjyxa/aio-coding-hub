import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  cliManagerClaudeInfoGet,
  cliManagerClaudeSettingsJsonGet,
  cliManagerClaudeSettingsJsonSet,
  cliManagerClaudeSettingsGet,
  cliManagerClaudeSettingsSet,
  cliManagerClaudeHooksGet,
  cliManagerClaudeHooksSet,
  cliManagerCodexConfigGet,
  cliManagerCodexConfigSet,
  cliManagerCodexConfigTomlGet,
  cliManagerCodexConfigTomlSet,
  cliManagerCodexInfoGet,
  cliManagerGeminiConfigGet,
  cliManagerGeminiConfigSet,
  cliManagerGeminiInfoGet,
  cliManagerGeminiSettingsJsonGet,
  cliManagerGeminiSettingsJsonSet,
  type ClaudeCliInfo,
  type ClaudeHooksSetInput,
  type ClaudeHooksState,
  type ClaudeSettingsJsonState,
  type ClaudeSettingsPatch,
  type ClaudeSettingsState,
  type CodexConfigPatch,
  type CodexConfigState,
  type GeminiConfigPatch,
  type GeminiConfigState,
  type GeminiSettingsJsonState,
  type SimpleCliInfo,
} from "../services/cli/cliManager";
import { cliManagerKeys } from "./keys";

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

export function useCliManagerClaudeSettingsJsonQuery(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: cliManagerKeys.claudeSettingsJson(),
    queryFn: () => cliManagerClaudeSettingsJsonGet(),
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

export function useCliManagerGeminiSettingsJsonQuery(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: cliManagerKeys.geminiSettingsJson(),
    queryFn: () => cliManagerGeminiSettingsJsonGet(),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
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

export function useCliManagerClaudeSettingsJsonSetMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { json: string }) => cliManagerClaudeSettingsJsonSet(input.json),
    onSuccess: (next) => {
      if (!next) return;
      queryClient.setQueryData<ClaudeSettingsJsonState | null>(
        cliManagerKeys.claudeSettingsJson(),
        next
      );
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: cliManagerKeys.claudeSettings() });
      queryClient.invalidateQueries({ queryKey: cliManagerKeys.claudeSettingsJson() });
    },
  });
}

export function useCliManagerCodexConfigSetMutation() {
  const queryClient = useQueryClient();

  return useMutation({
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

export function useCliManagerGeminiSettingsJsonSetMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { json: string }) => cliManagerGeminiSettingsJsonSet(input.json),
    onSuccess: (next) => {
      if (!next) return;
      queryClient.setQueryData<GeminiSettingsJsonState | null>(
        cliManagerKeys.geminiSettingsJson(),
        next
      );
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: cliManagerKeys.geminiConfig() });
      queryClient.invalidateQueries({ queryKey: cliManagerKeys.geminiSettingsJson() });
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
