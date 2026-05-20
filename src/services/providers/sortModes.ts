import { commands } from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";
import { validateProviderCliKey, type CliKey } from "./providers";

export const MAX_SORT_MODE_NAME_CHARS = 32;
export const MAX_SORT_MODE_PROVIDER_IDS = 512;

export type SortModeSummary = {
  id: number;
  name: string;
  created_at: number;
  updated_at: number;
};

export type SortModeActiveRow = {
  cli_key: CliKey;
  mode_id: number | null;
  updated_at: number;
};

export type SortModeProviderRow = {
  provider_id: number;
  enabled: boolean;
};

function normalizeSortModeName(name: string) {
  const trimmed = name.trim();
  if (!trimmed) {
    throw new Error("SEC_INVALID_INPUT: mode name is required");
  }
  if ([...trimmed].length > MAX_SORT_MODE_NAME_CHARS) {
    throw new Error(
      `SEC_INVALID_INPUT: mode name is too long (max ${MAX_SORT_MODE_NAME_CHARS} chars)`
    );
  }
  if (trimmed.toLowerCase() === "default" || trimmed === "默认") {
    throw new Error("SEC_INVALID_INPUT: mode name is reserved");
  }
  return trimmed;
}

function validatePositiveId(field: string, value: number) {
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new Error(`SEC_INVALID_INPUT: invalid ${field}=${value}`);
  }
}

export function validateSortModeId(modeId: number): number {
  validatePositiveId("modeId", modeId);
  return modeId;
}

function validateOrderedProviderIds(orderedProviderIds: number[]) {
  if (orderedProviderIds.length > MAX_SORT_MODE_PROVIDER_IDS) {
    throw new Error(
      `SEC_INVALID_INPUT: orderedProviderIds must contain at most ${MAX_SORT_MODE_PROVIDER_IDS} entries`
    );
  }

  const seen = new Set<number>();
  for (const providerId of orderedProviderIds) {
    validatePositiveId("providerId", providerId);
    if (seen.has(providerId)) {
      throw new Error(`SEC_INVALID_INPUT: duplicate providerId=${providerId}`);
    }
    seen.add(providerId);
  }
}

export async function sortModesList() {
  return invokeGeneratedIpc<SortModeSummary[]>({
    title: "读取排序模板失败",
    cmd: "sort_modes_list",
    invoke: () => commands.sortModesList() as Promise<GeneratedCommandResult<SortModeSummary[]>>,
  });
}

export async function sortModeCreate(input: { name: string }) {
  const name = normalizeSortModeName(input.name);

  return invokeGeneratedIpc<SortModeSummary>({
    title: "创建排序模板失败",
    cmd: "sort_mode_create",
    args: { name },
    invoke: () => commands.sortModeCreate(name) as Promise<GeneratedCommandResult<SortModeSummary>>,
  });
}

export async function sortModeRename(input: { mode_id: number; name: string }) {
  const modeId = validateSortModeId(input.mode_id);
  const name = normalizeSortModeName(input.name);

  return invokeGeneratedIpc<SortModeSummary>({
    title: "重命名排序模板失败",
    cmd: "sort_mode_rename",
    args: { modeId, name },
    invoke: () =>
      commands.sortModeRename(modeId, name) as Promise<GeneratedCommandResult<SortModeSummary>>,
  });
}

export async function sortModeDelete(input: { mode_id: number }) {
  const modeId = validateSortModeId(input.mode_id);

  return invokeGeneratedIpc<boolean>({
    title: "删除排序模板失败",
    cmd: "sort_mode_delete",
    args: { modeId },
    invoke: () => commands.sortModeDelete(modeId) as Promise<GeneratedCommandResult<boolean>>,
  });
}

export async function sortModeActiveList() {
  return invokeGeneratedIpc<SortModeActiveRow[]>({
    title: "读取激活排序模板失败",
    cmd: "sort_mode_active_list",
    invoke: () =>
      commands.sortModeActiveList() as Promise<GeneratedCommandResult<SortModeActiveRow[]>>,
  });
}

export async function sortModeActiveSet(input: { cli_key: CliKey; mode_id: number | null }) {
  const cliKey = validateProviderCliKey(input.cli_key);
  const modeId = input.mode_id == null ? null : validateSortModeId(input.mode_id);

  return invokeGeneratedIpc<SortModeActiveRow>({
    title: "设置激活排序模板失败",
    cmd: "sort_mode_active_set",
    args: { cliKey, modeId },
    invoke: () =>
      commands.sortModeActiveSet(cliKey, modeId) as Promise<
        GeneratedCommandResult<SortModeActiveRow>
      >,
  });
}

export async function sortModeProvidersList(input: { mode_id: number; cli_key: CliKey }) {
  const cliKey = validateProviderCliKey(input.cli_key);
  const modeId = validateSortModeId(input.mode_id);

  return invokeGeneratedIpc<SortModeProviderRow[]>({
    title: "读取排序模板供应商失败",
    cmd: "sort_mode_providers_list",
    args: { modeId, cliKey },
    invoke: () =>
      commands.sortModeProvidersList(modeId, cliKey) as Promise<
        GeneratedCommandResult<SortModeProviderRow[]>
      >,
  });
}

export async function sortModeProvidersSetOrder(input: {
  mode_id: number;
  cli_key: CliKey;
  ordered_provider_ids: number[];
}) {
  const cliKey = validateProviderCliKey(input.cli_key);
  const modeId = validateSortModeId(input.mode_id);
  validateOrderedProviderIds(input.ordered_provider_ids);

  return invokeGeneratedIpc<SortModeProviderRow[]>({
    title: "更新排序模板供应商顺序失败",
    cmd: "sort_mode_providers_set_order",
    args: {
      modeId,
      cliKey,
      orderedProviderIds: input.ordered_provider_ids,
    },
    invoke: () =>
      commands.sortModeProvidersSetOrder(modeId, cliKey, input.ordered_provider_ids) as Promise<
        GeneratedCommandResult<SortModeProviderRow[]>
      >,
  });
}

export async function sortModeProviderSetEnabled(input: {
  mode_id: number;
  cli_key: CliKey;
  provider_id: number;
  enabled: boolean;
}) {
  const cliKey = validateProviderCliKey(input.cli_key);
  const modeId = validateSortModeId(input.mode_id);
  validatePositiveId("providerId", input.provider_id);

  return invokeGeneratedIpc<SortModeProviderRow>({
    title: "更新排序模板供应商启用状态失败",
    cmd: "sort_mode_provider_set_enabled",
    args: {
      modeId,
      cliKey,
      providerId: input.provider_id,
      enabled: input.enabled,
    },
    invoke: () =>
      commands.sortModeProviderSetEnabled(
        modeId,
        cliKey,
        input.provider_id,
        input.enabled
      ) as Promise<GeneratedCommandResult<SortModeProviderRow>>,
  });
}
