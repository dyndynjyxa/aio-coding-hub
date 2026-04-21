import { commands } from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";
import type { CliKey } from "../providers/providers";

export type WorkspaceSummary = {
  id: number;
  cli_key: CliKey;
  name: string;
  created_at: number;
  updated_at: number;
};

export type WorkspacesListResult = {
  active_id: number | null;
  items: WorkspaceSummary[];
};

export async function workspacesList(cliKey: CliKey) {
  return invokeGeneratedIpc<WorkspacesListResult>({
    title: "读取工作区列表失败",
    cmd: "workspaces_list",
    args: { cliKey },
    invoke: () =>
      commands.workspacesList(cliKey) as Promise<GeneratedCommandResult<WorkspacesListResult>>,
  });
}

export async function workspaceCreate(input: {
  cli_key: CliKey;
  name: string;
  clone_from_active?: boolean;
}) {
  return invokeGeneratedIpc<WorkspaceSummary>({
    title: "创建工作区失败",
    cmd: "workspace_create",
    args: {
      cliKey: input.cli_key,
      name: input.name,
      cloneFromActive: input.clone_from_active ?? false,
    },
    invoke: () =>
      commands.workspaceCreate(
        input.cli_key,
        input.name,
        input.clone_from_active ?? false
      ) as Promise<GeneratedCommandResult<WorkspaceSummary>>,
  });
}

export async function workspaceRename(input: { workspace_id: number; name: string }) {
  return invokeGeneratedIpc<WorkspaceSummary>({
    title: "重命名工作区失败",
    cmd: "workspace_rename",
    args: {
      workspaceId: input.workspace_id,
      name: input.name,
    },
    invoke: () =>
      commands.workspaceRename(input.workspace_id, input.name) as Promise<
        GeneratedCommandResult<WorkspaceSummary>
      >,
  });
}

export async function workspaceDelete(workspaceId: number) {
  return invokeGeneratedIpc<boolean>({
    title: "删除工作区失败",
    cmd: "workspace_delete",
    args: { workspaceId },
    invoke: () =>
      commands.workspaceDelete(workspaceId) as Promise<GeneratedCommandResult<boolean>>,
  });
}

export type WorkspacePreview = {
  cli_key: CliKey;
  from_workspace_id: number | null;
  to_workspace_id: number;
  prompts: {
    from_enabled: { name: string; excerpt: string } | null;
    to_enabled: { name: string; excerpt: string } | null;
    will_change: boolean;
  };
  mcp: {
    from_enabled: string[];
    to_enabled: string[];
    added: string[];
    removed: string[];
  };
  skills: {
    from_enabled: string[];
    to_enabled: string[];
    added: string[];
    removed: string[];
  };
};

export type WorkspaceApplyReport = {
  cli_key: CliKey;
  from_workspace_id: number | null;
  to_workspace_id: number;
  applied_at: number;
};

export async function workspacePreview(workspaceId: number) {
  return invokeGeneratedIpc<WorkspacePreview>({
    title: "读取工作区预览失败",
    cmd: "workspace_preview",
    args: { workspaceId },
    invoke: () =>
      commands.workspacePreview(workspaceId) as Promise<GeneratedCommandResult<WorkspacePreview>>,
  });
}

export async function workspaceApply(workspaceId: number) {
  return invokeGeneratedIpc<WorkspaceApplyReport>({
    title: "应用工作区失败",
    cmd: "workspace_apply",
    args: { workspaceId },
    invoke: () =>
      commands.workspaceApply(workspaceId) as Promise<
        GeneratedCommandResult<WorkspaceApplyReport>
      >,
  });
}
