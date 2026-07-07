// Usage: Dev preview data for HomeTokenCostPanel.
// Provides synthetic UsageLeaderboardRow[] and UsageSummary when no real data is available.

import type {
  UsageDayDetailV1,
  UsageDayFolderRow,
  UsageDayHourRow,
  UsageFolderOptionV1,
  UsageLeaderboardRow,
  UsageSummary,
} from "../../services/usage/usage";

function previewDayKey(dayOffset: number) {
  const date = new Date();
  date.setDate(date.getDate() + dayOffset);
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function weightedAverage(
  rows: UsageLeaderboardRow[],
  value: (row: UsageLeaderboardRow) => number | null,
  weight: (row: UsageLeaderboardRow) => number
) {
  const totalWeight = rows.reduce((sum, row) => sum + Math.max(0, weight(row)), 0);
  if (totalWeight <= 0) return null;
  const totalValue = rows.reduce((sum, row) => {
    const current = value(row);
    if (current == null || !Number.isFinite(current)) return sum;
    return sum + current * Math.max(0, weight(row));
  }, 0);
  return totalValue / totalWeight;
}

type PreviewLeaderboardRowInput = Omit<
  UsageLeaderboardRow,
  "first_request_created_at_ms" | "last_request_created_at_ms"
>;

function withoutRequestBounds(row: PreviewLeaderboardRowInput): UsageLeaderboardRow {
  return {
    ...row,
    first_request_created_at_ms: null,
    last_request_created_at_ms: null,
  };
}

function localDayTimeMs(dayKey: string, hour: number, minute: number) {
  const [year, month, day] = dayKey.split("-").map(Number);
  return new Date(year, month - 1, day, hour, minute, 0, 0).getTime();
}

function withPreviewDayBounds(
  row: PreviewLeaderboardRowInput,
  firstHour: number,
  lastHour: number,
  lastMinute: number
): UsageLeaderboardRow {
  return {
    ...row,
    first_request_created_at_ms: localDayTimeMs(row.key, firstHour, 0),
    last_request_created_at_ms: localDayTimeMs(row.key, lastHour, lastMinute),
  };
}

const PREVIEW_TOKEN_PROVIDER_BASE_ROWS: PreviewLeaderboardRowInput[] = [
  {
    key: "provider:201",
    name: "OpenAI Primary",
    requests_total: 18,
    requests_success: 17,
    requests_failed: 1,
    total_tokens: 49_200,
    io_total_tokens: 42_000,
    input_tokens: 28_000,
    output_tokens: 14_000,
    cache_creation_input_tokens: 2_600,
    cache_read_input_tokens: 4_600,
    total_duration_ms: 17_640,
    avg_duration_ms: 980,
    avg_ttfb_ms: 240,
    avg_output_tokens_per_second: 96.5,
    cost_usd: 1.38,
  },
  {
    key: "provider:101",
    name: "Claude Main",
    requests_total: 15,
    requests_success: 14,
    requests_failed: 1,
    total_tokens: 41_400,
    io_total_tokens: 33_000,
    input_tokens: 21_000,
    output_tokens: 12_000,
    cache_creation_input_tokens: 2_100,
    cache_read_input_tokens: 6_300,
    total_duration_ms: 16_800,
    avg_duration_ms: 1_120,
    avg_ttfb_ms: 310,
    avg_output_tokens_per_second: 84.2,
    cost_usd: 1.16,
  },
  {
    key: "provider:301",
    name: "Gemini Mirror",
    requests_total: 12,
    requests_success: 11,
    requests_failed: 1,
    total_tokens: 28_600,
    io_total_tokens: 24_000,
    input_tokens: 15_000,
    output_tokens: 9_000,
    cache_creation_input_tokens: 1_200,
    cache_read_input_tokens: 3_400,
    total_duration_ms: 10_320,
    avg_duration_ms: 860,
    avg_ttfb_ms: 220,
    avg_output_tokens_per_second: 105.7,
    cost_usd: 0.82,
  },
];

export const PREVIEW_TOKEN_PROVIDER_ROWS: UsageLeaderboardRow[] =
  PREVIEW_TOKEN_PROVIDER_BASE_ROWS.map(withoutRequestBounds);

const PREVIEW_TOKEN_MODEL_BASE_ROWS: PreviewLeaderboardRowInput[] = [
  {
    key: "model:gpt-5.4",
    name: "gpt-5.4",
    requests_total: 14,
    requests_success: 13,
    requests_failed: 1,
    total_tokens: 37_100,
    io_total_tokens: 32_000,
    input_tokens: 21_000,
    output_tokens: 11_000,
    cache_creation_input_tokens: 1_900,
    cache_read_input_tokens: 3_200,
    total_duration_ms: 13_020,
    avg_duration_ms: 930,
    avg_ttfb_ms: 230,
    avg_output_tokens_per_second: 98.4,
    cost_usd: 1.12,
  },
  {
    key: "model:claude-3.7-sonnet",
    name: "claude-3.7-sonnet",
    requests_total: 11,
    requests_success: 10,
    requests_failed: 1,
    total_tokens: 29_800,
    io_total_tokens: 24_000,
    input_tokens: 15_000,
    output_tokens: 9_000,
    cache_creation_input_tokens: 1_500,
    cache_read_input_tokens: 4_300,
    total_duration_ms: 12_980,
    avg_duration_ms: 1_180,
    avg_ttfb_ms: 320,
    avg_output_tokens_per_second: 82.1,
    cost_usd: 0.86,
  },
  {
    key: "model:gemini-2.5-pro",
    name: "gemini-2.5-pro",
    requests_total: 8,
    requests_success: 7,
    requests_failed: 1,
    total_tokens: 17_900,
    io_total_tokens: 15_000,
    input_tokens: 9_000,
    output_tokens: 6_000,
    cache_creation_input_tokens: 800,
    cache_read_input_tokens: 2_100,
    total_duration_ms: 7_200,
    avg_duration_ms: 900,
    avg_ttfb_ms: 220,
    avg_output_tokens_per_second: 97.8,
    cost_usd: 0.48,
  },
  {
    key: "model:gpt-4.1",
    name: "gpt-4.1",
    requests_total: 4,
    requests_success: 4,
    requests_failed: 0,
    total_tokens: 12_100,
    io_total_tokens: 10_000,
    input_tokens: 7_000,
    output_tokens: 3_000,
    cache_creation_input_tokens: 700,
    cache_read_input_tokens: 1_400,
    total_duration_ms: 4_360,
    avg_duration_ms: 1_090,
    avg_ttfb_ms: 270,
    avg_output_tokens_per_second: 87.9,
    cost_usd: 0.33,
  },
  {
    key: "model:claude-3.5-haiku",
    name: "claude-3.5-haiku",
    requests_total: 4,
    requests_success: 4,
    requests_failed: 0,
    total_tokens: 11_600,
    io_total_tokens: 9_000,
    input_tokens: 6_000,
    output_tokens: 3_000,
    cache_creation_input_tokens: 600,
    cache_read_input_tokens: 2_000,
    total_duration_ms: 3_640,
    avg_duration_ms: 910,
    avg_ttfb_ms: 230,
    avg_output_tokens_per_second: 92.7,
    cost_usd: 0.31,
  },
  {
    key: "model:gemini-2.5-flash",
    name: "gemini-2.5-flash",
    requests_total: 4,
    requests_success: 4,
    requests_failed: 0,
    total_tokens: 10_700,
    io_total_tokens: 9_000,
    input_tokens: 6_000,
    output_tokens: 3_000,
    cache_creation_input_tokens: 400,
    cache_read_input_tokens: 1_300,
    total_duration_ms: 3_120,
    avg_duration_ms: 780,
    avg_ttfb_ms: 190,
    avg_output_tokens_per_second: 118.6,
    cost_usd: 0.26,
  },
];

export const PREVIEW_TOKEN_MODEL_ROWS: UsageLeaderboardRow[] =
  PREVIEW_TOKEN_MODEL_BASE_ROWS.map(withoutRequestBounds);

const PREVIEW_TODAY_KEY = previewDayKey(0);
const PREVIEW_YESTERDAY_KEY = previewDayKey(-1);
const PREVIEW_TWO_DAYS_AGO_KEY = previewDayKey(-2);

const PREVIEW_TOKEN_DAY_BASE_ROWS: PreviewLeaderboardRowInput[] = [
  {
    key: PREVIEW_TODAY_KEY,
    name: PREVIEW_TODAY_KEY,
    requests_total: 20,
    requests_success: 19,
    requests_failed: 1,
    total_tokens: 44_000,
    io_total_tokens: 36_000,
    input_tokens: 23_000,
    output_tokens: 13_000,
    cache_creation_input_tokens: 2_400,
    cache_read_input_tokens: 5_600,
    total_duration_ms: 19_200,
    avg_duration_ms: 960,
    avg_ttfb_ms: 230,
    avg_output_tokens_per_second: 101.2,
    cost_usd: 1.48,
  },
  {
    key: PREVIEW_YESTERDAY_KEY,
    name: PREVIEW_YESTERDAY_KEY,
    requests_total: 15,
    requests_success: 14,
    requests_failed: 1,
    total_tokens: 32_400,
    io_total_tokens: 27_000,
    input_tokens: 17_000,
    output_tokens: 10_000,
    cache_creation_input_tokens: 1_600,
    cache_read_input_tokens: 3_800,
    total_duration_ms: 16_200,
    avg_duration_ms: 1_080,
    avg_ttfb_ms: 290,
    avg_output_tokens_per_second: 88.4,
    cost_usd: 1.1,
  },
  {
    key: PREVIEW_TWO_DAYS_AGO_KEY,
    name: PREVIEW_TWO_DAYS_AGO_KEY,
    requests_total: 10,
    requests_success: 9,
    requests_failed: 1,
    total_tokens: 20_800,
    io_total_tokens: 18_000,
    input_tokens: 11_000,
    output_tokens: 7_000,
    cache_creation_input_tokens: 900,
    cache_read_input_tokens: 1_900,
    total_duration_ms: 9_000,
    avg_duration_ms: 900,
    avg_ttfb_ms: 220,
    avg_output_tokens_per_second: 104.8,
    cost_usd: 0.78,
  },
];

export const PREVIEW_TOKEN_DAY_ROWS: UsageLeaderboardRow[] = PREVIEW_TOKEN_DAY_BASE_ROWS.map(
  (row, index) => withPreviewDayBounds(row, 8 + index, 23 - index, 34)
);

const PREVIEW_DAY_FOLDER_SPECS = [
  {
    key: "/Users/demo/aio-coding-hub",
    name: "aio-coding-hub",
    folder_path: "/Users/demo/aio-coding-hub",
    share: 0.52,
    latencyOffsetMs: 80,
  },
  {
    key: "/Users/demo/workspace-alpha",
    name: "workspace-alpha",
    folder_path: "/Users/demo/workspace-alpha",
    share: 0.31,
    latencyOffsetMs: -40,
  },
  {
    key: "__unknown__",
    name: "未知文件夹",
    folder_path: null,
    share: 0.17,
    latencyOffsetMs: 20,
  },
] as const;

export const PREVIEW_TOKEN_FOLDER_OPTIONS: UsageFolderOptionV1[] = PREVIEW_DAY_FOLDER_SPECS.map(
  (spec) => ({
    key: spec.key,
    name: spec.name,
    folder_path: spec.folder_path,
    requests_total: Math.round(45 * spec.share),
    total_tokens: Math.round(97_200 * spec.share),
  })
);

const PREVIEW_DAY_HOUR_WEIGHTS: readonly number[] = [
  0, 0, 0, 0, 0, 0.08, 0.12, 0.1, 0.06, 0.04, 0.05, 0.08, 0.12, 0.1, 0.08, 0.06, 0.04, 0.03, 0.02,
  0.02, 0, 0, 0, 0,
];

export function scalePreviewTokenRows(
  rows: UsageLeaderboardRow[],
  factor: number
): UsageLeaderboardRow[] {
  const scale = (value: number) => Math.max(0, Math.round(value * factor));
  return rows.map((row) => {
    const requestsTotal = scale(row.requests_total);
    const requestsFailed = Math.min(requestsTotal, scale(row.requests_failed));
    const requestsSuccess = Math.max(0, requestsTotal - requestsFailed);

    return {
      ...row,
      requests_total: requestsTotal,
      requests_success: requestsSuccess,
      requests_failed: requestsFailed,
      total_tokens: scale(row.total_tokens),
      io_total_tokens: scale(row.io_total_tokens),
      input_tokens: scale(row.input_tokens),
      output_tokens: scale(row.output_tokens),
      cache_creation_input_tokens: scale(row.cache_creation_input_tokens),
      cache_read_input_tokens: scale(row.cache_read_input_tokens),
      total_duration_ms: scale(row.total_duration_ms),
      cost_usd: row.cost_usd == null ? null : row.cost_usd * factor,
    };
  });
}

export function previewFolderSelectionFactor(folderKeys: readonly string[] | null | undefined) {
  if (!folderKeys || folderKeys.length === 0) return 1;
  const shareByKey = new Map<string, number>(
    PREVIEW_DAY_FOLDER_SPECS.map((spec) => [spec.key, spec.share])
  );
  const share = folderKeys.reduce((sum, key) => sum + (shareByKey.get(key) ?? 0), 0);
  return Math.max(0, Math.min(1, share));
}

function buildPreviewFolderRows(dayRow: UsageLeaderboardRow): UsageDayFolderRow[] {
  return PREVIEW_DAY_FOLDER_SPECS.map((spec) => {
    const requestsTotal = Math.max(1, Math.round(dayRow.requests_total * spec.share));
    const requestsFailed = Math.min(requestsTotal, Math.round(dayRow.requests_failed * spec.share));
    const requestsSuccess = Math.max(0, requestsTotal - requestsFailed);
    return {
      key: spec.key,
      name: spec.name,
      folder_path: spec.folder_path,
      requests_total: requestsTotal,
      requests_success: requestsSuccess,
      requests_failed: requestsFailed,
      total_tokens: Math.round(dayRow.total_tokens * spec.share),
      io_total_tokens: Math.round(dayRow.io_total_tokens * spec.share),
      input_tokens: Math.round(dayRow.input_tokens * spec.share),
      output_tokens: Math.round(dayRow.output_tokens * spec.share),
      cache_creation_input_tokens: Math.round(dayRow.cache_creation_input_tokens * spec.share),
      cache_read_input_tokens: Math.round(dayRow.cache_read_input_tokens * spec.share),
      avg_duration_ms:
        dayRow.avg_duration_ms == null
          ? null
          : Math.max(0, Math.round(dayRow.avg_duration_ms + spec.latencyOffsetMs)),
      avg_ttfb_ms:
        dayRow.avg_ttfb_ms == null
          ? null
          : Math.max(0, Math.round(dayRow.avg_ttfb_ms + spec.latencyOffsetMs / 4)),
      avg_output_tokens_per_second: dayRow.avg_output_tokens_per_second,
      cost_usd: dayRow.cost_usd == null ? null : dayRow.cost_usd * spec.share,
    };
  });
}

function buildPreviewHourRows(dayRow: UsageLeaderboardRow): UsageDayHourRow[] {
  const totalWeight = PREVIEW_DAY_HOUR_WEIGHTS.reduce((sum, weight) => sum + weight, 0);
  return PREVIEW_DAY_HOUR_WEIGHTS.map((weight, hour) => {
    const ratio = totalWeight > 0 ? weight / totalWeight : 0;
    return {
      hour,
      requests_total: weight <= 0 ? 0 : Math.max(1, Math.round(dayRow.requests_total * ratio)),
      total_tokens: Math.round(dayRow.total_tokens * ratio),
      io_total_tokens: Math.round(dayRow.io_total_tokens * ratio),
    };
  });
}

export function buildPreviewTokenDayDetail(
  day: string,
  factor: number,
  folderKeys?: readonly string[] | null
): UsageDayDetailV1 | null {
  const scaledRows = scalePreviewTokenRows(PREVIEW_TOKEN_DAY_ROWS, factor);
  const dayRow = scaledRows.find((row) => row.key === day);
  if (!dayRow) return null;
  const selected = folderKeys && folderKeys.length > 0 ? new Set(folderKeys) : null;
  const folders = buildPreviewFolderRows(dayRow).filter(
    (folder) => !selected || selected.has(folder.key)
  );
  const hourFactor = previewFolderSelectionFactor(folderKeys);
  return {
    day,
    folders,
    hours: buildPreviewHourRows(scalePreviewTokenRows([dayRow], hourFactor)[0]),
  };
}

export function buildPreviewTokenSummary(rows: UsageLeaderboardRow[]): UsageSummary {
  const requestsTotal = rows.reduce((sum, row) => sum + row.requests_total, 0);
  const requestsSuccess = rows.reduce((sum, row) => sum + row.requests_success, 0);
  const requestsFailed = rows.reduce((sum, row) => sum + row.requests_failed, 0);
  const inputTokens = rows.reduce((sum, row) => sum + row.input_tokens, 0);
  const outputTokens = rows.reduce((sum, row) => sum + row.output_tokens, 0);
  const ioTotalTokens = rows.reduce((sum, row) => sum + row.io_total_tokens, 0);
  const totalTokens = rows.reduce((sum, row) => sum + row.total_tokens, 0);
  const totalDurationMs = rows.reduce((sum, row) => sum + row.total_duration_ms, 0);
  const cacheCreationTokens = rows.reduce((sum, row) => sum + row.cache_creation_input_tokens, 0);
  const cacheReadTokens = rows.reduce((sum, row) => sum + row.cache_read_input_tokens, 0);

  return {
    requests_total: requestsTotal,
    requests_with_usage: requestsTotal,
    requests_success: requestsSuccess,
    requests_failed: requestsFailed,
    cost_covered_success: rows.reduce(
      (sum, row) =>
        sum + (row.cost_usd != null && Number.isFinite(row.cost_usd) ? row.requests_success : 0),
      0
    ),
    total_duration_ms: totalDurationMs,
    avg_duration_ms: weightedAverage(
      rows,
      (row) => row.avg_duration_ms,
      (row) => row.requests_total
    ),
    avg_ttfb_ms: weightedAverage(
      rows,
      (row) => row.avg_ttfb_ms,
      (row) => row.requests_total
    ),
    avg_output_tokens_per_second: weightedAverage(
      rows,
      (row) => row.avg_output_tokens_per_second,
      (row) => row.output_tokens
    ),
    input_tokens: inputTokens,
    output_tokens: outputTokens,
    io_total_tokens: ioTotalTokens,
    total_tokens: totalTokens,
    cache_read_input_tokens: cacheReadTokens,
    cache_creation_input_tokens: cacheCreationTokens,
    cache_creation_5m_input_tokens: Math.round(cacheCreationTokens * 0.68),
    cache_creation_1h_input_tokens: Math.round(cacheCreationTokens * 0.32),
  };
}
