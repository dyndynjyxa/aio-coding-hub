// Usage:
// - Used by `src/components/home/HomeCostPanel.tsx` to load cost analytics for the Home "花费" tab.

import {
  commands,
  type CostBackfillReportV1,
  type CostModelBreakdownRowV1,
  type CostProviderBreakdownRowV1 as GeneratedCostProviderBreakdownRowV1,
  type CostQueryParams as GeneratedCostQueryParams,
  type CostScatterCliProviderModelRowV1 as GeneratedCostScatterCliProviderModelRowV1,
  type CostSummaryV1,
  type CostTopRequestRowV1 as GeneratedCostTopRequestRowV1,
  type CostTrendRowV1,
} from "../../generated/bindings";
import { invokeGeneratedIpc, mapGeneratedCommandResponse } from "../generatedIpc";
import {
  narrowGeneratedStringUnion,
  type OptionalNullableGeneratedFields,
  type Override,
} from "../generatedTypeUtils";
import type { CliKey } from "../providers/providers";

const CLI_KEY_VALUES = ["claude", "codex", "gemini"] as const satisfies readonly CliKey[];

export const COST_LIMIT_MIN = 1;
export const COST_BREAKDOWN_DEFAULT_LIMIT = 50;
export const COST_BREAKDOWN_MAX_LIMIT = 200;
export const COST_SCATTER_DEFAULT_LIMIT = 500;
export const COST_SCATTER_MAX_LIMIT = 5000;
export const COST_BACKFILL_DEFAULT_MAX_ROWS = 5000;
export const COST_BACKFILL_MAX_ROWS = 10_000;
export const COST_MODEL_FILTER_MAX_CHARS = 200;

export type CostPeriod = "daily" | "weekly" | "monthly" | "allTime" | "custom";

export type CostProviderBreakdownRowV1 = Override<
  GeneratedCostProviderBreakdownRowV1,
  {
    cli_key: CliKey;
  }
>;

export type CostScatterCliProviderModelRowV1 = Override<
  GeneratedCostScatterCliProviderModelRowV1,
  {
    cli_key: CliKey;
  }
>;

export type CostTopRequestRowV1 = Override<
  GeneratedCostTopRequestRowV1,
  {
    cli_key: CliKey;
  }
>;

export type CostQueryInput = Omit<
  OptionalNullableGeneratedFields<GeneratedCostQueryParams>,
  "period"
>;
export type NormalizedCostQueryInput = {
  startTs: number | null;
  endTs: number | null;
  cliKey: CliKey | null;
  providerId: number | null;
  model: string | null;
};

function normalizeBoundedCostInteger(
  label: string,
  value: number | null | undefined,
  max: number
): number | null {
  if (value == null) return null;
  if (!Number.isSafeInteger(value)) {
    throw new Error(`SEC_INVALID_INPUT: invalid ${label}=${value}`);
  }
  return Math.min(Math.max(value, COST_LIMIT_MIN), max);
}

export function normalizeCostBreakdownLimit(limit?: number | null): number | null {
  return normalizeBoundedCostInteger("cost breakdown limit", limit, COST_BREAKDOWN_MAX_LIMIT);
}

export function normalizeCostScatterLimit(limit?: number | null): number | null {
  return normalizeBoundedCostInteger("cost scatter limit", limit, COST_SCATTER_MAX_LIMIT);
}

export function normalizeCostBackfillMaxRows(maxRows?: number | null): number | null {
  return normalizeBoundedCostInteger("cost backfill maxRows", maxRows, COST_BACKFILL_MAX_ROWS);
}

export function validateCostCliKey(cliKey?: string | null): CliKey | null {
  if (cliKey == null) return null;
  const normalizedCliKey = cliKey.trim();
  if (!normalizedCliKey) return null;
  if ((CLI_KEY_VALUES as readonly string[]).includes(normalizedCliKey)) {
    return normalizedCliKey as CliKey;
  }
  throw new Error(`SEC_INVALID_INPUT: invalid cliKey=${cliKey}`);
}

function normalizeCostTimestamp(label: string, value: number | null | undefined): number | null {
  if (value == null) return null;
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new Error(`SEC_INVALID_INPUT: invalid ${label}=${value}`);
  }
  return value;
}

function normalizeCostProviderId(providerId?: number | null): number | null {
  if (providerId == null) return null;
  if (!Number.isSafeInteger(providerId) || providerId <= 0) {
    throw new Error(`SEC_INVALID_INPUT: invalid providerId=${providerId}`);
  }
  return providerId;
}

function normalizeCostModel(model?: string | null): string | null {
  if (model == null) return null;
  const normalizedModel = model.trim();
  if (!normalizedModel) return null;
  if ([...normalizedModel].length > COST_MODEL_FILTER_MAX_CHARS) {
    throw new Error(
      `SEC_INVALID_INPUT: model is too long (max ${COST_MODEL_FILTER_MAX_CHARS} chars)`
    );
  }
  return normalizedModel;
}

export function normalizeCostQueryInput(input?: CostQueryInput): NormalizedCostQueryInput {
  return {
    startTs: normalizeCostTimestamp("startTs", input?.startTs),
    endTs: normalizeCostTimestamp("endTs", input?.endTs),
    cliKey: validateCostCliKey(input?.cliKey),
    providerId: normalizeCostProviderId(input?.providerId),
    model: normalizeCostModel(input?.model),
  };
}

function buildParams(period: CostPeriod, input?: CostQueryInput): GeneratedCostQueryParams {
  const normalizedInput = normalizeCostQueryInput(input);
  return {
    period,
    startTs: normalizedInput.startTs,
    endTs: normalizedInput.endTs,
    cliKey: normalizedInput.cliKey,
    providerId: normalizedInput.providerId,
    model: normalizedInput.model,
  };
}

function toCliKey(value: string, label: string): CliKey {
  return narrowGeneratedStringUnion(value, CLI_KEY_VALUES, label);
}

function toCostProviderBreakdownRowV1(
  value: GeneratedCostProviderBreakdownRowV1
): CostProviderBreakdownRowV1 {
  return {
    ...value,
    cli_key: toCliKey(value.cli_key, "cost_breakdown_provider_v1.cli_key"),
  };
}

function toCostScatterCliProviderModelRowV1(
  value: GeneratedCostScatterCliProviderModelRowV1
): CostScatterCliProviderModelRowV1 {
  return {
    ...value,
    cli_key: toCliKey(value.cli_key, "cost_scatter_cli_provider_model_v1.cli_key"),
  };
}

function toCostTopRequestRowV1(value: GeneratedCostTopRequestRowV1): CostTopRequestRowV1 {
  return {
    ...value,
    cli_key: toCliKey(value.cli_key, "cost_top_requests_v1.cli_key"),
  };
}

export async function costSummaryV1(period: CostPeriod, input?: CostQueryInput) {
  const params = buildParams(period, input);
  return invokeGeneratedIpc<CostSummaryV1>({
    title: "读取花费汇总失败",
    cmd: "cost_summary_v1",
    args: { params },
    invoke: () => commands.costSummaryV1(params),
  });
}

export async function costTrendV1(period: CostPeriod, input?: CostQueryInput) {
  const params = buildParams(period, input);
  return invokeGeneratedIpc<CostTrendRowV1[]>({
    title: "读取花费趋势失败",
    cmd: "cost_trend_v1",
    args: { params },
    invoke: () => commands.costTrendV1(params),
  });
}

export async function costBreakdownProviderV1(
  period: CostPeriod,
  input?: CostQueryInput & { limit?: number | null }
) {
  const params = buildParams(period, input);
  const limit = normalizeCostBreakdownLimit(input?.limit);

  return invokeGeneratedIpc<CostProviderBreakdownRowV1[]>({
    title: "读取按供应商花费分布失败",
    cmd: "cost_breakdown_provider_v1",
    args: {
      params,
      limit,
    },
    invoke: async () =>
      mapGeneratedCommandResponse(await commands.costBreakdownProviderV1(params, limit), (rows) =>
        rows.map(toCostProviderBreakdownRowV1)
      ),
  });
}

export async function costBreakdownModelV1(
  period: CostPeriod,
  input?: CostQueryInput & { limit?: number | null }
) {
  const params = buildParams(period, input);
  const limit = normalizeCostBreakdownLimit(input?.limit);

  return invokeGeneratedIpc<CostModelBreakdownRowV1[]>({
    title: "读取按模型花费分布失败",
    cmd: "cost_breakdown_model_v1",
    args: {
      params,
      limit,
    },
    invoke: () => commands.costBreakdownModelV1(params, limit),
  });
}

export async function costTopRequestsV1(
  period: CostPeriod,
  input?: CostQueryInput & { limit?: number | null }
) {
  const params = buildParams(period, input);
  const limit = normalizeCostBreakdownLimit(input?.limit);

  return invokeGeneratedIpc<CostTopRequestRowV1[]>({
    title: "读取高花费请求失败",
    cmd: "cost_top_requests_v1",
    args: {
      params,
      limit,
    },
    invoke: async () =>
      mapGeneratedCommandResponse(await commands.costTopRequestsV1(params, limit), (rows) =>
        rows.map(toCostTopRequestRowV1)
      ),
  });
}

export async function costScatterCliProviderModelV1(
  period: CostPeriod,
  input?: CostQueryInput & { limit?: number | null }
) {
  const params = buildParams(period, input);
  const limit = normalizeCostScatterLimit(input?.limit);

  return invokeGeneratedIpc<CostScatterCliProviderModelRowV1[]>({
    title: "读取花费散点数据失败",
    cmd: "cost_scatter_cli_provider_model_v1",
    args: {
      params,
      limit,
    },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.costScatterCliProviderModelV1(params, limit),
        (rows) => rows.map(toCostScatterCliProviderModelRowV1)
      ),
  });
}

export async function costBackfillMissingV1(
  period: CostPeriod,
  input?: CostQueryInput & { maxRows?: number | null }
) {
  const params = buildParams(period, input);
  const maxRows = normalizeCostBackfillMaxRows(input?.maxRows);

  return invokeGeneratedIpc<CostBackfillReportV1>({
    title: "回填花费数据失败",
    cmd: "cost_backfill_missing_v1",
    args: {
      params,
      maxRows,
    },
    invoke: () => commands.costBackfillMissingV1(params, maxRows),
  });
}

export type { CostBackfillReportV1, CostModelBreakdownRowV1, CostSummaryV1, CostTrendRowV1 };
