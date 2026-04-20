import { commands } from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";
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
  return invokeGeneratedIpc<ModelPriceSummary[]>({
    title: "读取模型价格列表失败",
    cmd: "model_prices_list",
    args: { cliKey },
    invoke: () =>
      commands.modelPricesList(cliKey) as Promise<GeneratedCommandResult<ModelPriceSummary[]>>,
  });
}

export async function modelPricesSyncBasellm(force = false) {
  return invokeGeneratedIpc<ModelPricesSyncReport>({
    title: "同步模型价格失败",
    cmd: "model_prices_sync_basellm",
    args: { force },
    invoke: () =>
      commands.modelPricesSyncBasellm(force) as Promise<
        GeneratedCommandResult<ModelPricesSyncReport>
      >,
  });
}

export async function modelPriceAliasesGet() {
  return invokeGeneratedIpc<ModelPriceAliases>({
    title: "读取模型别名规则失败",
    cmd: "model_price_aliases_get",
    invoke: () =>
      commands.modelPriceAliasesGet() as Promise<GeneratedCommandResult<ModelPriceAliases>>,
  });
}

export async function modelPriceAliasesSet(aliases: ModelPriceAliases) {
  return invokeGeneratedIpc<ModelPriceAliases>({
    title: "保存模型别名规则失败",
    cmd: "model_price_aliases_set",
    args: { aliases },
    invoke: () =>
      commands.modelPriceAliasesSet(aliases as any) as Promise<
        GeneratedCommandResult<ModelPriceAliases>
      >,
  });
}
