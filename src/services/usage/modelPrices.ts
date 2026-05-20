import {
  commands,
  type ModelPriceAliasMatchTypeV1 as GeneratedModelPriceAliasMatchType,
  type ModelPriceAliasesV1 as GeneratedModelPriceAliases,
  type ModelPriceAliasRuleV1 as GeneratedModelPriceAliasRule,
  type ModelPriceSummary as GeneratedModelPriceSummary,
  type ModelPricesSyncReport as GeneratedModelPricesSyncReport,
} from "../../generated/bindings";
import { invokeGeneratedIpc, mapGeneratedCommandResponse } from "../generatedIpc";
import { narrowGeneratedStringUnion, type Override } from "../generatedTypeUtils";
import type { CliKey } from "../providers/providers";
import { observePromiseLikeRejection, type MaybePromiseLike } from "../../utils/promiseLike";
import { logToConsole } from "../consoleLog";

type Listener = () => MaybePromiseLike<void>;

const listeners = new Set<Listener>();
const CLI_KEY_VALUES = ["claude", "codex", "gemini"] as const satisfies readonly CliKey[];
const MODEL_PRICE_ALIAS_MATCH_TYPE_VALUES = [
  "exact",
  "prefix",
  "wildcard",
] as const satisfies readonly GeneratedModelPriceAliasMatchType[];
const MODEL_PRICES_SYNC_STATUS_VALUES = ["updated", "not_modified"] as const;
const MODEL_PRICE_ALIASES_VERSION = 1;
const MAX_MODEL_PRICE_ALIAS_RULES = 512;
const MAX_MODEL_PRICE_MODEL_CHARS = 512;
const MAX_MODEL_PRICE_ALIAS_PATTERN_CHARS = 200;
const MAX_MODEL_PRICE_CURRENCY_CHARS = 16;

export type ModelPricesSyncStatus = (typeof MODEL_PRICES_SYNC_STATUS_VALUES)[number];
export type ModelPricesSyncReport = Override<
  GeneratedModelPricesSyncReport,
  {
    status: ModelPricesSyncStatus;
  }
>;

function logListenerError(error: unknown) {
  logToConsole("warn", "模型定价更新订阅处理失败", { error: String(error) }, "model_prices");
}

function emitUpdated() {
  for (const listener of Array.from(listeners)) {
    if (!listeners.has(listener)) continue;
    try {
      const result = listener();
      observePromiseLikeRejection(result, logListenerError);
    } catch (error) {
      logListenerError(error);
    }
  }
}

export function subscribeModelPricesUpdated(listener: Listener) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function notifyModelPricesUpdated() {
  emitUpdated();
}

let _lastSyncedAt: number | null = null;
let _lastSyncReport: ModelPricesSyncReport | null = null;

export function setLastModelPricesSync(report: ModelPricesSyncReport) {
  _lastSyncedAt = Date.now();
  _lastSyncReport = report;
  emitUpdated();
}

export function getLastModelPricesSync(): {
  syncedAt: number | null;
  report: ModelPricesSyncReport | null;
} {
  return { syncedAt: _lastSyncedAt, report: _lastSyncReport };
}

export type ModelPriceAliasMatchType = GeneratedModelPriceAliasMatchType;

export type ModelPriceAliasRule = Override<
  GeneratedModelPriceAliasRule,
  {
    cli_key: CliKey;
    match_type: ModelPriceAliasMatchType;
  }
>;

export type ModelPriceAliases = Override<
  GeneratedModelPriceAliases,
  {
    rules: ModelPriceAliasRule[];
  }
>;

export type ModelPriceSummary = Override<
  GeneratedModelPriceSummary,
  {
    cli_key: CliKey;
  }
>;

function toCliKey(value: string, label: string): CliKey {
  return narrowGeneratedStringUnion(value.trim(), CLI_KEY_VALUES, label);
}

export function validateModelPricesCliKey(cliKey: string): CliKey {
  const normalizedCliKey = cliKey.trim();
  if ((CLI_KEY_VALUES as readonly string[]).includes(normalizedCliKey)) {
    return normalizedCliKey as CliKey;
  }
  throw new Error(`SEC_INVALID_INPUT: invalid cliKey=${cliKey}`);
}

function normalizeRequiredText(value: string, label: string, maxChars: number): string {
  const normalized = value.trim();
  if (!normalized) {
    throw new Error(`SEC_INVALID_INPUT: ${label} is required`);
  }
  if ([...normalized].length > maxChars) {
    throw new Error(`SEC_INVALID_INPUT: ${label} is too long (max ${maxChars} chars)`);
  }
  return normalized;
}

function normalizePositiveSafeInteger(value: number, label: string): number {
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new Error(`IPC_INVALID_RESULT: invalid ${label}=${value}`);
  }
  return value;
}

function normalizeNonNegativeSafeInteger(value: number, label: string): number {
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new Error(`IPC_INVALID_RESULT: invalid ${label}=${value}`);
  }
  return value;
}

function normalizeAliasMatchType(value: string): ModelPriceAliasMatchType {
  return narrowGeneratedStringUnion(
    value,
    MODEL_PRICE_ALIAS_MATCH_TYPE_VALUES,
    "model_price_aliases.rule.match_type"
  );
}

function validateAliasPatternForMatchType(
  matchType: ModelPriceAliasMatchType,
  pattern: string
): void {
  if (matchType === "wildcard") {
    const wildcardCount = [...pattern].filter((char) => char === "*").length;
    if (wildcardCount !== 1) {
      throw new Error("SEC_INVALID_INPUT: wildcard pattern must contain exactly one '*'");
    }
    return;
  }

  if (pattern.includes("*")) {
    throw new Error("SEC_INVALID_INPUT: pattern must not contain '*' for exact/prefix rules");
  }
}

function toModelPriceAliasRule(value: GeneratedModelPriceAliasRule): ModelPriceAliasRule {
  const cliKey = validateModelPricesCliKey(value.cli_key);
  const matchType = normalizeAliasMatchType(value.match_type);
  const pattern = normalizeRequiredText(
    value.pattern,
    "pattern",
    MAX_MODEL_PRICE_ALIAS_PATTERN_CHARS
  );
  const targetModel = normalizeRequiredText(
    value.target_model,
    "target_model",
    MAX_MODEL_PRICE_ALIAS_PATTERN_CHARS
  );
  if (targetModel.includes("*")) {
    throw new Error("SEC_INVALID_INPUT: target_model must not contain '*'");
  }
  validateAliasPatternForMatchType(matchType, pattern);
  if (typeof value.enabled !== "boolean") {
    throw new Error("SEC_INVALID_INPUT: enabled must be boolean");
  }

  return {
    cli_key: cliKey,
    match_type: matchType,
    pattern,
    target_model: targetModel,
    enabled: value.enabled,
  };
}

export function normalizeModelPriceAliases(value: GeneratedModelPriceAliases): ModelPriceAliases {
  if (!Number.isSafeInteger(value.version) || value.version !== MODEL_PRICE_ALIASES_VERSION) {
    throw new Error(`SEC_INVALID_INPUT: unsupported aliases version ${value.version}`);
  }
  if (!Array.isArray(value.rules)) {
    throw new Error("SEC_INVALID_INPUT: aliases.rules must be an array");
  }
  if (value.rules.length > MAX_MODEL_PRICE_ALIAS_RULES) {
    throw new Error(
      `SEC_INVALID_INPUT: aliases.rules must contain at most ${MAX_MODEL_PRICE_ALIAS_RULES} entries`
    );
  }

  return {
    version: MODEL_PRICE_ALIASES_VERSION,
    rules: value.rules.map(toModelPriceAliasRule),
  };
}

function toModelPriceSummary(value: GeneratedModelPriceSummary): ModelPriceSummary {
  return {
    id: normalizePositiveSafeInteger(value.id, "model_prices.id"),
    cli_key: toCliKey(value.cli_key, "model_prices_list.cli_key"),
    model: normalizeRequiredText(value.model, "model", MAX_MODEL_PRICE_MODEL_CHARS),
    currency: normalizeRequiredText(value.currency, "currency", MAX_MODEL_PRICE_CURRENCY_CHARS),
    created_at: normalizeNonNegativeSafeInteger(value.created_at, "model_prices.created_at"),
    updated_at: normalizeNonNegativeSafeInteger(value.updated_at, "model_prices.updated_at"),
  };
}

export async function modelPricesList(cliKey: CliKey) {
  const normalizedCliKey = validateModelPricesCliKey(cliKey);

  return invokeGeneratedIpc<ModelPriceSummary[]>({
    title: "读取模型价格列表失败",
    cmd: "model_prices_list",
    args: { cliKey: normalizedCliKey },
    invoke: async () =>
      mapGeneratedCommandResponse(await commands.modelPricesList(normalizedCliKey), (rows) =>
        rows.map(toModelPriceSummary)
      ),
  });
}

export async function modelPricesSyncBasellm(force = false) {
  if (typeof force !== "boolean") {
    throw new Error("SEC_INVALID_INPUT: force must be boolean");
  }

  return invokeGeneratedIpc<ModelPricesSyncReport>({
    title: "同步模型价格失败",
    cmd: "model_prices_sync_basellm",
    args: { force },
    invoke: async () =>
      mapGeneratedCommandResponse(await commands.modelPricesSyncBasellm(force), (report) => {
        const status = narrowGeneratedStringUnion(
          report.status,
          MODEL_PRICES_SYNC_STATUS_VALUES,
          "model_prices_sync_basellm.status"
        );
        const inserted = normalizeNonNegativeSafeInteger(
          report.inserted,
          "model_prices_sync_basellm.inserted"
        );
        const updated = normalizeNonNegativeSafeInteger(
          report.updated,
          "model_prices_sync_basellm.updated"
        );
        const skipped = normalizeNonNegativeSafeInteger(
          report.skipped,
          "model_prices_sync_basellm.skipped"
        );
        const total = normalizeNonNegativeSafeInteger(
          report.total,
          "model_prices_sync_basellm.total"
        );
        if (inserted + updated + skipped !== total) {
          throw new Error("IPC_INVALID_RESULT: model_prices_sync_basellm.total mismatch");
        }
        return { status, inserted, updated, skipped, total };
      }),
  });
}

export async function modelPriceAliasesGet() {
  return invokeGeneratedIpc<ModelPriceAliases>({
    title: "读取模型别名规则失败",
    cmd: "model_price_aliases_get",
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.modelPriceAliasesGet(),
        normalizeModelPriceAliases
      ),
  });
}

export async function modelPriceAliasesSet(aliases: ModelPriceAliases) {
  const normalizedAliases = normalizeModelPriceAliases(aliases);

  return invokeGeneratedIpc<ModelPriceAliases>({
    title: "保存模型别名规则失败",
    cmd: "model_price_aliases_set",
    args: { aliases: normalizedAliases },
    invoke: async () =>
      mapGeneratedCommandResponse(
        await commands.modelPriceAliasesSet(normalizedAliases),
        normalizeModelPriceAliases
      ),
  });
}
