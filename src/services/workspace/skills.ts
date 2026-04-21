import { commands } from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";
import type { CliKey } from "../providers/providers";

export type SkillRepoSummary = {
  id: number;
  git_url: string;
  branch: string;
  enabled: boolean;
  created_at: number;
  updated_at: number;
};

export type InstalledSkillSummary = {
  id: number;
  skill_key: string;
  name: string;
  description: string;
  source_git_url: string;
  source_branch: string;
  source_subdir: string;
  installed_commit?: string | null;
  enabled: boolean;
  created_at: number;
  updated_at: number;
};

export type AvailableSkillSummary = {
  name: string;
  description: string;
  source_git_url: string;
  source_branch: string;
  source_subdir: string;
  installed: boolean;
};

export type SkillsPaths = {
  ssot_dir: string;
  repos_dir: string;
  cli_dir: string;
};

export type LocalSkillSummary = {
  dir_name: string;
  path: string;
  name: string;
  description: string;
  source_git_url?: string | null;
  source_branch?: string | null;
  source_subdir?: string | null;
};

export type SkillImportIssue = {
  dir_name: string;
  error_code: string | null;
  message: string;
};

export type SkillImportLocalBatchReport = {
  imported: InstalledSkillSummary[];
  skipped: SkillImportIssue[];
  failed: SkillImportIssue[];
};

export type SkillUpdateInfo = {
  skill_id: number;
  has_update: boolean;
  installed_commit?: string | null;
  latest_commit?: string | null;
};

export async function skillReposList() {
  return invokeGeneratedIpc<SkillRepoSummary[]>({
    title: "读取技能仓库列表失败",
    cmd: "skill_repos_list",
    invoke: () => commands.skillReposList() as Promise<GeneratedCommandResult<SkillRepoSummary[]>>,
  });
}

export async function skillRepoUpsert(input: {
  repo_id?: number | null;
  git_url: string;
  branch: string;
  enabled: boolean;
}) {
  return invokeGeneratedIpc<SkillRepoSummary>({
    title: "保存技能仓库失败",
    cmd: "skill_repo_upsert",
    args: {
      repoId: input.repo_id ?? null,
      gitUrl: input.git_url,
      branch: input.branch,
      enabled: input.enabled,
    },
    invoke: () =>
      commands.skillRepoUpsert(
        input.repo_id ?? null,
        input.git_url,
        input.branch,
        input.enabled
      ) as Promise<GeneratedCommandResult<SkillRepoSummary>>,
  });
}

export async function skillRepoDelete(repoId: number) {
  return invokeGeneratedIpc<boolean>({
    title: "删除技能仓库失败",
    cmd: "skill_repo_delete",
    args: { repoId },
    invoke: () => commands.skillRepoDelete(repoId) as Promise<GeneratedCommandResult<boolean>>,
  });
}

export async function skillsInstalledList(workspaceId: number) {
  return invokeGeneratedIpc<InstalledSkillSummary[]>({
    title: "读取已安装技能失败",
    cmd: "skills_installed_list",
    args: { workspaceId },
    invoke: () =>
      commands.skillsInstalledList(workspaceId) as Promise<
        GeneratedCommandResult<InstalledSkillSummary[]>
      >,
  });
}

export async function skillsDiscoverAvailable(refresh: boolean) {
  return invokeGeneratedIpc<AvailableSkillSummary[]>({
    title: "发现可用技能失败",
    cmd: "skills_discover_available",
    args: { refresh },
    invoke: () =>
      commands.skillsDiscoverAvailable(refresh) as Promise<
        GeneratedCommandResult<AvailableSkillSummary[]>
      >,
  });
}

export async function skillInstall(input: {
  workspace_id: number;
  git_url: string;
  branch: string;
  source_subdir: string;
  enabled: boolean;
}) {
  return invokeGeneratedIpc<InstalledSkillSummary>({
    title: "安装技能失败",
    cmd: "skill_install",
    args: {
      workspaceId: input.workspace_id,
      gitUrl: input.git_url,
      branch: input.branch,
      sourceSubdir: input.source_subdir,
      enabled: input.enabled,
    },
    invoke: () =>
      commands.skillInstall(
        input.workspace_id,
        input.git_url,
        input.branch,
        input.source_subdir,
        input.enabled
      ) as Promise<GeneratedCommandResult<InstalledSkillSummary>>,
  });
}

export async function skillInstallToLocal(input: {
  workspace_id: number;
  git_url: string;
  branch: string;
  source_subdir: string;
}) {
  return invokeGeneratedIpc<LocalSkillSummary>({
    title: "安装到当前 CLI 失败",
    cmd: "skill_install_to_local",
    args: {
      workspaceId: input.workspace_id,
      gitUrl: input.git_url,
      branch: input.branch,
      sourceSubdir: input.source_subdir,
    },
    invoke: () =>
      commands.skillInstallToLocal(
        input.workspace_id,
        input.git_url,
        input.branch,
        input.source_subdir
      ) as Promise<GeneratedCommandResult<LocalSkillSummary>>,
  });
}

export async function skillSetEnabled(input: {
  workspace_id: number;
  skill_id: number;
  enabled: boolean;
}) {
  return invokeGeneratedIpc<InstalledSkillSummary>({
    title: "更新技能启用状态失败",
    cmd: "skill_set_enabled",
    args: {
      workspaceId: input.workspace_id,
      skillId: input.skill_id,
      enabled: input.enabled,
    },
    invoke: () =>
      commands.skillSetEnabled(input.workspace_id, input.skill_id, input.enabled) as Promise<
        GeneratedCommandResult<InstalledSkillSummary>
      >,
  });
}

export async function skillUninstall(skillId: number) {
  return invokeGeneratedIpc<boolean>({
    title: "卸载技能失败",
    cmd: "skill_uninstall",
    args: { skillId },
    invoke: () => commands.skillUninstall(skillId) as Promise<GeneratedCommandResult<boolean>>,
  });
}

export async function skillReturnToLocal(input: { workspace_id: number; skill_id: number }) {
  return invokeGeneratedIpc<boolean>({
    title: "返回本机技能失败",
    cmd: "skill_return_to_local",
    args: {
      workspaceId: input.workspace_id,
      skillId: input.skill_id,
    },
    invoke: () =>
      commands.skillReturnToLocal(input.workspace_id, input.skill_id) as Promise<
        GeneratedCommandResult<boolean>
      >,
  });
}

export async function skillsLocalList(workspaceId: number) {
  return invokeGeneratedIpc<LocalSkillSummary[]>({
    title: "读取本地技能列表失败",
    cmd: "skills_local_list",
    args: { workspaceId },
    invoke: () =>
      commands.skillsLocalList(workspaceId) as Promise<
        GeneratedCommandResult<LocalSkillSummary[]>
      >,
  });
}

export async function skillLocalDelete(input: { workspace_id: number; dir_name: string }) {
  return invokeGeneratedIpc<boolean>({
    title: "删除本地技能失败",
    cmd: "skill_local_delete",
    args: {
      workspaceId: input.workspace_id,
      dirName: input.dir_name,
    },
    invoke: () =>
      commands.skillLocalDelete(input.workspace_id, input.dir_name) as Promise<
        GeneratedCommandResult<boolean>
      >,
  });
}

export async function skillImportLocal(input: { workspace_id: number; dir_name: string }) {
  return invokeGeneratedIpc<InstalledSkillSummary>({
    title: "导入本地技能失败",
    cmd: "skill_import_local",
    args: {
      workspaceId: input.workspace_id,
      dirName: input.dir_name,
    },
    invoke: () =>
      commands.skillImportLocal(input.workspace_id, input.dir_name) as Promise<
        GeneratedCommandResult<InstalledSkillSummary>
      >,
  });
}

export async function skillsImportLocalBatch(input: { workspace_id: number; dir_names: string[] }) {
  return invokeGeneratedIpc<SkillImportLocalBatchReport>({
    title: "批量导入本地技能失败",
    cmd: "skills_import_local_batch",
    args: {
      workspaceId: input.workspace_id,
      dirNames: input.dir_names,
    },
    invoke: () =>
      commands.skillsImportLocalBatch(input.workspace_id, input.dir_names) as Promise<
        GeneratedCommandResult<SkillImportLocalBatchReport>
      >,
  });
}

export async function skillsPathsGet(cliKey: CliKey) {
  return invokeGeneratedIpc<SkillsPaths>({
    title: "读取技能路径失败",
    cmd: "skills_paths_get",
    args: { cliKey },
    invoke: () =>
      commands.skillsPathsGet(cliKey) as Promise<GeneratedCommandResult<SkillsPaths>>,
  });
}

export async function skillCheckUpdates(workspaceId: number) {
  return invokeGeneratedIpc<SkillUpdateInfo[]>({
    title: "检查技能更新失败",
    cmd: "skill_check_updates",
    args: { workspaceId },
    invoke: () =>
      commands.skillCheckUpdates(workspaceId) as Promise<
        GeneratedCommandResult<SkillUpdateInfo[]>
      >,
  });
}

export async function skillUpdate(input: { workspace_id: number; skill_id: number }) {
  return invokeGeneratedIpc<InstalledSkillSummary>({
    title: "更新技能失败",
    cmd: "skill_update",
    args: {
      workspaceId: input.workspace_id,
      skillId: input.skill_id,
    },
    invoke: () =>
      commands.skillUpdate(input.workspace_id, input.skill_id) as Promise<
        GeneratedCommandResult<InstalledSkillSummary>
      >,
  });
}
