import {
  commands,
  type DefaultPromptSyncItem as GeneratedDefaultPromptSyncItem,
  type DefaultPromptSyncReport as GeneratedDefaultPromptSyncReport,
  type PromptSummary as GeneratedPromptSummary,
} from "../../generated/bindings";
import {
  invokeGeneratedIpc,
  mapGeneratedCommandResponse,
  type GeneratedCommandResult,
} from "../generatedIpc";
import type { CliKey } from "../providers/providers";
import { narrowGeneratedStringUnion, type Override } from "../generatedTypeUtils";

const CLI_KEY_VALUES = ["claude", "codex", "gemini"] as const satisfies readonly CliKey[];
const DEFAULT_PROMPT_SYNC_ACTION_VALUES = [
  "created",
  "updated",
  "unchanged",
  "skipped",
  "error",
] as const;

export type PromptSummary = Override<
  GeneratedPromptSummary,
  {
    cli_key: CliKey;
  }
>;

export type DefaultPromptSyncAction = (typeof DEFAULT_PROMPT_SYNC_ACTION_VALUES)[number];

export type DefaultPromptSyncItem = Override<
  GeneratedDefaultPromptSyncItem,
  {
    cli_key: CliKey;
    action: DefaultPromptSyncAction;
  }
>;

export type DefaultPromptSyncReport = Override<
  GeneratedDefaultPromptSyncReport,
  {
    items: DefaultPromptSyncItem[];
  }
>;

export type PromptUpsertInput = {
  promptId?: number | null;
  workspaceId: number;
  name: string;
  content: string;
  enabled: boolean;
};

function validatePositiveSafeInteger(label: string, value: number): number {
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new Error(`SEC_INVALID_INPUT: invalid ${label}=${value}`);
  }
  return value;
}

export function validatePromptWorkspaceId(workspaceId: number): number {
  return validatePositiveSafeInteger("workspaceId", workspaceId);
}

export function validatePromptId(promptId: number): number {
  return validatePositiveSafeInteger("promptId", promptId);
}

function normalizeOptionalPromptId(promptId: number | null | undefined): number | null {
  if (promptId == null) return null;
  return validatePromptId(promptId);
}

function normalizePromptName(name: string): string {
  const normalized = name.trim();
  if (!normalized) {
    throw new Error("SEC_INVALID_INPUT: prompt name is required");
  }
  return normalized;
}

function normalizePromptContent(content: string): string {
  return content.trim();
}

function toCliKey(value: string, label: string): CliKey {
  return narrowGeneratedStringUnion(value, CLI_KEY_VALUES, label);
}

function toDefaultPromptSyncAction(value: string, label: string): DefaultPromptSyncAction {
  return narrowGeneratedStringUnion(value, DEFAULT_PROMPT_SYNC_ACTION_VALUES, label);
}

function toPromptSummary(value: GeneratedPromptSummary): PromptSummary {
  return {
    ...value,
    cli_key: toCliKey(value.cli_key, "prompts_list.cli_key"),
  };
}

function toDefaultPromptSyncItem(value: GeneratedDefaultPromptSyncItem): DefaultPromptSyncItem {
  return {
    ...value,
    cli_key: toCliKey(value.cli_key, "prompts_default_sync_from_files.cli_key"),
    action: toDefaultPromptSyncAction(value.action, "prompts_default_sync_from_files.action"),
  };
}

function toDefaultPromptSyncReport(
  value: GeneratedDefaultPromptSyncReport
): DefaultPromptSyncReport {
  return {
    ...value,
    items: value.items.map(toDefaultPromptSyncItem),
  };
}

export async function promptsList(workspaceId: number) {
  const normalizedWorkspaceId = validatePromptWorkspaceId(workspaceId);

  return invokeGeneratedIpc<PromptSummary[]>({
    title: "读取提示词列表失败",
    cmd: "prompts_list",
    args: { workspaceId: normalizedWorkspaceId },
    invoke: async () =>
      mapGeneratedCommandResponse(await commands.promptsList(normalizedWorkspaceId), (rows) =>
        rows.map(toPromptSummary)
      ),
  });
}

export async function promptsDefaultSyncFromFiles() {
  return invokeGeneratedIpc<DefaultPromptSyncReport>({
    title: "同步默认提示词失败",
    cmd: "prompts_default_sync_from_files",
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.promptsDefaultSyncFromFiles(),
        toDefaultPromptSyncReport
      ),
  });
}

export async function promptUpsert(input: PromptUpsertInput) {
  const promptId = normalizeOptionalPromptId(input.promptId);
  const workspaceId = validatePromptWorkspaceId(input.workspaceId);
  const name = normalizePromptName(input.name);
  const content = normalizePromptContent(input.content);

  return invokeGeneratedIpc<PromptSummary>({
    title: "保存提示词失败",
    cmd: "prompt_upsert",
    args: {
      promptId,
      workspaceId,
      name,
      content,
      enabled: input.enabled,
    },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.promptUpsert(promptId, workspaceId, name, content, input.enabled),
        toPromptSummary
      ),
  });
}

export async function promptSetEnabled(promptId: number, enabled: boolean) {
  const normalizedPromptId = validatePromptId(promptId);

  return invokeGeneratedIpc<PromptSummary>({
    title: "更新提示词启用状态失败",
    cmd: "prompt_set_enabled",
    args: {
      promptId: normalizedPromptId,
      enabled,
    },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.promptSetEnabled(normalizedPromptId, enabled),
        toPromptSummary
      ),
  });
}

export async function promptDelete(promptId: number) {
  const normalizedPromptId = validatePromptId(promptId);

  return invokeGeneratedIpc<boolean>({
    title: "删除提示词失败",
    cmd: "prompt_delete",
    args: { promptId: normalizedPromptId },
    invoke: () =>
      commands.promptDelete(normalizedPromptId) as Promise<GeneratedCommandResult<boolean>>,
  });
}
