import { commands } from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";
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
  return invokeGeneratedIpc<PromptSummary[]>({
    title: "读取提示词列表失败",
    cmd: "prompts_list",
    args: { workspaceId },
    invoke: () =>
      commands.promptsList(workspaceId) as Promise<GeneratedCommandResult<PromptSummary[]>>,
  });
}

export async function promptsDefaultSyncFromFiles() {
  return invokeGeneratedIpc<DefaultPromptSyncReport>({
    title: "同步默认提示词失败",
    cmd: "prompts_default_sync_from_files",
    invoke: () =>
      commands.promptsDefaultSyncFromFiles() as Promise<
        GeneratedCommandResult<DefaultPromptSyncReport>
      >,
  });
}

export async function promptUpsert(input: {
  prompt_id?: number | null;
  workspace_id: number;
  name: string;
  content: string;
  enabled: boolean;
}) {
  return invokeGeneratedIpc<PromptSummary>({
    title: "保存提示词失败",
    cmd: "prompt_upsert",
    args: {
      promptId: input.prompt_id ?? null,
      workspaceId: input.workspace_id,
      name: input.name,
      content: input.content,
      enabled: input.enabled,
    },
    invoke: () =>
      commands.promptUpsert(
        input.prompt_id ?? null,
        input.workspace_id,
        input.name,
        input.content,
        input.enabled
      ) as Promise<GeneratedCommandResult<PromptSummary>>,
  });
}

export async function promptSetEnabled(promptId: number, enabled: boolean) {
  return invokeGeneratedIpc<PromptSummary>({
    title: "更新提示词启用状态失败",
    cmd: "prompt_set_enabled",
    args: {
      promptId,
      enabled,
    },
    invoke: () =>
      commands.promptSetEnabled(promptId, enabled) as Promise<
        GeneratedCommandResult<PromptSummary>
      >,
  });
}

export async function promptDelete(promptId: number) {
  return invokeGeneratedIpc<boolean>({
    title: "删除提示词失败",
    cmd: "prompt_delete",
    args: { promptId },
    invoke: () => commands.promptDelete(promptId) as Promise<GeneratedCommandResult<boolean>>,
  });
}
