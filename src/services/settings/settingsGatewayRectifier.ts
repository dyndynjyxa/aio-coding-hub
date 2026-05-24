import { commands } from "../../generated/bindings";
import type { AppSettings } from "./settings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";
import {
  normalizeBooleanSetting,
  normalizePositiveSafeIntegerSetting,
} from "./settingsPrimitiveValidation";

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

const RESPONSE_FIXER_MAX_JSON_DEPTH_MAX = 2000;
const RESPONSE_FIXER_MAX_FIX_SIZE_MAX = 16 * 1024 * 1024;

export function normalizeGatewayRectifierSettingsPatch(
  input: GatewayRectifierSettingsPatch
): GatewayRectifierSettingsPatch {
  return {
    verbose_provider_error: normalizeBooleanSetting(
      input.verbose_provider_error,
      "verbose_provider_error"
    ),
    intercept_anthropic_warmup_requests: normalizeBooleanSetting(
      input.intercept_anthropic_warmup_requests,
      "intercept_anthropic_warmup_requests"
    ),
    enable_thinking_signature_rectifier: normalizeBooleanSetting(
      input.enable_thinking_signature_rectifier,
      "enable_thinking_signature_rectifier"
    ),
    enable_thinking_budget_rectifier: normalizeBooleanSetting(
      input.enable_thinking_budget_rectifier,
      "enable_thinking_budget_rectifier"
    ),
    enable_billing_header_rectifier: normalizeBooleanSetting(
      input.enable_billing_header_rectifier,
      "enable_billing_header_rectifier"
    ),
    enable_claude_metadata_user_id_injection: normalizeBooleanSetting(
      input.enable_claude_metadata_user_id_injection,
      "enable_claude_metadata_user_id_injection"
    ),
    enable_response_fixer: normalizeBooleanSetting(
      input.enable_response_fixer,
      "enable_response_fixer"
    ),
    response_fixer_fix_encoding: normalizeBooleanSetting(
      input.response_fixer_fix_encoding,
      "response_fixer_fix_encoding"
    ),
    response_fixer_fix_sse_format: normalizeBooleanSetting(
      input.response_fixer_fix_sse_format,
      "response_fixer_fix_sse_format"
    ),
    response_fixer_fix_truncated_json: normalizeBooleanSetting(
      input.response_fixer_fix_truncated_json,
      "response_fixer_fix_truncated_json"
    ),
    response_fixer_max_json_depth: normalizePositiveSafeIntegerSetting(
      input.response_fixer_max_json_depth,
      "response_fixer_max_json_depth",
      RESPONSE_FIXER_MAX_JSON_DEPTH_MAX
    ),
    response_fixer_max_fix_size: normalizePositiveSafeIntegerSetting(
      input.response_fixer_max_fix_size,
      "response_fixer_max_fix_size",
      RESPONSE_FIXER_MAX_FIX_SIZE_MAX
    ),
  };
}

export async function settingsGatewayRectifierSet(input: GatewayRectifierSettingsPatch) {
  const normalizedInput = normalizeGatewayRectifierSettingsPatch(input);
  const update = {
    verboseProviderError: normalizedInput.verbose_provider_error,
    interceptAnthropicWarmupRequests: normalizedInput.intercept_anthropic_warmup_requests,
    enableThinkingSignatureRectifier: normalizedInput.enable_thinking_signature_rectifier,
    enableThinkingBudgetRectifier: normalizedInput.enable_thinking_budget_rectifier,
    enableBillingHeaderRectifier: normalizedInput.enable_billing_header_rectifier,
    enableClaudeMetadataUserIdInjection: normalizedInput.enable_claude_metadata_user_id_injection,
    enableResponseFixer: normalizedInput.enable_response_fixer,
    responseFixerFixEncoding: normalizedInput.response_fixer_fix_encoding,
    responseFixerFixSseFormat: normalizedInput.response_fixer_fix_sse_format,
    responseFixerFixTruncatedJson: normalizedInput.response_fixer_fix_truncated_json,
    responseFixerMaxJsonDepth: normalizedInput.response_fixer_max_json_depth,
    responseFixerMaxFixSize: normalizedInput.response_fixer_max_fix_size,
  };

  return invokeGeneratedIpc<AppSettings>({
    title: "保存网关修复配置失败",
    cmd: "settings_gateway_rectifier_set",
    args: { update },
    invoke: () =>
      commands.settingsGatewayRectifierSet(update) as Promise<GeneratedCommandResult<AppSettings>>,
  });
}
