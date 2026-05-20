import {
  commands,
  type AvailableSkillSummary as GeneratedAvailableSkillSummary,
  type InstalledSkillSummary as GeneratedInstalledSkillSummary,
  type LocalSkillSummary as GeneratedLocalSkillSummary,
  type SkillImportIssue as GeneratedSkillImportIssue,
  type SkillImportLocalBatchReport as GeneratedSkillImportLocalBatchReport,
  type SkillRepoSummary as GeneratedSkillRepoSummary,
  type SkillsPaths as GeneratedSkillsPaths,
  type SkillUpdateInfo as GeneratedSkillUpdateInfo,
} from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";
import { createRiskyIpcConfirm } from "../ipcConfirm";
import type { CliKey } from "../providers/providers";

const CLI_KEY_VALUES = ["claude", "codex", "gemini"] as const satisfies readonly CliKey[];

export type SkillRepoSummary = GeneratedSkillRepoSummary;
export type InstalledSkillSummary = GeneratedInstalledSkillSummary;
export type AvailableSkillSummary = GeneratedAvailableSkillSummary;
export type SkillsPaths = GeneratedSkillsPaths;
export type LocalSkillSummary = GeneratedLocalSkillSummary;
export type SkillImportIssue = GeneratedSkillImportIssue;
export type SkillImportLocalBatchReport = GeneratedSkillImportLocalBatchReport;
export type SkillUpdateInfo = GeneratedSkillUpdateInfo;

export type SkillRepoUpsertInput = {
  repoId?: number | null;
  gitUrl: string;
  branch: string;
  enabled: boolean;
};

export type SkillInstallInput = {
  workspaceId: number;
  gitUrl: string;
  branch: string;
  sourceSubdir: string;
  enabled: boolean;
};

export type SkillInstallToLocalInput = {
  workspaceId: number;
  gitUrl: string;
  branch: string;
  sourceSubdir: string;
};

export type SkillSetEnabledInput = {
  workspaceId: number;
  skillId: number;
  enabled: boolean;
};

export type SkillReturnToLocalInput = {
  workspaceId: number;
  skillId: number;
};

export type SkillLocalDeleteInput = {
  workspaceId: number;
  dirName: string;
};

export type SkillImportLocalInput = {
  workspaceId: number;
  dirName: string;
};

export type SkillsImportLocalBatchInput = {
  workspaceId: number;
  dirNames: string[];
};

export type SkillUpdateInput = {
  workspaceId: number;
  skillId: number;
};

export const SKILLS_IMPORT_LOCAL_MAX_DIR_NAMES = 512;

function validatePositiveSafeInteger(label: string, value: number): number {
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new Error(`SEC_INVALID_INPUT: invalid ${label}=${value}`);
  }
  return value;
}

export function validateSkillsWorkspaceId(workspaceId: number): number {
  return validatePositiveSafeInteger("workspaceId", workspaceId);
}

export function validateSkillId(skillId: number): number {
  return validatePositiveSafeInteger("skillId", skillId);
}

export function validateSkillRepoId(repoId: number): number {
  return validatePositiveSafeInteger("repoId", repoId);
}

export function validateSkillsCliKey(cliKey: string): CliKey {
  const normalizedCliKey = cliKey.trim();
  if ((CLI_KEY_VALUES as readonly string[]).includes(normalizedCliKey)) {
    return normalizedCliKey as CliKey;
  }
  throw new Error(`SEC_INVALID_INPUT: invalid cliKey=${cliKey}`);
}

function normalizeOptionalSkillRepoId(repoId: number | null | undefined): number | null {
  if (repoId == null) return null;
  return validateSkillRepoId(repoId);
}

function normalizeRequiredText(value: string, label: string): string {
  const normalized = value.trim();
  if (!normalized) {
    throw new Error(`SEC_INVALID_INPUT: ${label} is required`);
  }
  return normalized;
}

function normalizeSkillSourceSubdir(sourceSubdir: string): string {
  const normalized = normalizeRequiredText(sourceSubdir, "sourceSubdir");
  if (
    normalized.startsWith("/") ||
    /^[A-Za-z]:[\\/]/.test(normalized) ||
    normalized.split(/[\\/]+/).includes("..")
  ) {
    throw new Error("SEC_INVALID_INPUT: sourceSubdir must be a relative subdir");
  }
  return normalized;
}

export function normalizeSkillLocalDirName(dirName: string): string {
  const normalized = normalizeRequiredText(dirName, "dirName");
  if (
    normalized === "." ||
    normalized === ".." ||
    normalized.includes("/") ||
    normalized.includes("\\")
  ) {
    throw new Error("SEC_INVALID_INPUT: dirName must be a single directory name");
  }
  return normalized;
}

export function normalizeSkillsLocalDirNames(dirNames: readonly string[]): string[] {
  if (!Array.isArray(dirNames)) {
    throw new Error("SEC_INVALID_INPUT: dirNames must be an array");
  }
  if (dirNames.length === 0) {
    throw new Error("SEC_INVALID_INPUT: dirNames is required");
  }
  if (dirNames.length > SKILLS_IMPORT_LOCAL_MAX_DIR_NAMES) {
    throw new Error(
      `SEC_INVALID_INPUT: dirNames must contain at most ${SKILLS_IMPORT_LOCAL_MAX_DIR_NAMES} entries`
    );
  }

  const normalized: string[] = [];
  const seen = new Set<string>();
  for (const rawDirName of dirNames) {
    const trimmed = rawDirName.trim();
    if (!trimmed) continue;
    const dirName = normalizeSkillLocalDirName(trimmed);
    if (seen.has(dirName)) continue;
    seen.add(dirName);
    normalized.push(dirName);
  }

  if (normalized.length === 0) {
    throw new Error("SEC_INVALID_INPUT: dirNames is required");
  }

  return normalized;
}

export async function skillReposList() {
  return invokeGeneratedIpc<SkillRepoSummary[]>({
    title: "读取技能仓库列表失败",
    cmd: "skill_repos_list",
    invoke: () => commands.skillReposList() as Promise<GeneratedCommandResult<SkillRepoSummary[]>>,
  });
}

export async function skillRepoUpsert(input: SkillRepoUpsertInput) {
  const repoId = normalizeOptionalSkillRepoId(input.repoId);
  const gitUrl = normalizeRequiredText(input.gitUrl, "gitUrl");
  const branch = normalizeRequiredText(input.branch, "branch");

  return invokeGeneratedIpc<SkillRepoSummary>({
    title: "保存技能仓库失败",
    cmd: "skill_repo_upsert",
    args: {
      repoId,
      gitUrl,
      branch,
      enabled: input.enabled,
    },
    invoke: () =>
      commands.skillRepoUpsert(repoId, gitUrl, branch, input.enabled) as Promise<
        GeneratedCommandResult<SkillRepoSummary>
      >,
  });
}

export async function skillRepoDelete(repoId: number) {
  const normalizedRepoId = validateSkillRepoId(repoId);

  return invokeGeneratedIpc<boolean>({
    title: "删除技能仓库失败",
    cmd: "skill_repo_delete",
    args: { repoId: normalizedRepoId },
    invoke: () =>
      commands.skillRepoDelete(normalizedRepoId) as Promise<GeneratedCommandResult<boolean>>,
  });
}

export async function skillsInstalledList(workspaceId: number) {
  const normalizedWorkspaceId = validateSkillsWorkspaceId(workspaceId);

  return invokeGeneratedIpc<InstalledSkillSummary[]>({
    title: "读取已安装技能失败",
    cmd: "skills_installed_list",
    args: { workspaceId: normalizedWorkspaceId },
    invoke: () =>
      commands.skillsInstalledList(normalizedWorkspaceId) as Promise<
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

export async function skillInstall(input: SkillInstallInput) {
  const workspaceId = validateSkillsWorkspaceId(input.workspaceId);
  const gitUrl = normalizeRequiredText(input.gitUrl, "gitUrl");
  const branch = normalizeRequiredText(input.branch, "branch");
  const sourceSubdir = normalizeSkillSourceSubdir(input.sourceSubdir);

  return invokeGeneratedIpc<InstalledSkillSummary>({
    title: "安装技能失败",
    cmd: "skill_install",
    args: {
      workspaceId,
      gitUrl,
      branch,
      sourceSubdir,
      enabled: input.enabled,
    },
    invoke: () =>
      commands.skillInstall(workspaceId, gitUrl, branch, sourceSubdir, input.enabled) as Promise<
        GeneratedCommandResult<InstalledSkillSummary>
      >,
  });
}

export async function skillInstallToLocal(input: SkillInstallToLocalInput) {
  const workspaceId = validateSkillsWorkspaceId(input.workspaceId);
  const gitUrl = normalizeRequiredText(input.gitUrl, "gitUrl");
  const branch = normalizeRequiredText(input.branch, "branch");
  const sourceSubdir = normalizeSkillSourceSubdir(input.sourceSubdir);

  return invokeGeneratedIpc<LocalSkillSummary>({
    title: "安装到当前 CLI 失败",
    cmd: "skill_install_to_local",
    args: {
      workspaceId,
      gitUrl,
      branch,
      sourceSubdir,
    },
    invoke: () =>
      commands.skillInstallToLocal(workspaceId, gitUrl, branch, sourceSubdir) as Promise<
        GeneratedCommandResult<LocalSkillSummary>
      >,
  });
}

export async function skillSetEnabled(input: SkillSetEnabledInput) {
  const workspaceId = validateSkillsWorkspaceId(input.workspaceId);
  const skillId = validateSkillId(input.skillId);

  return invokeGeneratedIpc<InstalledSkillSummary>({
    title: "更新技能启用状态失败",
    cmd: "skill_set_enabled",
    args: {
      workspaceId,
      skillId,
      enabled: input.enabled,
    },
    invoke: () =>
      commands.skillSetEnabled(workspaceId, skillId, input.enabled) as Promise<
        GeneratedCommandResult<InstalledSkillSummary>
      >,
  });
}

export async function skillUninstall(skillId: number) {
  const normalizedSkillId = validateSkillId(skillId);

  return invokeGeneratedIpc<boolean>({
    title: "卸载技能失败",
    cmd: "skill_uninstall",
    args: { skillId: normalizedSkillId },
    invoke: () =>
      commands.skillUninstall(normalizedSkillId) as Promise<GeneratedCommandResult<boolean>>,
  });
}

export async function skillReturnToLocal(input: SkillReturnToLocalInput) {
  const workspaceId = validateSkillsWorkspaceId(input.workspaceId);
  const skillId = validateSkillId(input.skillId);

  return invokeGeneratedIpc<boolean>({
    title: "返回本机技能失败",
    cmd: "skill_return_to_local",
    args: {
      workspaceId,
      skillId,
    },
    invoke: () =>
      commands.skillReturnToLocal(workspaceId, skillId) as Promise<GeneratedCommandResult<boolean>>,
  });
}

export async function skillsLocalList(workspaceId: number) {
  const normalizedWorkspaceId = validateSkillsWorkspaceId(workspaceId);

  return invokeGeneratedIpc<LocalSkillSummary[]>({
    title: "读取本地技能列表失败",
    cmd: "skills_local_list",
    args: { workspaceId: normalizedWorkspaceId },
    invoke: () =>
      commands.skillsLocalList(normalizedWorkspaceId) as Promise<
        GeneratedCommandResult<LocalSkillSummary[]>
      >,
  });
}

export async function skillLocalDelete(input: SkillLocalDeleteInput) {
  const workspaceId = validateSkillsWorkspaceId(input.workspaceId);
  const dirName = normalizeSkillLocalDirName(input.dirName);
  const confirm = createRiskyIpcConfirm(
    "skill_local_delete",
    `workspace:${workspaceId}:skill-local:${dirName}`
  );
  return invokeGeneratedIpc<boolean>({
    title: "删除本地技能失败",
    cmd: "skill_local_delete",
    args: {
      workspaceId,
      dirName,
      confirm,
    },
    invoke: () =>
      commands.skillLocalDelete(workspaceId, dirName, confirm) as Promise<
        GeneratedCommandResult<boolean>
      >,
  });
}

export async function skillImportLocal(input: SkillImportLocalInput) {
  const workspaceId = validateSkillsWorkspaceId(input.workspaceId);
  const dirName = normalizeSkillLocalDirName(input.dirName);

  return invokeGeneratedIpc<InstalledSkillSummary>({
    title: "导入本地技能失败",
    cmd: "skill_import_local",
    args: {
      workspaceId,
      dirName,
    },
    invoke: () =>
      commands.skillImportLocal(workspaceId, dirName) as Promise<
        GeneratedCommandResult<InstalledSkillSummary>
      >,
  });
}

export async function skillsImportLocalBatch(input: SkillsImportLocalBatchInput) {
  const workspaceId = validateSkillsWorkspaceId(input.workspaceId);
  const dirNames = normalizeSkillsLocalDirNames(input.dirNames);

  return invokeGeneratedIpc<SkillImportLocalBatchReport>({
    title: "批量导入本地技能失败",
    cmd: "skills_import_local_batch",
    args: {
      workspaceId,
      dirNames,
    },
    invoke: () =>
      commands.skillsImportLocalBatch(workspaceId, dirNames) as Promise<
        GeneratedCommandResult<SkillImportLocalBatchReport>
      >,
  });
}

export async function skillsPathsGet(cliKey: CliKey) {
  const normalizedCliKey = validateSkillsCliKey(cliKey);

  return invokeGeneratedIpc<SkillsPaths>({
    title: "读取技能路径失败",
    cmd: "skills_paths_get",
    args: { cliKey: normalizedCliKey },
    invoke: () =>
      commands.skillsPathsGet(normalizedCliKey) as Promise<GeneratedCommandResult<SkillsPaths>>,
  });
}

export async function skillCheckUpdates(workspaceId: number) {
  const normalizedWorkspaceId = validateSkillsWorkspaceId(workspaceId);

  return invokeGeneratedIpc<SkillUpdateInfo[]>({
    title: "检查技能更新失败",
    cmd: "skill_check_updates",
    args: { workspaceId: normalizedWorkspaceId },
    invoke: () =>
      commands.skillCheckUpdates(normalizedWorkspaceId) as Promise<
        GeneratedCommandResult<SkillUpdateInfo[]>
      >,
  });
}

export async function skillUpdate(input: SkillUpdateInput) {
  const workspaceId = validateSkillsWorkspaceId(input.workspaceId);
  const skillId = validateSkillId(input.skillId);

  return invokeGeneratedIpc<InstalledSkillSummary>({
    title: "更新技能失败",
    cmd: "skill_update",
    args: {
      workspaceId,
      skillId,
    },
    invoke: () =>
      commands.skillUpdate(workspaceId, skillId) as Promise<
        GeneratedCommandResult<InstalledSkillSummary>
      >,
  });
}
