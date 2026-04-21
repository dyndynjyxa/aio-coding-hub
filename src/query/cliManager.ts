import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  cliManagerClaudeInfoGet,
  cliManagerClaudeSettingsGet,
  cliManagerClaudeSettingsSet,
  cliManagerCodexConfigGet,
  cliManagerCodexConfigSet,
  cliManagerCodexConfigTomlGet,
  cliManagerCodexConfigTomlSet,
  cliManagerCodexInfoGet,
  cliManagerGeminiConfigGet,
  cliManagerGeminiConfigSet,
  cliManagerGeminiInfoGet,
  type ClaudeCliInfo,
  type ClaudeSettingsPatch,
  type ClaudeSettingsState,
  type CodexConfigPatch,
  type CodexConfigState,
  type GeminiConfigPatch,
  type GeminiConfigState,
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

export function pickCliAvailable(info: SimpleCliInfo | ClaudeCliInfo | null) {
  if (!info) return "unavailable" as const;
  return info.found ? ("available" as const) : ("unavailable" as const);
}
