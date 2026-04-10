import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { CliKey } from "../services/providers/providers";
import {
  workspaceApply,
  workspaceCreate,
  workspaceDelete,
  workspacePreview,
  workspaceRename,
  workspacesList,
  type WorkspacePreview,
  type WorkspaceSummary,
  type WorkspacesListResult,
} from "../services/workspace/workspaces";
import { workspacesKeys } from "./keys";

export function useWorkspacesListQuery(cliKey: CliKey, options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: workspacesKeys.list(cliKey),
    queryFn: () => workspacesList(cliKey),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function useWorkspacePreviewQuery(
  workspaceId: number | null,
  options?: { enabled?: boolean }
) {
  return useQuery({
    queryKey: workspacesKeys.preview(workspaceId),
    queryFn: () => {
      if (workspaceId == null) return null;
      return workspacePreview(workspaceId);
    },
    enabled: workspaceId != null && (options?.enabled ?? true),
    placeholderData: keepPreviousData,
  });
}

export function useWorkspaceCreateMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { cliKey: CliKey; name: string; cloneFromActive?: boolean }) =>
      workspaceCreate({
        cli_key: input.cliKey,
        name: input.name,
        clone_from_active: input.cloneFromActive,
      }),
    onSuccess: (created, input) => {
      if (!created) return;

      queryClient.setQueryData<WorkspacesListResult | null>(
        workspacesKeys.list(input.cliKey),
        (prev) => {
          if (!prev) return { active_id: null, items: [created] };
          const exists = prev.items.some((w) => w.id === created.id);
          const nextItems = exists
            ? prev.items.map((w) => (w.id === created.id ? created : w))
            : [created, ...prev.items];
          return { ...prev, items: nextItems };
        }
      );
    },
    onSettled: (_res, _err, input) => {
      if (input) queryClient.invalidateQueries({ queryKey: workspacesKeys.list(input.cliKey) });
    },
  });
}

export function useWorkspaceRenameMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { cliKey: CliKey; workspaceId: number; name: string }) =>
      workspaceRename({ workspace_id: input.workspaceId, name: input.name }),
    onSuccess: (updated, input) => {
      if (!updated) return;
      queryClient.setQueryData<WorkspacesListResult | null>(
        workspacesKeys.list(input.cliKey),
        (prev) => {
          if (!prev) return prev;
          return {
            ...prev,
            items: prev.items.map((w) => (w.id === updated.id ? updated : w)),
          };
        }
      );
    },
    onSettled: (_res, _err, input) => {
      if (input) queryClient.invalidateQueries({ queryKey: workspacesKeys.list(input.cliKey) });
    },
  });
}

export function useWorkspaceDeleteMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { cliKey: CliKey; workspaceId: number }) =>
      workspaceDelete(input.workspaceId),
    onSuccess: (ok, input) => {
      if (!ok) return;
      queryClient.setQueryData<WorkspacesListResult | null>(
        workspacesKeys.list(input.cliKey),
        (prev) => {
          if (!prev) return prev;
          const nextItems = prev.items.filter((w) => w.id !== input.workspaceId);
          const nextActiveId = prev.active_id === input.workspaceId ? null : prev.active_id;
          return { ...prev, active_id: nextActiveId, items: nextItems };
        }
      );
    },
    onSettled: (_res, _err, input) => {
      if (input) queryClient.invalidateQueries({ queryKey: workspacesKeys.list(input.cliKey) });
    },
  });
}

export function useWorkspaceApplyMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { cliKey: CliKey; workspaceId: number }) =>
      workspaceApply(input.workspaceId),
    onSuccess: (report, input) => {
      if (!report) return;

      queryClient.setQueryData<WorkspacesListResult | null>(
        workspacesKeys.list(input.cliKey),
        (prev) => {
          if (!prev) return prev;
          return { ...prev, active_id: input.workspaceId };
        }
      );

      queryClient.invalidateQueries({ queryKey: workspacesKeys.preview(input.workspaceId) });
    },
    onSettled: (_res, _err, input) => {
      if (input) queryClient.invalidateQueries({ queryKey: workspacesKeys.list(input.cliKey) });
    },
  });
}

export function pickWorkspaceById(items: WorkspaceSummary[], id: number | null) {
  if (id == null) return null;
  const byId = new Map(items.map((w) => [w.id, w]));
  return byId.get(id) ?? null;
}

export function isWorkspacePreviewReady(
  preview: WorkspacePreview | null,
  workspaceId: number | null
) {
  if (!preview) return false;
  return workspaceId != null && preview.to_workspace_id === workspaceId;
}
