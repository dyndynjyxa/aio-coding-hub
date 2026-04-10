import { invokeService } from "../invokeServiceCommand";

export type ClaudeModelValidationRunRow = {
  id: number;
  provider_id: number;
  created_at: number;
  request_json: string;
  result_json: string;
};

export async function claudeValidationHistoryList(input: { provider_id: number; limit?: number }) {
  return invokeService<ClaudeModelValidationRunRow[]>(
    "读取 Claude 模型验证历史失败",
    "claude_validation_history_list",
    {
      providerId: input.provider_id,
      limit: input.limit,
    }
  );
}

export async function claudeValidationHistoryClearProvider(input: { provider_id: number }) {
  return invokeService<boolean>(
    "清空 Claude 模型验证历史失败",
    "claude_validation_history_clear_provider",
    {
      providerId: input.provider_id,
    }
  );
}
