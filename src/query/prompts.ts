import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  promptDelete,
  promptSetEnabled,
  promptUpsert,
  promptsList,
  type PromptSummary,
} from "../services/workspace/prompts";
import { promptsKeys } from "./keys";

export function usePromptsListQuery(workspaceId: number | null, options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: promptsKeys.list(workspaceId),
    queryFn: () => {
      if (workspaceId == null) return null;
      return promptsList(workspaceId);
    },
    enabled: workspaceId != null && (options?.enabled ?? true),
    placeholderData: keepPreviousData,
  });
}

export function usePromptUpsertMutation(workspaceId: number) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: {
      promptId: number | null;
      name: string;
      content: string;
      enabled: boolean;
    }) =>
      promptUpsert({
        prompt_id: input.promptId,
        workspace_id: workspaceId,
        name: input.name,
        content: input.content,
        enabled: input.enabled,
      }),
    onSuccess: (next) => {
      if (!next) return;

      queryClient.setQueryData<PromptSummary[] | null>(promptsKeys.list(workspaceId), (prev) => {
        const base = prev ?? [];
        const exists = base.some((p) => p.id === next.id);
        const nextItems = exists ? base.map((p) => (p.id === next.id ? next : p)) : [next, ...base];
        return nextItems;
      });
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: promptsKeys.list(workspaceId) });
    },
  });
}

export function usePromptSetEnabledMutation(workspaceId: number) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { promptId: number; enabled: boolean }) =>
      promptSetEnabled(input.promptId, input.enabled),
    onSuccess: (next) => {
      if (!next) return;

      queryClient.setQueryData<PromptSummary[] | null>(promptsKeys.list(workspaceId), (prev) => {
        if (!prev) return prev;
        return prev.map((p) => {
          if (p.id === next.id) return next;
          if (next.enabled) return { ...p, enabled: false };
          return p;
        });
      });
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: promptsKeys.list(workspaceId) });
    },
  });
}

export function usePromptDeleteMutation(workspaceId: number) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (promptId: number) => promptDelete(promptId),
    onSuccess: (ok, promptId) => {
      if (!ok) return;
      queryClient.setQueryData<PromptSummary[] | null>(promptsKeys.list(workspaceId), (prev) => {
        if (!prev) return prev;
        return prev.filter((p) => p.id !== promptId);
      });
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: promptsKeys.list(workspaceId) });
    },
  });
}
