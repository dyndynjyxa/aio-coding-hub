// Usage:
// - Used by `src/components/home/HomeProviderLimitPanel.tsx` to load provider limit usage data.

import {
  commands,
  type ProviderLimitUsageRow as GeneratedProviderLimitUsageRow,
} from "../../generated/bindings";
import { invokeGeneratedIpc, mapGeneratedCommandResponse } from "../generatedIpc";
import { narrowGeneratedStringUnion, type Override } from "../generatedTypeUtils";
import type { CliKey, DailyResetMode } from "./providers";

const CLI_KEY_VALUES = ["claude", "codex", "gemini"] as const satisfies readonly CliKey[];
const DAILY_RESET_MODE_VALUES = ["fixed", "rolling"] as const satisfies readonly DailyResetMode[];
const PROVIDER_NAME_MAX_CHARS = 256;
const RESET_TIME_MAX_CHARS = 64;

export type ProviderLimitUsageRow = Override<
  GeneratedProviderLimitUsageRow,
  {
    cli_key: CliKey;
    daily_reset_mode: DailyResetMode | null;
  }
>;

export function validateProviderLimitUsageCliKey(cliKey?: string | null): CliKey | null {
  if (cliKey == null) return null;
  const normalizedCliKey = cliKey.trim();
  if ((CLI_KEY_VALUES as readonly string[]).includes(normalizedCliKey)) {
    return normalizedCliKey as CliKey;
  }
  throw new Error(`SEC_INVALID_INPUT: invalid cliKey=${cliKey}`);
}

function normalizeRequiredText(value: string, label: string, maxChars: number): string {
  const normalized = value.trim();
  if (!normalized) {
    throw new Error(`IPC_INVALID_RESULT: ${label} is required`);
  }
  if ([...normalized].length > maxChars) {
    throw new Error(`IPC_INVALID_RESULT: ${label} is too long (max ${maxChars} chars)`);
  }
  return normalized;
}

function normalizeOptionalText(
  value: string | null | undefined,
  label: string,
  maxChars: number
): string | null {
  if (value == null) return null;
  const normalized = value.trim();
  if (!normalized) return null;
  if ([...normalized].length > maxChars) {
    throw new Error(`IPC_INVALID_RESULT: ${label} is too long (max ${maxChars} chars)`);
  }
  return normalized;
}

function normalizeProviderId(providerId: number): number {
  if (!Number.isSafeInteger(providerId) || providerId <= 0) {
    throw new Error(`IPC_INVALID_RESULT: invalid provider_limit_usage.provider_id=${providerId}`);
  }
  return providerId;
}

function normalizeNonNegativeFiniteNumber(value: number, label: string): number {
  if (!Number.isFinite(value) || value < 0) {
    throw new Error(`IPC_INVALID_RESULT: ${label} must be a non-negative finite number`);
  }
  return value;
}

function normalizeNullableNonNegativeFiniteNumber(
  value: number | null | undefined,
  label: string
): number | null {
  if (value == null) return null;
  return normalizeNonNegativeFiniteNumber(value, label);
}

function normalizeWindowTs(value: number, label: string): number {
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new Error(`IPC_INVALID_RESULT: ${label} must be a non-negative safe integer`);
  }
  return value;
}

function normalizeDailyResetMode(value: string | null | undefined): DailyResetMode | null {
  if (value == null) return null;
  const normalized = value.trim();
  if (!normalized) return null;
  return narrowGeneratedStringUnion(
    normalized,
    DAILY_RESET_MODE_VALUES,
    "provider_limit_usage_v1.daily_reset_mode"
  );
}

function toProviderLimitUsageRow(value: GeneratedProviderLimitUsageRow): ProviderLimitUsageRow {
  return {
    provider_id: normalizeProviderId(value.provider_id),
    cli_key: narrowGeneratedStringUnion(
      value.cli_key,
      CLI_KEY_VALUES,
      "provider_limit_usage_v1.cli_key"
    ),
    provider_name: normalizeRequiredText(
      value.provider_name,
      "provider_limit_usage.provider_name",
      PROVIDER_NAME_MAX_CHARS
    ),
    enabled: value.enabled,
    limit_5h_usd: normalizeNullableNonNegativeFiniteNumber(
      value.limit_5h_usd,
      "provider_limit_usage.limit_5h_usd"
    ),
    limit_daily_usd: normalizeNullableNonNegativeFiniteNumber(
      value.limit_daily_usd,
      "provider_limit_usage.limit_daily_usd"
    ),
    daily_reset_mode: normalizeDailyResetMode(value.daily_reset_mode),
    daily_reset_time: normalizeOptionalText(
      value.daily_reset_time,
      "provider_limit_usage.daily_reset_time",
      RESET_TIME_MAX_CHARS
    ),
    limit_weekly_usd: normalizeNullableNonNegativeFiniteNumber(
      value.limit_weekly_usd,
      "provider_limit_usage.limit_weekly_usd"
    ),
    limit_monthly_usd: normalizeNullableNonNegativeFiniteNumber(
      value.limit_monthly_usd,
      "provider_limit_usage.limit_monthly_usd"
    ),
    limit_total_usd: normalizeNullableNonNegativeFiniteNumber(
      value.limit_total_usd,
      "provider_limit_usage.limit_total_usd"
    ),
    usage_5h_usd: normalizeNonNegativeFiniteNumber(
      value.usage_5h_usd,
      "provider_limit_usage.usage_5h_usd"
    ),
    usage_daily_usd: normalizeNonNegativeFiniteNumber(
      value.usage_daily_usd,
      "provider_limit_usage.usage_daily_usd"
    ),
    usage_weekly_usd: normalizeNonNegativeFiniteNumber(
      value.usage_weekly_usd,
      "provider_limit_usage.usage_weekly_usd"
    ),
    usage_monthly_usd: normalizeNonNegativeFiniteNumber(
      value.usage_monthly_usd,
      "provider_limit_usage.usage_monthly_usd"
    ),
    usage_total_usd: normalizeNonNegativeFiniteNumber(
      value.usage_total_usd,
      "provider_limit_usage.usage_total_usd"
    ),
    window_5h_start_ts: normalizeWindowTs(
      value.window_5h_start_ts,
      "provider_limit_usage.window_5h_start_ts"
    ),
    window_daily_start_ts: normalizeWindowTs(
      value.window_daily_start_ts,
      "provider_limit_usage.window_daily_start_ts"
    ),
    window_weekly_start_ts: normalizeWindowTs(
      value.window_weekly_start_ts,
      "provider_limit_usage.window_weekly_start_ts"
    ),
    window_monthly_start_ts: normalizeWindowTs(
      value.window_monthly_start_ts,
      "provider_limit_usage.window_monthly_start_ts"
    ),
  };
}

export async function providerLimitUsageV1(cliKey?: CliKey | null) {
  const normalizedCliKey = validateProviderLimitUsageCliKey(cliKey);

  return invokeGeneratedIpc<ProviderLimitUsageRow[]>({
    title: "读取 Provider 限额用量失败",
    cmd: "provider_limit_usage_v1",
    args: {
      cliKey: normalizedCliKey,
    },
    invoke: async () =>
      mapGeneratedCommandResponse(await commands.providerLimitUsageV1(normalizedCliKey), (rows) =>
        rows.map(toProviderLimitUsageRow)
      ),
  });
}
