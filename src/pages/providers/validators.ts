// Usage: Client-side validators for provider-related forms (toast-based UX).

export function validateProviderName(name: string) {
  if (name.trim()) return null;
  return "名称不能为空";
}

export function validateProviderApiKeyForCreate(apiKey: string) {
  if (apiKey.trim()) return null;
  return "API Key 不能为空（新增 Provider 必填）";
}

export function parseAndValidateCostMultiplier(raw: string) {
  const value = Number(raw);
  if (!Number.isFinite(value)) {
    return { ok: false as const, message: "价格倍率必须是数字" };
  }
  if (value <= 0) {
    return { ok: false as const, message: "价格倍率必须大于 0" };
  }
  if (value > 1000) {
    return { ok: false as const, message: "价格倍率不能大于 1000" };
  }
  return { ok: true as const, value };
}

const MAX_MODEL_NAME_LEN = 200;

export function validateProviderClaudeModels(input: {
  main_model?: string | null;
  reasoning_model?: string | null;
  haiku_model?: string | null;
  sonnet_model?: string | null;
  opus_model?: string | null;
}) {
  const fields: Array<[label: string, value: string | null | undefined]> = [
    ["主模型", input.main_model],
    ["推理模型(Thinking)", input.reasoning_model],
    ["Haiku 默认模型", input.haiku_model],
    ["Sonnet 默认模型", input.sonnet_model],
    ["Opus 默认模型", input.opus_model],
  ];

  for (const [label, value] of fields) {
    const trimmed = (value ?? "").trim();
    if (!trimmed) continue;
    if (trimmed.length > MAX_MODEL_NAME_LEN) {
      return `${label} 过长（最多 ${MAX_MODEL_NAME_LEN} 字符）`;
    }
  }

  return null;
}
