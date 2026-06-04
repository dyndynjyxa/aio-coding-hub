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
  normalizeWorkspaceName,
  validateWorkspaceCliKey,
  validateWorkspaceId,
} from "../services/workspace/workspaces";
import { workspacesKeys } from "./keys";

function maybeValidateWorkspaceCliKey(input: { cliKey: CliKey } | undefined): CliKey | null {
  if (!input) return null;
  try {
    return validateWorkspaceCliKey(input.cliKey);
  } catch {
    return null;
  }
}

export function useWorkspacesListQuery(cliKey: CliKey, options?: { enabled?: boolean }) {
  const normalizedCliKey = validateWorkspaceCliKey(cliKey);

  return useQuery({
    queryKey: workspacesKeys.list(normalizedCliKey),
    queryFn: () => workspacesList(normalizedCliKey),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function useWorkspacePreviewQuery(
  workspaceId: number | null,
  options?: { enabled?: boolean }
) {
  const normalizedWorkspaceId = workspaceId == null ? null : validateWorkspaceId(workspaceId);

  return useQuery({
    queryKey: workspacesKeys.preview(normalizedWorkspaceId),
    queryFn: () => {
      if (normalizedWorkspaceId == null) return null;
      return workspacePreview(normalizedWorkspaceId);
    },
    enabled: normalizedWorkspaceId != null && (options?.enabled ?? true),
    placeholderData: keepPreviousData,
  });
}

export function useWorkspaceCreateMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { cliKey: CliKey; name: string; cloneFromActive?: boolean }) =>
      workspaceCreate({
        cliKey: validateWorkspaceCliKey(input.cliKey),
        name: normalizeWorkspaceName(input.name),
        cloneFromActive: input.cloneFromActive,
      }),
    onSuccess: (created) => {
      if (!created) return;

      const cliKey = validateWorkspaceCliKey(created.cli_key);

      queryClient.setQueryData<WorkspacesListResult | null>(workspacesKeys.list(cliKey), (prev) => {
        if (!prev) return { active_id: null, items: [created] };
        const exists = prev.items.some((w) => w.id === created.id);
        const nextItems = exists
          ? prev.items.map((w) => (w.id === created.id ? created : w))
          : [created, ...prev.items];
        return { ...prev, items: nextItems };
      });
    },
    onSettled: (_res, _err, input) => {
      const cliKey = maybeValidateWorkspaceCliKey(input);
      if (cliKey) queryClient.invalidateQueries({ queryKey: workspacesKeys.list(cliKey) });
    },
  });
}

export function useWorkspaceRenameMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { cliKey: CliKey; workspaceId: number; name: string }) =>
      workspaceRename({
        workspaceId: validateWorkspaceId(input.workspaceId),
        name: normalizeWorkspaceName(input.name),
      }),
    onSuccess: (updated) => {
      if (!updated) return;
      const cliKey = validateWorkspaceCliKey(updated.cli_key);
      queryClient.setQueryData<WorkspacesListResult | null>(workspacesKeys.list(cliKey), (prev) => {
        if (!prev) return prev;
        return {
          ...prev,
          items: prev.items.map((w) => (w.id === updated.id ? updated : w)),
        };
      });
    },
    onSettled: (_res, _err, input) => {
      const cliKey = maybeValidateWorkspaceCliKey(input);
      if (cliKey) queryClient.invalidateQueries({ queryKey: workspacesKeys.list(cliKey) });
    },
  });
}

export function useWorkspaceDeleteMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { cliKey: CliKey; workspaceId: number }) =>
      workspaceDelete(validateWorkspaceId(input.workspaceId)),
    onSuccess: (ok, input) => {
      if (!ok) return;
      const cliKey = validateWorkspaceCliKey(input.cliKey);
      const workspaceId = validateWorkspaceId(input.workspaceId);
      queryClient.setQueryData<WorkspacesListResult | null>(workspacesKeys.list(cliKey), (prev) => {
        if (!prev) return prev;
        const nextItems = prev.items.filter((w) => w.id !== workspaceId);
        const nextActiveId = prev.active_id === workspaceId ? null : prev.active_id;
        return { ...prev, active_id: nextActiveId, items: nextItems };
      });
    },
    onSettled: (_res, _err, input) => {
      const cliKey = maybeValidateWorkspaceCliKey(input);
      if (cliKey) queryClient.invalidateQueries({ queryKey: workspacesKeys.list(cliKey) });
    },
  });
}

export function useWorkspaceApplyMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { cliKey: CliKey; workspaceId: number }) =>
      workspaceApply(validateWorkspaceId(input.workspaceId)),
    onSuccess: (report) => {
      if (!report) return;

      const cliKey = validateWorkspaceCliKey(report.cli_key);
      const workspaceId = validateWorkspaceId(report.to_workspace_id);

      queryClient.setQueryData<WorkspacesListResult | null>(workspacesKeys.list(cliKey), (prev) => {
        if (!prev) return prev;
        return { ...prev, active_id: workspaceId };
      });

      queryClient.invalidateQueries({ queryKey: workspacesKeys.preview(workspaceId) });
    },
    onSettled: (_res, _err, input) => {
      const cliKey = maybeValidateWorkspaceCliKey(input);
      if (cliKey) queryClient.invalidateQueries({ queryKey: workspacesKeys.list(cliKey) });
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
