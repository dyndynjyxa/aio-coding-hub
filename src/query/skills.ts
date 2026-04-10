// Usage:
// - Query adapters for `src/services/skills.ts`, used by skills pages/views.

import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { CliKey } from "../services/providers/providers";
import {
  skillInstall,
  skillRepoDelete,
  skillRepoUpsert,
  skillInstallToLocal,
  skillReposList,
  skillSetEnabled,
  skillUninstall,
  skillsDiscoverAvailable,
  skillsInstalledList,
  skillLocalDelete,
  skillsLocalList,
  skillsPathsGet,
  skillImportLocal,
  skillsImportLocalBatch,
  skillReturnToLocal,
  type AvailableSkillSummary,
  type InstalledSkillSummary,
  type LocalSkillSummary,
  type SkillImportIssue,
  type SkillImportLocalBatchReport,
  type SkillRepoSummary,
  type SkillsPaths,
} from "../services/workspace/skills";
import { skillsKeys } from "./keys";

export function useSkillReposListQuery(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: skillsKeys.reposList(),
    queryFn: () => skillReposList(),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function useSkillsInstalledListQuery(
  workspaceId: number | null,
  options?: { enabled?: boolean }
) {
  return useQuery({
    queryKey: skillsKeys.installedList(workspaceId),
    queryFn: () => {
      if (!workspaceId) return null;
      return skillsInstalledList(workspaceId);
    },
    enabled: Boolean(workspaceId) && (options?.enabled ?? true),
  });
}

export function useSkillsLocalListQuery(
  workspaceId: number | null,
  options?: { enabled?: boolean }
) {
  return useQuery({
    queryKey: skillsKeys.localList(workspaceId),
    queryFn: () => {
      if (!workspaceId) return null;
      return skillsLocalList(workspaceId);
    },
    enabled: Boolean(workspaceId) && (options?.enabled ?? true),
  });
}

export function useSkillsDiscoverAvailableQuery(refresh: boolean, options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: skillsKeys.discoverAvailable(refresh),
    queryFn: () => skillsDiscoverAvailable(refresh),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function useSkillsDiscoverAvailableMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (refresh: boolean) => skillsDiscoverAvailable(refresh),
    onSuccess: (rows, refresh) => {
      if (!rows) return;
      queryClient.setQueryData<AvailableSkillSummary[]>(
        skillsKeys.discoverAvailable(refresh),
        rows
      );
      queryClient.setQueryData<AvailableSkillSummary[]>(skillsKeys.discoverAvailable(false), rows);
    },
    onSettled: (_res, _err, refresh) => {
      queryClient.invalidateQueries({ queryKey: skillsKeys.discoverAvailable(refresh ?? false) });
      queryClient.invalidateQueries({ queryKey: skillsKeys.discoverAvailable(false) });
    },
  });
}

export function useSkillsPathsQuery(cliKey: CliKey | null, options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: skillsKeys.paths(cliKey),
    queryFn: () => {
      if (!cliKey) return null;
      return skillsPathsGet(cliKey);
    },
    enabled: Boolean(cliKey) && (options?.enabled ?? true),
    placeholderData: keepPreviousData,
  });
}

export function useSkillRepoUpsertMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (input: {
      repoId: number | null;
      gitUrl: string;
      branch: string;
      enabled: boolean;
    }) =>
      skillRepoUpsert({
        repo_id: input.repoId,
        git_url: input.gitUrl,
        branch: input.branch,
        enabled: input.enabled,
      }),
    onSuccess: (next) => {
      if (!next) return;
      queryClient.setQueryData<SkillRepoSummary[]>(skillsKeys.reposList(), (cur) => {
        const prev = cur ?? [];
        const exists = prev.some((r) => r.id === next.id);
        if (exists) return prev.map((r) => (r.id === next.id ? next : r));
        return [next, ...prev];
      });
      queryClient.invalidateQueries({ queryKey: skillsKeys.discoverAvailable(false) });
    },
  });
}

export function useSkillRepoDeleteMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (repoId: number) => skillRepoDelete(repoId),
    onSuccess: (ok, repoId) => {
      if (!ok) return;
      queryClient.setQueryData<SkillRepoSummary[]>(skillsKeys.reposList(), (cur) =>
        (cur ?? []).filter((r) => r.id !== repoId)
      );
      queryClient.invalidateQueries({ queryKey: skillsKeys.discoverAvailable(false) });
    },
  });
}

export function useSkillInstallMutation(workspaceId: number) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (input: {
      gitUrl: string;
      branch: string;
      sourceSubdir: string;
      enabled: boolean;
    }) =>
      skillInstall({
        workspace_id: workspaceId,
        git_url: input.gitUrl,
        branch: input.branch,
        source_subdir: input.sourceSubdir,
        enabled: input.enabled,
      }),
    onSuccess: (next) => {
      if (!next) return;
      queryClient.setQueryData<InstalledSkillSummary[]>(
        skillsKeys.installedList(workspaceId),
        (cur) => {
          const prev = cur ?? [];
          const exists = prev.some((s) => s.id === next.id);
          if (exists) return prev.map((s) => (s.id === next.id ? next : s));
          return [next, ...prev];
        }
      );
      queryClient.invalidateQueries({ queryKey: skillsKeys.discoverAvailable(false) });
    },
  });
}

export function useSkillInstallToLocalMutation(workspaceId: number) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (input: { gitUrl: string; branch: string; sourceSubdir: string }) =>
      skillInstallToLocal({
        workspace_id: workspaceId,
        git_url: input.gitUrl,
        branch: input.branch,
        source_subdir: input.sourceSubdir,
      }),
    onSuccess: (next) => {
      if (!next) return;
      queryClient.setQueryData<LocalSkillSummary[]>(skillsKeys.localList(workspaceId), (cur) => {
        const prev = cur ?? [];
        const exists = prev.some((skill) => skill.dir_name === next.dir_name);
        if (exists) {
          return prev.map((skill) => (skill.dir_name === next.dir_name ? next : skill));
        }
        return [next, ...prev];
      });
    },
  });
}

export function useSkillSetEnabledMutation(workspaceId: number) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (input: { skillId: number; enabled: boolean }) =>
      skillSetEnabled({
        workspace_id: workspaceId,
        skill_id: input.skillId,
        enabled: input.enabled,
      }),
    onSuccess: (next) => {
      if (!next) return;
      queryClient.setQueryData<InstalledSkillSummary[]>(
        skillsKeys.installedList(workspaceId),
        (cur) => (cur ?? []).map((s) => (s.id === next.id ? next : s))
      );
    },
  });
}

export function useSkillUninstallMutation(workspaceId: number) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (skillId: number) => skillUninstall(skillId),
    onSuccess: (ok, skillId) => {
      if (!ok) return;
      queryClient.setQueryData<InstalledSkillSummary[]>(
        skillsKeys.installedList(workspaceId),
        (cur) => (cur ?? []).filter((s) => s.id !== skillId)
      );
      queryClient.invalidateQueries({ queryKey: skillsKeys.discoverAvailable(false) });
    },
  });
}

export function useSkillImportLocalMutation(workspaceId: number) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (dirName: string) =>
      skillImportLocal({ workspace_id: workspaceId, dir_name: dirName }),
    onSuccess: (next) => {
      if (!next) return;
      queryClient.setQueryData<InstalledSkillSummary[]>(
        skillsKeys.installedList(workspaceId),
        (cur) => {
          const prev = cur ?? [];
          const exists = prev.some((s) => s.id === next.id);
          if (exists) return prev.map((s) => (s.id === next.id ? next : s));
          return [next, ...prev];
        }
      );
      queryClient.invalidateQueries({ queryKey: skillsKeys.localList(workspaceId) });
    },
  });
}

export function useSkillReturnToLocalMutation(workspaceId: number) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (skillId: number) =>
      skillReturnToLocal({ workspace_id: workspaceId, skill_id: skillId }),
    onSuccess: (ok, skillId) => {
      if (!ok) return;
      queryClient.setQueryData<InstalledSkillSummary[]>(
        skillsKeys.installedList(workspaceId),
        (cur) => (cur ?? []).filter((s) => s.id !== skillId)
      );
      queryClient.invalidateQueries({ queryKey: skillsKeys.localList(workspaceId) });
      queryClient.invalidateQueries({ queryKey: skillsKeys.discoverAvailable(false) });
    },
  });
}

export function useSkillLocalDeleteMutation(workspaceId: number) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (dirName: string) =>
      skillLocalDelete({ workspace_id: workspaceId, dir_name: dirName }),
    onSuccess: (ok, dirName) => {
      if (!ok) return;
      queryClient.setQueryData<LocalSkillSummary[]>(skillsKeys.localList(workspaceId), (cur) =>
        (cur ?? []).filter((skill) => skill.dir_name !== dirName)
      );
    },
  });
}

export function useSkillsImportLocalBatchMutation(workspaceId: number) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (dirNames: string[]) =>
      skillsImportLocalBatch({ workspace_id: workspaceId, dir_names: dirNames }),
    onSuccess: (report) => {
      if (!report) return;
      const imported = report.imported ?? [];
      queryClient.setQueryData<InstalledSkillSummary[]>(
        skillsKeys.installedList(workspaceId),
        (cur) => {
          const prev = cur ?? [];
          if (imported.length === 0) return prev;
          const byId = new Map(prev.map((item) => [item.id, item]));
          for (const row of imported) {
            byId.set(row.id, row);
          }
          return Array.from(byId.values());
        }
      );
      queryClient.invalidateQueries({ queryKey: skillsKeys.localList(workspaceId) });
    },
  });
}

export type {
  AvailableSkillSummary,
  InstalledSkillSummary,
  LocalSkillSummary,
  SkillImportIssue,
  SkillImportLocalBatchReport,
  SkillRepoSummary,
  SkillsPaths,
};
