import {
  commands,
  type UsageFolderOptionV1,
  type UsageDayRow,
  type UsageHourlyRow,
  type UsageLeaderboardRow,
  type UsageProviderCacheRateTrendRowV1,
  type UsageProviderMetricsTrendRowV1,
  type UsageProviderRow as GeneratedUsageProviderRow,
  type UsageQueryParams as GeneratedUsageQueryParams,
  type UsageSummary,
} from "../../generated/bindings";
import {
  invokeGeneratedIpc,
  mapGeneratedCommandResponse,
  type GeneratedCommandResult,
} from "../generatedIpc";
import {
  narrowGeneratedStringUnion,
  type OptionalNullableGeneratedFields,
  type Override,
} from "../generatedTypeUtils";
import type { CliKey } from "../providers/providers";
import { CLI_KEYS } from "../../constants/clis";
import {
  USAGE_DEFAULT_FULL_IDLE_GAP_MINUTES,
  USAGE_DEFAULT_SESSION_BREAK_GAP_MINUTES,
  USAGE_FULL_IDLE_GAP_MINUTES_MAX,
  USAGE_FULL_IDLE_GAP_MINUTES_MIN,
  USAGE_SESSION_BREAK_GAP_MINUTES_MAX,
  USAGE_SESSION_BREAK_GAP_MINUTES_MIN,
} from "../../constants/usageDevelopmentTime";

export {
  USAGE_DEFAULT_FULL_IDLE_GAP_MINUTES,
  USAGE_DEFAULT_SESSION_BREAK_GAP_MINUTES,
  USAGE_FULL_IDLE_GAP_MINUTES_MAX,
  USAGE_FULL_IDLE_GAP_MINUTES_MIN,
  USAGE_SESSION_BREAK_GAP_MINUTES_MAX,
  USAGE_SESSION_BREAK_GAP_MINUTES_MIN,
} from "../../constants/usageDevelopmentTime";

const CLI_KEY_VALUES = CLI_KEYS;

export const USAGE_LIMIT_MIN = 1;
export const USAGE_LEADERBOARD_DEFAULT_LIMIT = 10;
export const USAGE_LEADERBOARD_MAX_LIMIT = 50;
export const USAGE_LEADERBOARD_V2_DEFAULT_LIMIT = 200;
export const USAGE_LEADERBOARD_V2_MAX_LIMIT = 200;
export const USAGE_HOURLY_SERIES_MIN_DAYS = 1;
export const USAGE_HOURLY_SERIES_MAX_DAYS = 60;
export const USAGE_PROVIDER_TREND_MAX_LIMIT = 200;

export type UsageRange = "today" | "last7" | "last30" | "month" | "all";
export type UsageScope = "cli" | "provider" | "model" | "folder" | "day";
export type UsagePeriod = "daily" | "weekly" | "monthly" | "allTime" | "custom";

export type UsageProviderRow = Override<
  GeneratedUsageProviderRow,
  {
    cli_key: CliKey;
  }
>;

export type UsageQueryInputV2 = Omit<
  OptionalNullableGeneratedFields<GeneratedUsageQueryParams>,
  "period"
>;
export type NormalizedUsageSummaryInput = {
  cliKey: CliKey | null;
};
export type NormalizedUsageQueryInputV2 = {
  startTs: number | null;
  endTs: number | null;
  cliKey: CliKey | null;
  providerId: number | null;
  folderKeys: string[] | null;
  dayStartHour: number | null;
  fullIdleGapMinutes: number | null;
  sessionBreakGapMinutes: number | null;
  excludeCx2CcGatewayBridge: boolean | null;
};
export type UsageProviderCacheRateTrendInput = Omit<
  UsageQueryInputV2,
  "folderKeys" | "dayStartHour" | "fullIdleGapMinutes" | "sessionBreakGapMinutes"
> & {
  limit?: number | null;
};

export function normalizeUsageLeaderboardCsvExportFilePath(filePath: string): string {
  const normalized = filePath.trim();
  if (!normalized) {
    throw new Error("SEC_INVALID_INPUT: filePath is required");
  }
  return normalized;
}

export function normalizeUsageLeaderboardCsvExportContent(csv: string): string {
  if (
    !csv
      .trimStart()
      .replace(/^\uFEFF/, "")
      .trim()
  ) {
    throw new Error("SEC_INVALID_INPUT: csv is required");
  }
  return csv;
}

function normalizeBoundedInteger(
  label: string,
  value: number | null | undefined,
  max: number
): number | null {
  if (value == null) return null;
  if (!Number.isSafeInteger(value)) {
    throw new Error(`SEC_INVALID_INPUT: invalid ${label}=${value}`);
  }
  return Math.min(Math.max(value, USAGE_LIMIT_MIN), max);
}

export function normalizeUsageLeaderboardLimit(limit?: number | null): number | null {
  return normalizeBoundedInteger("usage leaderboard limit", limit, USAGE_LEADERBOARD_MAX_LIMIT);
}

export function normalizeUsageLeaderboardV2Limit(limit?: number | null): number | null {
  return normalizeBoundedInteger(
    "usage leaderboard v2 limit",
    limit,
    USAGE_LEADERBOARD_V2_MAX_LIMIT
  );
}

export function normalizeUsageHourlySeriesDays(days: number): number {
  const normalized = normalizeBoundedInteger(
    "usage hourly series days",
    days,
    USAGE_HOURLY_SERIES_MAX_DAYS
  );
  return normalized ?? USAGE_HOURLY_SERIES_MIN_DAYS;
}

export function normalizeUsageProviderTrendLimit(limit?: number | null): number | null {
  return normalizeBoundedInteger(
    "usage provider trend limit",
    limit,
    USAGE_PROVIDER_TREND_MAX_LIMIT
  );
}

export function validateUsageCliKey(cliKey?: string | null): CliKey | null {
  if (cliKey == null) return null;
  const normalizedCliKey = cliKey.trim();
  if (!normalizedCliKey) return null;
  if ((CLI_KEY_VALUES as readonly string[]).includes(normalizedCliKey)) {
    return normalizedCliKey as CliKey;
  }
  throw new Error(`SEC_INVALID_INPUT: invalid cliKey=${cliKey}`);
}

function normalizeUsageTimestamp(label: string, value: number | null | undefined): number | null {
  if (value == null) return null;
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new Error(`SEC_INVALID_INPUT: invalid ${label}=${value}`);
  }
  return value;
}

function normalizeUsageProviderId(providerId?: number | null): number | null {
  if (providerId == null) return null;
  if (!Number.isSafeInteger(providerId) || providerId <= 0) {
    throw new Error(`SEC_INVALID_INPUT: invalid providerId=${providerId}`);
  }
  return providerId;
}

function normalizeUsageDayStartHour(value?: number | null): number | null {
  if (value == null) return null;
  if (!Number.isSafeInteger(value) || value < 0 || value > 9) {
    throw new Error(`SEC_INVALID_INPUT: invalid dayStartHour=${value}`);
  }
  return value;
}

function normalizeDevelopmentTimeGapMinutes(
  label: string,
  value: number | null | undefined,
  min: number,
  max: number
): number | null {
  if (value == null) return null;
  if (!Number.isSafeInteger(value) || value < min || value > max) {
    throw new Error(`SEC_INVALID_INPUT: invalid ${label}=${value}`);
  }
  return value;
}

function normalizeUsageBoolean(label: string, value: boolean | null | undefined): boolean | null {
  if (value == null) return null;
  if (typeof value !== "boolean") {
    throw new Error(`SEC_INVALID_INPUT: invalid ${label}`);
  }
  return value;
}

function normalizeUsageFolderKeys(folderKeys?: readonly string[] | null): string[] | null {
  if (folderKeys == null) return null;
  if (!Array.isArray(folderKeys)) {
    throw new Error("SEC_INVALID_INPUT: folderKeys must be an array");
  }

  const normalized = new Set<string>();
  for (const raw of folderKeys) {
    if (typeof raw !== "string") {
      throw new Error("SEC_INVALID_INPUT: folderKeys must contain strings");
    }
    const key = raw.trim();
    if (!key) continue;
    normalized.add(key);
  }

  if (normalized.size === 0) return null;
  return [...normalized].sort((a, b) => a.localeCompare(b));
}

export function normalizeUsageSummaryInput(input?: {
  cliKey?: string | null;
}): NormalizedUsageSummaryInput {
  return {
    cliKey: validateUsageCliKey(input?.cliKey),
  };
}

export function normalizeUsageQueryInputV2(input?: UsageQueryInputV2): NormalizedUsageQueryInputV2 {
  const fullIdleGapMinutes = normalizeDevelopmentTimeGapMinutes(
    "fullIdleGapMinutes",
    input?.fullIdleGapMinutes,
    USAGE_FULL_IDLE_GAP_MINUTES_MIN,
    USAGE_FULL_IDLE_GAP_MINUTES_MAX
  );
  const sessionBreakGapMinutes = normalizeDevelopmentTimeGapMinutes(
    "sessionBreakGapMinutes",
    input?.sessionBreakGapMinutes,
    USAGE_SESSION_BREAK_GAP_MINUTES_MIN,
    USAGE_SESSION_BREAK_GAP_MINUTES_MAX
  );
  if (
    (fullIdleGapMinutes ?? USAGE_DEFAULT_FULL_IDLE_GAP_MINUTES) >=
    (sessionBreakGapMinutes ?? USAGE_DEFAULT_SESSION_BREAK_GAP_MINUTES)
  ) {
    throw new Error(
      "SEC_INVALID_INPUT: fullIdleGapMinutes must be less than sessionBreakGapMinutes"
    );
  }
  return {
    startTs: normalizeUsageTimestamp("startTs", input?.startTs),
    endTs: normalizeUsageTimestamp("endTs", input?.endTs),
    cliKey: validateUsageCliKey(input?.cliKey),
    providerId: normalizeUsageProviderId(input?.providerId),
    folderKeys: normalizeUsageFolderKeys(input?.folderKeys),
    dayStartHour: normalizeUsageDayStartHour(input?.dayStartHour),
    fullIdleGapMinutes,
    sessionBreakGapMinutes,
    excludeCx2CcGatewayBridge: normalizeUsageBoolean(
      "excludeCx2CcGatewayBridge",
      input?.excludeCx2CcGatewayBridge
    ),
  };
}

function buildQueryParamsV2(
  period: UsagePeriod,
  input?: UsageQueryInputV2
): GeneratedUsageQueryParams {
  const normalizedInput = normalizeUsageQueryInputV2(input);
  return {
    period,
    startTs: normalizedInput.startTs,
    endTs: normalizedInput.endTs,
    cliKey: normalizedInput.cliKey,
    providerId: normalizedInput.providerId,
    folderKeys: normalizedInput.folderKeys,
    dayStartHour: normalizedInput.dayStartHour,
    fullIdleGapMinutes: normalizedInput.fullIdleGapMinutes,
    sessionBreakGapMinutes: normalizedInput.sessionBreakGapMinutes,
    excludeCx2CcGatewayBridge: normalizedInput.excludeCx2CcGatewayBridge,
  };
}

function toUsageProviderRow(value: GeneratedUsageProviderRow): UsageProviderRow {
  return {
    ...value,
    cli_key: narrowGeneratedStringUnion(
      value.cli_key,
      CLI_KEY_VALUES,
      "usage_provider_row.cli_key"
    ),
  };
}

export async function usageSummary(range: UsageRange, input?: { cliKey?: CliKey | null }) {
  const normalizedInput = normalizeUsageSummaryInput(input);
  return invokeGeneratedIpc<UsageSummary>({
    title: "读取用量汇总失败",
    cmd: "usage_summary",
    args: {
      range,
      cliKey: normalizedInput.cliKey,
    },
    invoke: () => commands.usageSummary(range, normalizedInput.cliKey),
  });
}

export async function usageLeaderboardProvider(
  range: UsageRange,
  input?: { cliKey?: CliKey | null; limit?: number | null }
) {
  const normalizedInput = normalizeUsageSummaryInput(input);
  const limit = normalizeUsageLeaderboardLimit(input?.limit);

  return invokeGeneratedIpc<UsageProviderRow[]>({
    title: "读取按供应商用量排行失败",
    cmd: "usage_leaderboard_provider",
    args: {
      range,
      cliKey: normalizedInput.cliKey,
      limit,
    },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.usageLeaderboardProvider(range, normalizedInput.cliKey, limit),
        (rows) => rows.map(toUsageProviderRow)
      ),
  });
}

export async function usageLeaderboardDay(
  range: UsageRange,
  input?: { cliKey?: CliKey | null; limit?: number | null }
) {
  const normalizedInput = normalizeUsageSummaryInput(input);
  const limit = normalizeUsageLeaderboardLimit(input?.limit);

  return invokeGeneratedIpc<UsageDayRow[]>({
    title: "读取按日期用量排行失败",
    cmd: "usage_leaderboard_day",
    args: {
      range,
      cliKey: normalizedInput.cliKey,
      limit,
    },
    invoke: () => commands.usageLeaderboardDay(range, normalizedInput.cliKey, limit),
  });
}

export async function usageHourlySeries(days: number) {
  const normalizedDays = normalizeUsageHourlySeriesDays(days);

  return invokeGeneratedIpc<UsageHourlyRow[]>({
    title: "读取小时用量序列失败",
    cmd: "usage_hourly_series",
    args: { days: normalizedDays },
    invoke: () => commands.usageHourlySeries(normalizedDays),
  });
}

export async function usageSummaryV2(period: UsagePeriod, input?: UsageQueryInputV2) {
  const params = buildQueryParamsV2(period, input);
  return invokeGeneratedIpc<UsageSummary>({
    title: "读取用量汇总失败",
    cmd: "usage_summary_v2",
    args: {
      params,
    },
    invoke: () => commands.usageSummaryV2(params),
  });
}

export async function usageLeaderboardV2(
  scope: UsageScope,
  period: UsagePeriod,
  input?: UsageQueryInputV2 & { limit?: number | null }
) {
  const params = buildQueryParamsV2(period, input);
  const limit = normalizeUsageLeaderboardV2Limit(input?.limit);

  return invokeGeneratedIpc<UsageLeaderboardRow[]>({
    title: "读取用量排行榜失败",
    cmd: "usage_leaderboard_v2",
    args: {
      scope,
      params,
      limit,
    },
    invoke: () => commands.usageLeaderboardV2(scope, params, limit),
  });
}

export async function usageFolderOptionsV1(period: UsagePeriod, input?: UsageQueryInputV2) {
  const params = buildQueryParamsV2(period, input);
  return invokeGeneratedIpc<UsageFolderOptionV1[]>({
    title: "读取用量文件夹筛选项失败",
    cmd: "usage_folder_options_v1",
    args: {
      params,
    },
    invoke: () => commands.usageFolderOptionsV1(params),
  });
}

export async function usageProviderCacheRateTrendV1(
  period: UsagePeriod,
  input?: UsageProviderCacheRateTrendInput
) {
  const params = buildQueryParamsV2(period, { ...input, folderKeys: null, dayStartHour: null });
  const limit = normalizeUsageProviderTrendLimit(input?.limit);

  return invokeGeneratedIpc<UsageProviderCacheRateTrendRowV1[]>({
    title: "读取供应商缓存命中趋势失败",
    cmd: "usage_provider_cache_rate_trend_v1",
    args: {
      params,
      limit,
    },
    invoke: () => commands.usageProviderCacheRateTrendV1(params, limit),
  });
}

export async function usageProviderMetricsTrendV1(
  period: UsagePeriod,
  input?: UsageQueryInputV2 & { limit?: number | null }
) {
  const params = buildQueryParamsV2(period, { ...input, folderKeys: null, dayStartHour: null });
  const limit = normalizeUsageProviderTrendLimit(input?.limit);

  return invokeGeneratedIpc<UsageProviderMetricsTrendRowV1[]>({
    title: "读取供应商指标趋势失败",
    cmd: "usage_provider_metrics_trend_v1",
    args: {
      params,
      limit,
    },
    invoke: () => commands.usageProviderMetricsTrendV1(params, limit),
  });
}

export async function usageLeaderboardCsvExport(filePath: string, csv: string) {
  const normalizedFilePath = normalizeUsageLeaderboardCsvExportFilePath(filePath);
  const normalizedCsv = normalizeUsageLeaderboardCsvExportContent(csv);

  return invokeGeneratedIpc<boolean>({
    title: "导出用量排行 CSV 失败",
    cmd: "usage_leaderboard_csv_export",
    args: {
      filePath: normalizedFilePath,
      csv: normalizedCsv,
    },
    invoke: () =>
      commands.usageLeaderboardCsvExport(normalizedFilePath, normalizedCsv) as Promise<
        GeneratedCommandResult<boolean>
      >,
  });
}

export type {
  UsageFolderOptionV1,
  UsageDayRow,
  UsageHourlyRow,
  UsageLeaderboardRow,
  UsageProviderCacheRateTrendRowV1,
  UsageProviderMetricsTrendRowV1,
  UsageSummary,
};
