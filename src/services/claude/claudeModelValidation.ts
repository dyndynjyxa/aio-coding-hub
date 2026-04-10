import { invokeService } from "../invokeServiceCommand";

export type ClaudeModelValidationResult = {
  ok: boolean;
  provider_id: number;
  provider_name: string;
  base_url: string;
  target_url: string;
  status: number | null;
  duration_ms: number;
  requested_model: string | null;
  responded_model: string | null;
  stream: boolean;
  output_text_chars: number;
  output_text_preview: string;
  checks: unknown;
  signals: unknown;
  response_headers?: unknown;
  usage: unknown | null;
  error: string | null;
  raw_excerpt: string;
  request: unknown;
};

export async function claudeProviderValidateModel(input: {
  provider_id: number;
  base_url: string;
  request_json: string;
}) {
  return invokeService<ClaudeModelValidationResult>(
    "Claude 模型验证失败",
    "claude_provider_validate_model",
    {
      providerId: input.provider_id,
      baseUrl: input.base_url,
      requestJson: input.request_json,
    }
  );
}

export async function claudeProviderGetApiKeyPlaintext(providerId: number) {
  return invokeService<string>(
    "读取 Claude API Key 失败",
    "claude_provider_get_api_key_plaintext",
    { providerId }
  );
}
