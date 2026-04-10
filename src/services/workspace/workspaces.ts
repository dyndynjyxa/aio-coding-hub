import { invokeService } from "../invokeServiceCommand";
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
  return invokeService<WorkspacesListResult>("读取工作区列表失败", "workspaces_list", { cliKey });
}

export async function workspaceCreate(input: {
  cli_key: CliKey;
  name: string;
  clone_from_active?: boolean;
}) {
  return invokeService<WorkspaceSummary>("创建工作区失败", "workspace_create", {
    cliKey: input.cli_key,
    name: input.name,
    cloneFromActive: input.clone_from_active ?? false,
  });
}

export async function workspaceRename(input: { workspace_id: number; name: string }) {
  return invokeService<WorkspaceSummary>("重命名工作区失败", "workspace_rename", {
    workspaceId: input.workspace_id,
    name: input.name,
  });
}

export async function workspaceDelete(workspaceId: number) {
  return invokeService<boolean>("删除工作区失败", "workspace_delete", { workspaceId });
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
  return invokeService<WorkspacePreview>("读取工作区预览失败", "workspace_preview", {
    workspaceId,
  });
}

export async function workspaceApply(workspaceId: number) {
  return invokeService<WorkspaceApplyReport>("应用工作区失败", "workspace_apply", {
    workspaceId,
  });
}
