import { invokeService } from "../invokeServiceCommand";
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

export async function skillReposList() {
  return invokeService<SkillRepoSummary[]>("读取技能仓库列表失败", "skill_repos_list");
}

export async function skillRepoUpsert(input: {
  repo_id?: number | null;
  git_url: string;
  branch: string;
  enabled: boolean;
}) {
  return invokeService<SkillRepoSummary>("保存技能仓库失败", "skill_repo_upsert", {
    repoId: input.repo_id ?? null,
    gitUrl: input.git_url,
    branch: input.branch,
    enabled: input.enabled,
  });
}

export async function skillRepoDelete(repoId: number) {
  return invokeService<boolean>("删除技能仓库失败", "skill_repo_delete", { repoId });
}

export async function skillsInstalledList(workspaceId: number) {
  return invokeService<InstalledSkillSummary[]>("读取已安装技能失败", "skills_installed_list", {
    workspaceId,
  });
}

export async function skillsDiscoverAvailable(refresh: boolean) {
  return invokeService<AvailableSkillSummary[]>("发现可用技能失败", "skills_discover_available", {
    refresh,
  });
}

export async function skillInstall(input: {
  workspace_id: number;
  git_url: string;
  branch: string;
  source_subdir: string;
  enabled: boolean;
}) {
  return invokeService<InstalledSkillSummary>("安装技能失败", "skill_install", {
    workspaceId: input.workspace_id,
    gitUrl: input.git_url,
    branch: input.branch,
    sourceSubdir: input.source_subdir,
    enabled: input.enabled,
  });
}

export async function skillInstallToLocal(input: {
  workspace_id: number;
  git_url: string;
  branch: string;
  source_subdir: string;
}) {
  return invokeService<LocalSkillSummary>("安装到当前 CLI 失败", "skill_install_to_local", {
    workspaceId: input.workspace_id,
    gitUrl: input.git_url,
    branch: input.branch,
    sourceSubdir: input.source_subdir,
  });
}

export async function skillSetEnabled(input: {
  workspace_id: number;
  skill_id: number;
  enabled: boolean;
}) {
  return invokeService<InstalledSkillSummary>("更新技能启用状态失败", "skill_set_enabled", {
    workspaceId: input.workspace_id,
    skillId: input.skill_id,
    enabled: input.enabled,
  });
}

export async function skillUninstall(skillId: number) {
  return invokeService<boolean>("卸载技能失败", "skill_uninstall", { skillId });
}

export async function skillReturnToLocal(input: { workspace_id: number; skill_id: number }) {
  return invokeService<boolean>("返回本机技能失败", "skill_return_to_local", {
    workspaceId: input.workspace_id,
    skillId: input.skill_id,
  });
}

export async function skillsLocalList(workspaceId: number) {
  return invokeService<LocalSkillSummary[]>("读取本地技能列表失败", "skills_local_list", {
    workspaceId,
  });
}

export async function skillLocalDelete(input: { workspace_id: number; dir_name: string }) {
  return invokeService<boolean>("删除本地技能失败", "skill_local_delete", {
    workspaceId: input.workspace_id,
    dirName: input.dir_name,
  });
}

export async function skillImportLocal(input: { workspace_id: number; dir_name: string }) {
  return invokeService<InstalledSkillSummary>("导入本地技能失败", "skill_import_local", {
    workspaceId: input.workspace_id,
    dirName: input.dir_name,
  });
}

export async function skillsImportLocalBatch(input: { workspace_id: number; dir_names: string[] }) {
  return invokeService<SkillImportLocalBatchReport>(
    "批量导入本地技能失败",
    "skills_import_local_batch",
    {
      workspaceId: input.workspace_id,
      dirNames: input.dir_names,
    }
  );
}

export async function skillsPathsGet(cliKey: CliKey) {
  return invokeService<SkillsPaths>("读取技能路径失败", "skills_paths_get", { cliKey });
}
