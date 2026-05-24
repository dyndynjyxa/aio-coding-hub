import {
  commands,
  type WorkspaceApplyReport as GeneratedWorkspaceApplyReport,
  type WorkspacePreview as GeneratedWorkspacePreview,
  type WorkspaceSummary as GeneratedWorkspaceSummary,
  type WorkspacesListResult as GeneratedWorkspacesListResult,
} from "../../generated/bindings";
import {
  invokeGeneratedIpc,
  mapGeneratedCommandResponse,
  type GeneratedCommandResult,
} from "../generatedIpc";
import type { CliKey } from "../providers/providers";
import { narrowGeneratedStringUnion, type Override } from "../generatedTypeUtils";

const CLI_KEY_VALUES = ["claude", "codex", "gemini"] as const satisfies readonly CliKey[];
export const MAX_WORKSPACE_NAME_CHARS = 128;

export type WorkspaceSummary = Override<
  GeneratedWorkspaceSummary,
  {
    cli_key: CliKey;
  }
>;

export type WorkspacesListResult = Override<
  GeneratedWorkspacesListResult,
  {
    items: WorkspaceSummary[];
  }
>;

export type WorkspaceCreateInput = {
  cliKey: CliKey;
  name: string;
  cloneFromActive?: boolean;
};

export type WorkspaceRenameInput = {
  workspaceId: number;
  name: string;
};

export type WorkspacePreview = Override<
  GeneratedWorkspacePreview,
  {
    cli_key: CliKey;
  }
>;

export type WorkspaceApplyReport = Override<
  GeneratedWorkspaceApplyReport,
  {
    cli_key: CliKey;
  }
>;

function validatePositiveSafeInteger(label: string, value: number): number {
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new Error(`SEC_INVALID_INPUT: invalid ${label}=${value}`);
  }
  return value;
}

export function validateWorkspaceId(workspaceId: number): number {
  return validatePositiveSafeInteger("workspaceId", workspaceId);
}

export function validateWorkspaceCliKey(cliKey: string): CliKey {
  const normalizedCliKey = cliKey.trim();
  if ((CLI_KEY_VALUES as readonly string[]).includes(normalizedCliKey)) {
    return normalizedCliKey as CliKey;
  }
  throw new Error(`SEC_INVALID_INPUT: invalid cliKey=${cliKey}`);
}

export function normalizeWorkspaceName(name: string): string {
  const normalized = name.trim();
  if (!normalized) {
    throw new Error("SEC_INVALID_INPUT: workspace name is required");
  }
  if (/[\u0000-\u001f\u007f]/u.test(normalized)) {
    throw new Error("SEC_INVALID_INPUT: workspace name contains control characters");
  }
  if ([...normalized].length > MAX_WORKSPACE_NAME_CHARS) {
    throw new Error(
      `SEC_INVALID_INPUT: workspace name is too long (max ${MAX_WORKSPACE_NAME_CHARS} chars)`
    );
  }
  return normalized;
}

function toCliKey(value: string, label: string): CliKey {
  return narrowGeneratedStringUnion(value, CLI_KEY_VALUES, label);
}

function toWorkspaceSummary(value: GeneratedWorkspaceSummary): WorkspaceSummary {
  return {
    ...value,
    cli_key: toCliKey(value.cli_key, "workspaces.cli_key"),
  };
}

function toWorkspacesListResult(value: GeneratedWorkspacesListResult): WorkspacesListResult {
  return {
    ...value,
    items: value.items.map(toWorkspaceSummary),
  };
}

function toWorkspacePreview(value: GeneratedWorkspacePreview): WorkspacePreview {
  return {
    ...value,
    cli_key: toCliKey(value.cli_key, "workspace_preview.cli_key"),
  };
}

function toWorkspaceApplyReport(value: GeneratedWorkspaceApplyReport): WorkspaceApplyReport {
  return {
    ...value,
    cli_key: toCliKey(value.cli_key, "workspace_apply.cli_key"),
  };
}

export async function workspacesList(cliKey: CliKey) {
  const normalizedCliKey = validateWorkspaceCliKey(cliKey);

  return invokeGeneratedIpc<WorkspacesListResult>({
    title: "读取工作区列表失败",
    cmd: "workspaces_list",
    args: { cliKey: normalizedCliKey },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.workspacesList(normalizedCliKey),
        toWorkspacesListResult
      ),
  });
}

export async function workspaceCreate(input: WorkspaceCreateInput) {
  const cliKey = validateWorkspaceCliKey(input.cliKey);
  const name = normalizeWorkspaceName(input.name);
  const cloneFromActive = input.cloneFromActive ?? false;

  return invokeGeneratedIpc<WorkspaceSummary>({
    title: "创建工作区失败",
    cmd: "workspace_create",
    args: {
      cliKey,
      name,
      cloneFromActive,
    },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.workspaceCreate(cliKey, name, cloneFromActive),
        toWorkspaceSummary
      ),
  });
}

export async function workspaceRename(input: WorkspaceRenameInput) {
  const workspaceId = validateWorkspaceId(input.workspaceId);
  const name = normalizeWorkspaceName(input.name);

  return invokeGeneratedIpc<WorkspaceSummary>({
    title: "重命名工作区失败",
    cmd: "workspace_rename",
    args: {
      workspaceId,
      name,
    },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.workspaceRename(workspaceId, name),
        toWorkspaceSummary
      ),
  });
}

export async function workspaceDelete(workspaceId: number) {
  const normalizedWorkspaceId = validateWorkspaceId(workspaceId);

  return invokeGeneratedIpc<boolean>({
    title: "删除工作区失败",
    cmd: "workspace_delete",
    args: { workspaceId: normalizedWorkspaceId },
    invoke: () =>
      commands.workspaceDelete(normalizedWorkspaceId) as Promise<GeneratedCommandResult<boolean>>,
  });
}

export async function workspacePreview(workspaceId: number) {
  const normalizedWorkspaceId = validateWorkspaceId(workspaceId);

  return invokeGeneratedIpc<WorkspacePreview>({
    title: "读取工作区预览失败",
    cmd: "workspace_preview",
    args: { workspaceId: normalizedWorkspaceId },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.workspacePreview(normalizedWorkspaceId),
        toWorkspacePreview
      ),
  });
}

export async function workspaceApply(workspaceId: number) {
  const normalizedWorkspaceId = validateWorkspaceId(workspaceId);

  return invokeGeneratedIpc<WorkspaceApplyReport>({
    title: "应用工作区失败",
    cmd: "workspace_apply",
    args: { workspaceId: normalizedWorkspaceId },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.workspaceApply(normalizedWorkspaceId),
        toWorkspaceApplyReport
      ),
  });
}
