// Usage: Dev preview data for HomeTokenCostPanel.
// Provides synthetic UsageLeaderboardRow[] and UsageSummary when no real data is available.

import type { UsageLeaderboardRow, UsageSummary } from "../../services/usage/usage";

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

export const PREVIEW_TOKEN_PROVIDER_ROWS: UsageLeaderboardRow[] = [
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
    avg_duration_ms: 860,
    avg_ttfb_ms: 220,
    avg_output_tokens_per_second: 105.7,
    cost_usd: 0.82,
  },
];

export const PREVIEW_TOKEN_MODEL_ROWS: UsageLeaderboardRow[] = [
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
    avg_duration_ms: 780,
    avg_ttfb_ms: 190,
    avg_output_tokens_per_second: 118.6,
    cost_usd: 0.26,
  },
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
      cost_usd: row.cost_usd == null ? null : row.cost_usd * factor,
    };
  });
}

export function buildPreviewTokenSummary(rows: UsageLeaderboardRow[]): UsageSummary {
  const requestsTotal = rows.reduce((sum, row) => sum + row.requests_total, 0);
  const requestsSuccess = rows.reduce((sum, row) => sum + row.requests_success, 0);
  const requestsFailed = rows.reduce((sum, row) => sum + row.requests_failed, 0);
  const inputTokens = rows.reduce((sum, row) => sum + row.input_tokens, 0);
  const outputTokens = rows.reduce((sum, row) => sum + row.output_tokens, 0);
  const ioTotalTokens = rows.reduce((sum, row) => sum + row.io_total_tokens, 0);
  const totalTokens = rows.reduce((sum, row) => sum + row.total_tokens, 0);
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
  };
}
