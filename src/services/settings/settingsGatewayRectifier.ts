import { invokeService } from "../invokeServiceCommand";
import type { AppSettings } from "./settings";

export type GatewayRectifierSettingsPatch = {
  verbose_provider_error: boolean;
  intercept_anthropic_warmup_requests: boolean;
  enable_thinking_signature_rectifier: boolean;
  enable_thinking_budget_rectifier: boolean;
  enable_billing_header_rectifier: boolean;
  enable_claude_metadata_user_id_injection: boolean;
  enable_response_fixer: boolean;
  response_fixer_fix_encoding: boolean;
  response_fixer_fix_sse_format: boolean;
  response_fixer_fix_truncated_json: boolean;
  response_fixer_max_json_depth: number;
  response_fixer_max_fix_size: number;
};

export async function settingsGatewayRectifierSet(input: GatewayRectifierSettingsPatch) {
  return invokeService<AppSettings>("保存网关修复配置失败", "settings_gateway_rectifier_set", {
    verboseProviderError: input.verbose_provider_error,
    interceptAnthropicWarmupRequests: input.intercept_anthropic_warmup_requests,
    enableThinkingSignatureRectifier: input.enable_thinking_signature_rectifier,
    enableThinkingBudgetRectifier: input.enable_thinking_budget_rectifier,
    enableBillingHeaderRectifier: input.enable_billing_header_rectifier,
    enableClaudeMetadataUserIdInjection: input.enable_claude_metadata_user_id_injection,
    enableResponseFixer: input.enable_response_fixer,
    responseFixerFixEncoding: input.response_fixer_fix_encoding,
    responseFixerFixSseFormat: input.response_fixer_fix_sse_format,
    responseFixerFixTruncatedJson: input.response_fixer_fix_truncated_json,
    responseFixerMaxJsonDepth: input.response_fixer_max_json_depth,
    responseFixerMaxFixSize: input.response_fixer_max_fix_size,
  });
}
