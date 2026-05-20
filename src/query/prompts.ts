import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  promptDelete,
  promptSetEnabled,
  promptUpsert,
  promptsList,
  type PromptSummary,
  validatePromptWorkspaceId,
} from "../services/workspace/prompts";
import { promptsKeys } from "./keys";

export function usePromptsListQuery(workspaceId: number | null, options?: { enabled?: boolean }) {
  const normalizedWorkspaceId = workspaceId == null ? null : validatePromptWorkspaceId(workspaceId);

  return useQuery({
    queryKey: promptsKeys.list(normalizedWorkspaceId),
    queryFn: () => {
      if (normalizedWorkspaceId == null) return null;
      return promptsList(normalizedWorkspaceId);
    },
    enabled: normalizedWorkspaceId != null && (options?.enabled ?? true),
    placeholderData: keepPreviousData,
  });
}

export function usePromptUpsertMutation(workspaceId: number) {
  const normalizedWorkspaceId = validatePromptWorkspaceId(workspaceId);
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: {
      promptId: number | null;
      name: string;
      content: string;
      enabled: boolean;
    }) =>
      promptUpsert({
        promptId: input.promptId,
        workspaceId: normalizedWorkspaceId,
        name: input.name,
        content: input.content,
        enabled: input.enabled,
      }),
    onSuccess: (next) => {
      if (!next) return;

      queryClient.setQueryData<PromptSummary[] | null>(
        promptsKeys.list(normalizedWorkspaceId),
        (prev) => {
          const base = prev ?? [];
          const exists = base.some((p) => p.id === next.id);
          const nextItems = exists
            ? base.map((p) => (p.id === next.id ? next : p))
            : [next, ...base];
          return nextItems;
        }
      );
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: promptsKeys.list(normalizedWorkspaceId) });
    },
  });
}

export function usePromptSetEnabledMutation(workspaceId: number) {
  const normalizedWorkspaceId = validatePromptWorkspaceId(workspaceId);
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { promptId: number; enabled: boolean }) =>
      promptSetEnabled(input.promptId, input.enabled),
    onSuccess: (next) => {
      if (!next) return;

      queryClient.setQueryData<PromptSummary[] | null>(
        promptsKeys.list(normalizedWorkspaceId),
        (prev) => {
          if (!prev) return prev;
          return prev.map((p) => {
            if (p.id === next.id) return next;
            if (next.enabled) return { ...p, enabled: false };
            return p;
          });
        }
      );
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: promptsKeys.list(normalizedWorkspaceId) });
    },
  });
}

export function usePromptDeleteMutation(workspaceId: number) {
  const normalizedWorkspaceId = validatePromptWorkspaceId(workspaceId);
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (promptId: number) => promptDelete(promptId),
    onSuccess: (ok, promptId) => {
      if (!ok) return;
      queryClient.setQueryData<PromptSummary[] | null>(
        promptsKeys.list(normalizedWorkspaceId),
        (prev) => {
          if (!prev) return prev;
          return prev.filter((p) => p.id !== promptId);
        }
      );
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: promptsKeys.list(normalizedWorkspaceId) });
    },
  });
}
