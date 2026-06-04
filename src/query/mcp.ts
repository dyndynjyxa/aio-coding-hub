// Usage:
// - Query adapters for `src/services/mcp.ts`, used by MCP pages/views.

import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  mcpImportFromWorkspaceCli,
  mcpImportServers,
  type McpImportServer,
  type McpImportReport,
  type McpSecretPatchInput,
  mcpServerDelete,
  mcpServerSetEnabled,
  mcpServerUpsert,
  mcpServersList,
  type McpServerSummary,
  type McpTransport,
  validateMcpWorkspaceId,
} from "../services/workspace/mcp";
import { mcpKeys } from "./keys";

export function useMcpServersListQuery(
  workspaceId: number | null,
  options?: { enabled?: boolean }
) {
  const normalizedWorkspaceId = workspaceId == null ? null : validateMcpWorkspaceId(workspaceId);

  return useQuery({
    queryKey: mcpKeys.serversList(normalizedWorkspaceId),
    queryFn: () => {
      if (normalizedWorkspaceId == null) return null;
      return mcpServersList(normalizedWorkspaceId);
    },
    enabled: normalizedWorkspaceId != null && (options?.enabled ?? true),
    placeholderData: keepPreviousData,
  });
}

export function useMcpServerUpsertMutation(workspaceId: number) {
  const normalizedWorkspaceId = validateMcpWorkspaceId(workspaceId);
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (input: {
      serverId: number | null;
      serverKey: string;
      name: string;
      transport: McpTransport;
      command: string | null;
      args: string[];
      env?: McpSecretPatchInput;
      cwd: string | null;
      url: string | null;
      headers?: McpSecretPatchInput;
    }) => mcpServerUpsert(input),
    onSuccess: (next) => {
      queryClient.setQueryData<McpServerSummary[]>(
        mcpKeys.serversList(normalizedWorkspaceId),
        (cur) => {
          const prev = cur ?? [];
          const exists = prev.some((s) => s.id === next.id);
          if (exists) return prev.map((s) => (s.id === next.id ? next : s));
          return [next, ...prev];
        }
      );
    },
  });
}

export function useMcpServerSetEnabledMutation(workspaceId: number) {
  const normalizedWorkspaceId = validateMcpWorkspaceId(workspaceId);
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (input: { serverId: number; enabled: boolean }) =>
      mcpServerSetEnabled({
        workspaceId: normalizedWorkspaceId,
        serverId: input.serverId,
        enabled: input.enabled,
      }),
    onSuccess: (next) => {
      queryClient.setQueryData<McpServerSummary[]>(
        mcpKeys.serversList(normalizedWorkspaceId),
        (cur) => (cur ?? []).map((s) => (s.id === next.id ? next : s))
      );
    },
  });
}

export function useMcpServerDeleteMutation(workspaceId: number) {
  const normalizedWorkspaceId = validateMcpWorkspaceId(workspaceId);
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (serverId: number) => mcpServerDelete(serverId),
    onSuccess: (ok, serverId) => {
      if (!ok) return;
      queryClient.setQueryData<McpServerSummary[]>(
        mcpKeys.serversList(normalizedWorkspaceId),
        (cur) => (cur ?? []).filter((s) => s.id !== serverId)
      );
    },
  });
}

export function useMcpImportServersMutation(workspaceId: number) {
  const normalizedWorkspaceId = validateMcpWorkspaceId(workspaceId);
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (servers: McpImportServer[]) =>
      mcpImportServers({ workspaceId: normalizedWorkspaceId, servers }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: mcpKeys.serversList(normalizedWorkspaceId) });
    },
  });
}

export function useMcpImportFromWorkspaceCliMutation(workspaceId: number) {
  const normalizedWorkspaceId = validateMcpWorkspaceId(workspaceId);
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async () => mcpImportFromWorkspaceCli(normalizedWorkspaceId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: mcpKeys.serversList(normalizedWorkspaceId) });
    },
  });
}

export type { McpImportReport, McpImportServer };
