// Usage: React Query adapters for community plugin management.

import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  normalizePluginId,
  pluginDisable,
  pluginEnable,
  pluginGet,
  pluginGrantPermissions,
  pluginInstallFromFile,
  pluginInstallRemote,
  pluginInstallOfficial,
  pluginList,
  pluginListAuditLogs,
  pluginQuarantineRevoked,
  pluginRevokePermission,
  pluginRollback,
  pluginSaveConfig,
  pluginUninstall,
  pluginUpdateFromFile,
  type JsonValue,
  type PluginDetail,
  type PluginSummary,
} from "../services/plugins";
import { pluginKeys } from "./keys";

type QueryClientLike = ReturnType<typeof useQueryClient>;

function refreshPluginQueries(queryClient: QueryClientLike, pluginId: string) {
  queryClient.invalidateQueries({ queryKey: pluginKeys.list() });
  queryClient.invalidateQueries({ queryKey: pluginKeys.detail(pluginId) });
}

function upsertPluginSummary(
  current: PluginSummary[] | undefined,
  detail: PluginDetail
): PluginSummary[] {
  const previous = current ?? [];
  const nextSummary = detail.summary;
  const exists = previous.some((item) => item.plugin_id === nextSummary.plugin_id);
  if (exists) {
    return previous.map((item) => (item.plugin_id === nextSummary.plugin_id ? nextSummary : item));
  }
  return [nextSummary, ...previous];
}

export function usePluginsListQuery(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: pluginKeys.list(),
    queryFn: () => pluginList(),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function usePluginQuery(pluginId: string | null, options?: { enabled?: boolean }) {
  const normalizedPluginId = pluginId == null ? null : normalizePluginId(pluginId);

  return useQuery({
    queryKey: pluginKeys.detail(normalizedPluginId),
    queryFn: () => {
      if (normalizedPluginId == null) return null;
      return pluginGet(normalizedPluginId);
    },
    enabled: normalizedPluginId != null && (options?.enabled ?? true),
    placeholderData: keepPreviousData,
  });
}

export function usePluginAuditLogsQuery(
  pluginId: string | null,
  limit = 50,
  options?: { enabled?: boolean }
) {
  const normalizedPluginId = pluginId == null ? null : normalizePluginId(pluginId);

  return useQuery({
    queryKey: pluginKeys.auditLogs(normalizedPluginId, limit),
    queryFn: () => pluginListAuditLogs({ pluginId: normalizedPluginId, limit }),
    enabled: (options?.enabled ?? true) && normalizedPluginId != null,
    placeholderData: keepPreviousData,
  });
}

export function usePluginInstallFromFileMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (filePath: string) => pluginInstallFromFile(filePath),
    onSuccess: (next) => {
      if (!next) return;
      queryClient.setQueryData<PluginSummary[]>(pluginKeys.list(), (current) =>
        upsertPluginSummary(current, next)
      );
      queryClient.setQueryData(pluginKeys.detail(next.summary.plugin_id), next);
      refreshPluginQueries(queryClient, next.summary.plugin_id);
    },
  });
}

export function usePluginUpdateFromFileMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (filePath: string) => pluginUpdateFromFile(filePath),
    onSuccess: (next) => {
      if (next) {
        queryClient.setQueryData(pluginKeys.detail(next.summary.plugin_id), next);
        refreshPluginQueries(queryClient, next.summary.plugin_id);
      } else {
        queryClient.invalidateQueries({ queryKey: pluginKeys.list() });
      }
    },
  });
}

export function usePluginInstallRemoteMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: Parameters<typeof pluginInstallRemote>[0]) => pluginInstallRemote(input),
    onSuccess: (next) => {
      if (!next) return;
      queryClient.setQueryData<PluginSummary[]>(pluginKeys.list(), (current) =>
        upsertPluginSummary(current, next)
      );
      queryClient.setQueryData(pluginKeys.detail(next.summary.plugin_id), next);
      refreshPluginQueries(queryClient, next.summary.plugin_id);
    },
  });
}

export function usePluginRollbackMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { pluginId: string; version: string }) =>
      pluginRollback(input.pluginId, input.version),
    onSuccess: (next, input) => {
      const normalizedPluginId = normalizePluginId(input.pluginId);
      if (next) {
        queryClient.setQueryData(pluginKeys.detail(normalizedPluginId), next);
      }
      refreshPluginQueries(queryClient, normalizedPluginId);
    },
  });
}

export function usePluginQuarantineRevokedMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (pluginId: string) => pluginQuarantineRevoked(pluginId),
    onSuccess: (next, pluginId) => {
      const normalizedPluginId = normalizePluginId(pluginId);
      if (next) {
        queryClient.setQueryData(pluginKeys.detail(normalizedPluginId), next);
      }
      refreshPluginQueries(queryClient, normalizedPluginId);
    },
  });
}

export function usePluginInstallOfficialMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (pluginId: string) => pluginInstallOfficial(pluginId),
    onSuccess: (next, pluginId) => {
      const normalizedPluginId = normalizePluginId(pluginId);
      if (next) {
        queryClient.setQueryData<PluginSummary[]>(pluginKeys.list(), (current) =>
          upsertPluginSummary(current, next)
        );
        queryClient.setQueryData(pluginKeys.detail(normalizedPluginId), next);
      }
      refreshPluginQueries(queryClient, normalizedPluginId);
    },
  });
}

export function usePluginEnableMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (pluginId: string) => pluginEnable(pluginId),
    onSuccess: (next, pluginId) => {
      const normalizedPluginId = normalizePluginId(pluginId);
      if (next) {
        queryClient.setQueryData(pluginKeys.detail(normalizedPluginId), next);
      }
      refreshPluginQueries(queryClient, normalizedPluginId);
    },
  });
}

export function usePluginDisableMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (pluginId: string) => pluginDisable(pluginId),
    onSuccess: (next, pluginId) => {
      const normalizedPluginId = normalizePluginId(pluginId);
      if (next) {
        queryClient.setQueryData(pluginKeys.detail(normalizedPluginId), next);
      }
      refreshPluginQueries(queryClient, normalizedPluginId);
    },
  });
}

export function usePluginUninstallMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (pluginId: string) => pluginUninstall(pluginId),
    onSuccess: (next, pluginId) => {
      const normalizedPluginId = normalizePluginId(pluginId);
      if (next) {
        queryClient.setQueryData(pluginKeys.detail(normalizedPluginId), next);
      }
      refreshPluginQueries(queryClient, normalizedPluginId);
    },
  });
}

export function usePluginSaveConfigMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { pluginId: string; config: JsonValue }) =>
      pluginSaveConfig(input.pluginId, input.config),
    onSuccess: (next, input) => {
      const normalizedPluginId = normalizePluginId(input.pluginId);
      if (next) {
        queryClient.setQueryData(pluginKeys.detail(normalizedPluginId), next);
      }
      refreshPluginQueries(queryClient, normalizedPluginId);
    },
  });
}

export function usePluginGrantPermissionsMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { pluginId: string; permissions: readonly string[] }) =>
      pluginGrantPermissions(input.pluginId, input.permissions),
    onSuccess: (next, input) => {
      const normalizedPluginId = normalizePluginId(input.pluginId);
      if (next) {
        queryClient.setQueryData(pluginKeys.detail(normalizedPluginId), next);
      }
      refreshPluginQueries(queryClient, normalizedPluginId);
    },
  });
}

export function usePluginRevokePermissionMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { pluginId: string; permission: string }) =>
      pluginRevokePermission(input.pluginId, input.permission),
    onSuccess: (next, input) => {
      const normalizedPluginId = normalizePluginId(input.pluginId);
      if (next) {
        queryClient.setQueryData(pluginKeys.detail(normalizedPluginId), next);
      }
      refreshPluginQueries(queryClient, normalizedPluginId);
    },
  });
}
