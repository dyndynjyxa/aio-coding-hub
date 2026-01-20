import { invokeTauriOrNull } from "./tauriInvoke";

export type CliKey = "claude" | "codex" | "gemini";

export type ClaudeModels = {
  main_model?: string | null;
  reasoning_model?: string | null;
  haiku_model?: string | null;
  sonnet_model?: string | null;
  opus_model?: string | null;
};

export type ProviderSummary = {
  id: number;
  cli_key: CliKey;
  name: string;
  base_urls: string[];
  base_url_mode: "order" | "ping";
  claude_models: ClaudeModels;
  enabled: boolean;
  priority: number;
  cost_multiplier: number;
  created_at: number;
  updated_at: number;
};

export async function providersList(cliKey: CliKey) {
  return invokeTauriOrNull<ProviderSummary[]>("providers_list", { cliKey });
}

export async function providerUpsert(input: {
  provider_id?: number | null;
  cli_key: CliKey;
  name: string;
  base_urls: string[];
  base_url_mode: "order" | "ping";
  api_key?: string | null;
  enabled: boolean;
  cost_multiplier: number;
  priority?: number | null;
  claude_models?: ClaudeModels | null;
}) {
  return invokeTauriOrNull<ProviderSummary>("provider_upsert", {
    providerId: input.provider_id ?? null,
    cliKey: input.cli_key,
    name: input.name,
    baseUrls: input.base_urls,
    baseUrlMode: input.base_url_mode,
    apiKey: input.api_key ?? null,
    enabled: input.enabled,
    costMultiplier: input.cost_multiplier,
    priority: input.priority ?? null,
    claudeModels: input.claude_models ?? null,
  });
}

export async function baseUrlPingMs(baseUrl: string) {
  return invokeTauriOrNull<number>("base_url_ping_ms", { baseUrl });
}

export async function providerSetEnabled(providerId: number, enabled: boolean) {
  return invokeTauriOrNull<ProviderSummary>("provider_set_enabled", {
    providerId,
    enabled,
  });
}

export async function providerDelete(providerId: number) {
  return invokeTauriOrNull<boolean>("provider_delete", { providerId });
}

export async function providersReorder(cliKey: CliKey, orderedProviderIds: number[]) {
  return invokeTauriOrNull<ProviderSummary[]>("providers_reorder", {
    cliKey,
    orderedProviderIds,
  });
}
