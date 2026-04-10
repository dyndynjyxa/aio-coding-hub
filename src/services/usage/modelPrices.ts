import { invokeService } from "../invokeServiceCommand";
import type { CliKey } from "../providers/providers";

type Listener = () => void;

const listeners = new Set<Listener>();

function emitUpdated() {
  for (const listener of listeners) listener();
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

export type ModelPricesSyncReport = {
  status: "updated" | "not_modified" | string;
  inserted: number;
  updated: number;
  skipped: number;
  total: number;
};

export type ModelPriceAliasMatchType = "exact" | "prefix" | "wildcard";

export type ModelPriceAliasRule = {
  cli_key: CliKey;
  match_type: ModelPriceAliasMatchType;
  pattern: string;
  target_model: string;
  enabled: boolean;
};

export type ModelPriceAliases = {
  version: number;
  rules: ModelPriceAliasRule[];
};

export type ModelPriceSummary = {
  id: number;
  cli_key: CliKey;
  model: string;
  currency: string;
  created_at: number;
  updated_at: number;
};

export async function modelPricesList(cliKey: CliKey) {
  return invokeService<ModelPriceSummary[]>("读取模型价格列表失败", "model_prices_list", {
    cliKey,
  });
}

export async function modelPricesSyncBasellm(force = false) {
  return invokeService<ModelPricesSyncReport>("同步模型价格失败", "model_prices_sync_basellm", {
    force,
  });
}

export async function modelPriceAliasesGet() {
  return invokeService<ModelPriceAliases>("读取模型别名规则失败", "model_price_aliases_get");
}

export async function modelPriceAliasesSet(aliases: ModelPriceAliases) {
  return invokeService<ModelPriceAliases>("保存模型别名规则失败", "model_price_aliases_set", {
    aliases,
  });
}
