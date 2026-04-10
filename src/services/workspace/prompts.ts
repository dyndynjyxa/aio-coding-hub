import { invokeService } from "../invokeServiceCommand";
import type { CliKey } from "../providers/providers";

export type PromptSummary = {
  id: number;
  workspace_id: number;
  cli_key: CliKey;
  name: string;
  content: string;
  enabled: boolean;
  created_at: number;
  updated_at: number;
};

export type DefaultPromptSyncItem = {
  cli_key: CliKey;
  action: "created" | "updated" | "unchanged" | "skipped" | "error";
  message: string | null;
};

export type DefaultPromptSyncReport = {
  items: DefaultPromptSyncItem[];
};

export async function promptsList(workspaceId: number) {
  return invokeService<PromptSummary[]>("读取提示词列表失败", "prompts_list", { workspaceId });
}

export async function promptsDefaultSyncFromFiles() {
  return invokeService<DefaultPromptSyncReport>(
    "同步默认提示词失败",
    "prompts_default_sync_from_files"
  );
}

export async function promptUpsert(input: {
  prompt_id?: number | null;
  workspace_id: number;
  name: string;
  content: string;
  enabled: boolean;
}) {
  return invokeService<PromptSummary>("保存提示词失败", "prompt_upsert", {
    promptId: input.prompt_id ?? null,
    workspaceId: input.workspace_id,
    name: input.name,
    content: input.content,
    enabled: input.enabled,
  });
}

export async function promptSetEnabled(promptId: number, enabled: boolean) {
  return invokeService<PromptSummary>("更新提示词启用状态失败", "prompt_set_enabled", {
    promptId,
    enabled,
  });
}

export async function promptDelete(promptId: number) {
  return invokeService<boolean>("删除提示词失败", "prompt_delete", { promptId });
}
