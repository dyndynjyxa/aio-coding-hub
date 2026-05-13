import {
  commands,
  type UsageDayDetailParams as GeneratedUsageDayDetailParams,
  type UsageDayDetailV1,
  type UsageDayFolderRow,
  type UsageDayHourRow,
  type UsageFolderOptionV1,
  type UsageDayRow,
  type UsageHourlyRow,
  type UsageLeaderboardRow,
  type UsageProviderCacheRateTrendRowV1,
  type UsageProviderRow as GeneratedUsageProviderRow,
  type UsageQueryParams as GeneratedUsageQueryParams,
  type UsageSummary,
} from "../../generated/bindings";
import { invokeGeneratedIpc, mapGeneratedCommandResponse } from "../generatedIpc";
import {
  narrowGeneratedStringUnion,
  type OptionalNullableGeneratedFields,
  type Override,
} from "../generatedTypeUtils";
import type { CliKey } from "../providers/providers";

const CLI_KEY_VALUES = ["claude", "codex", "gemini"] as const satisfies readonly CliKey[];

export type UsageRange = "today" | "last7" | "last30" | "month" | "all";
export type UsageScope = "cli" | "provider" | "model" | "day";
export type UsagePeriod = "daily" | "weekly" | "monthly" | "allTime" | "custom";

export type UsageProviderRow = Override<
  GeneratedUsageProviderRow,
  {
    cli_key: CliKey;
  }
>;

type UsageQueryInputV2 = Omit<OptionalNullableGeneratedFields<GeneratedUsageQueryParams>, "period">;
export type UsageDayDetailInput = Override<
  OptionalNullableGeneratedFields<GeneratedUsageDayDetailParams>,
  {
    cliKey?: CliKey | null;
  }
>;

function buildQueryParamsV2(
  period: UsagePeriod,
  input?: UsageQueryInputV2
): GeneratedUsageQueryParams {
  return {
    period,
    startTs: input?.startTs ?? null,
    endTs: input?.endTs ?? null,
    cliKey: input?.cliKey ?? null,
    providerId: input?.providerId ?? null,
    folderKeys: input?.folderKeys ?? null,
    excludeCx2CcGatewayBridge: input?.excludeCx2CcGatewayBridge ?? null,
  };
}

function buildUsageDayDetailParams(input: UsageDayDetailInput): GeneratedUsageDayDetailParams {
  return {
    day: input.day,
    cliKey: input.cliKey ?? null,
    providerId: input.providerId ?? null,
    folderLimit: input.folderLimit ?? null,
    folderKeys: input.folderKeys ?? null,
    excludeCx2CcGatewayBridge: input.excludeCx2CcGatewayBridge ?? null,
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
  return invokeGeneratedIpc<UsageSummary>({
    title: "读取用量汇总失败",
    cmd: "usage_summary",
    args: {
      range,
      cliKey: input?.cliKey ?? null,
    },
    invoke: () => commands.usageSummary(range, input?.cliKey ?? null),
  });
}

export async function usageLeaderboardProvider(
  range: UsageRange,
  input?: { cliKey?: CliKey | null; limit?: number }
) {
  return invokeGeneratedIpc<UsageProviderRow[]>({
    title: "读取按供应商用量排行失败",
    cmd: "usage_leaderboard_provider",
    args: {
      range,
      cliKey: input?.cliKey ?? null,
      limit: input?.limit,
    },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.usageLeaderboardProvider(range, input?.cliKey ?? null, input?.limit ?? null),
        (rows) => rows.map(toUsageProviderRow)
      ),
  });
}

export async function usageLeaderboardDay(
  range: UsageRange,
  input?: { cliKey?: CliKey | null; limit?: number }
) {
  return invokeGeneratedIpc<UsageDayRow[]>({
    title: "读取按日期用量排行失败",
    cmd: "usage_leaderboard_day",
    args: {
      range,
      cliKey: input?.cliKey ?? null,
      limit: input?.limit,
    },
    invoke: () => commands.usageLeaderboardDay(range, input?.cliKey ?? null, input?.limit ?? null),
  });
}

export async function usageHourlySeries(days: number) {
  return invokeGeneratedIpc<UsageHourlyRow[]>({
    title: "读取小时用量序列失败",
    cmd: "usage_hourly_series",
    args: { days },
    invoke: () => commands.usageHourlySeries(days),
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
  return invokeGeneratedIpc<UsageLeaderboardRow[]>({
    title: "读取用量排行榜失败",
    cmd: "usage_leaderboard_v2",
    args: {
      scope,
      params,
      limit: input?.limit,
    },
    invoke: () => commands.usageLeaderboardV2(scope, params, input?.limit ?? null),
  });
}

export async function usageDayDetailV1(input: UsageDayDetailInput) {
  const params = buildUsageDayDetailParams(input);
  return invokeGeneratedIpc<UsageDayDetailV1>({
    title: "读取日期用量详情失败",
    cmd: "usage_day_detail_v1",
    args: {
      params,
    },
    invoke: () => commands.usageDayDetailV1(params),
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
  input?: UsageQueryInputV2 & { limit?: number | null }
) {
  const params = buildQueryParamsV2(period, input);
  return invokeGeneratedIpc<UsageProviderCacheRateTrendRowV1[]>({
    title: "读取供应商缓存命中趋势失败",
    cmd: "usage_provider_cache_rate_trend_v1",
    args: {
      params,
      limit: input?.limit,
    },
    invoke: () => commands.usageProviderCacheRateTrendV1(params, input?.limit ?? null),
  });
}

export type {
  UsageDayDetailV1,
  UsageDayFolderRow,
  UsageDayHourRow,
  UsageFolderOptionV1,
  UsageDayRow,
  UsageHourlyRow,
  UsageLeaderboardRow,
  UsageProviderCacheRateTrendRowV1,
  UsageSummary,
};
