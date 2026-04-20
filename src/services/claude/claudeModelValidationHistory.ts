import { commands } from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

export type ClaudeModelValidationRunRow = {
  id: number;
  provider_id: number;
  created_at: number;
  request_json: string;
  result_json: string;
};

export async function claudeValidationHistoryList(input: { provider_id: number; limit?: number }) {
  return invokeGeneratedIpc<ClaudeModelValidationRunRow[]>({
    title: "读取 Claude 模型验证历史失败",
    cmd: "claude_validation_history_list",
    args: {
      providerId: input.provider_id,
      limit: input.limit,
    },
    invoke: () =>
      commands.claudeValidationHistoryList(input.provider_id, input.limit ?? null) as Promise<
        GeneratedCommandResult<ClaudeModelValidationRunRow[]>
      >,
  });
}

export async function claudeValidationHistoryClearProvider(input: { provider_id: number }) {
  return invokeGeneratedIpc<boolean>({
    title: "清空 Claude 模型验证历史失败",
    cmd: "claude_validation_history_clear_provider",
    args: {
      providerId: input.provider_id,
    },
    invoke: () =>
      commands.claudeValidationHistoryClearProvider(input.provider_id) as Promise<
        GeneratedCommandResult<boolean>
      >,
  });
}
