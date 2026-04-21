import { commands } from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";
import type { CliKey } from "./providers";

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

export async function sortModesList() {
  return invokeGeneratedIpc<SortModeSummary[]>({
    title: "读取排序模板失败",
    cmd: "sort_modes_list",
    invoke: () => commands.sortModesList() as Promise<GeneratedCommandResult<SortModeSummary[]>>,
  });
}

export async function sortModeCreate(input: { name: string }) {
  return invokeGeneratedIpc<SortModeSummary>({
    title: "创建排序模板失败",
    cmd: "sort_mode_create",
    args: { name: input.name },
    invoke: () =>
      commands.sortModeCreate(input.name) as Promise<GeneratedCommandResult<SortModeSummary>>,
  });
}

export async function sortModeRename(input: { mode_id: number; name: string }) {
  return invokeGeneratedIpc<SortModeSummary>({
    title: "重命名排序模板失败",
    cmd: "sort_mode_rename",
    args: { modeId: input.mode_id, name: input.name },
    invoke: () =>
      commands.sortModeRename(input.mode_id, input.name) as Promise<
        GeneratedCommandResult<SortModeSummary>
      >,
  });
}

export async function sortModeDelete(input: { mode_id: number }) {
  return invokeGeneratedIpc<boolean>({
    title: "删除排序模板失败",
    cmd: "sort_mode_delete",
    args: { modeId: input.mode_id },
    invoke: () =>
      commands.sortModeDelete(input.mode_id) as Promise<GeneratedCommandResult<boolean>>,
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
  return invokeGeneratedIpc<SortModeActiveRow>({
    title: "设置激活排序模板失败",
    cmd: "sort_mode_active_set",
    args: { cliKey: input.cli_key, modeId: input.mode_id },
    invoke: () =>
      commands.sortModeActiveSet(input.cli_key, input.mode_id) as Promise<
        GeneratedCommandResult<SortModeActiveRow>
      >,
  });
}

export async function sortModeProvidersList(input: { mode_id: number; cli_key: CliKey }) {
  return invokeGeneratedIpc<SortModeProviderRow[]>({
    title: "读取排序模板供应商失败",
    cmd: "sort_mode_providers_list",
    args: { modeId: input.mode_id, cliKey: input.cli_key },
    invoke: () =>
      commands.sortModeProvidersList(input.mode_id, input.cli_key) as Promise<
        GeneratedCommandResult<SortModeProviderRow[]>
      >,
  });
}

export async function sortModeProvidersSetOrder(input: {
  mode_id: number;
  cli_key: CliKey;
  ordered_provider_ids: number[];
}) {
  return invokeGeneratedIpc<SortModeProviderRow[]>({
    title: "更新排序模板供应商顺序失败",
    cmd: "sort_mode_providers_set_order",
    args: {
      modeId: input.mode_id,
      cliKey: input.cli_key,
      orderedProviderIds: input.ordered_provider_ids,
    },
    invoke: () =>
      commands.sortModeProvidersSetOrder(
        input.mode_id,
        input.cli_key,
        input.ordered_provider_ids
      ) as Promise<GeneratedCommandResult<SortModeProviderRow[]>>,
  });
}

export async function sortModeProviderSetEnabled(input: {
  mode_id: number;
  cli_key: CliKey;
  provider_id: number;
  enabled: boolean;
}) {
  return invokeGeneratedIpc<SortModeProviderRow>({
    title: "更新排序模板供应商启用状态失败",
    cmd: "sort_mode_provider_set_enabled",
    args: {
      modeId: input.mode_id,
      cliKey: input.cli_key,
      providerId: input.provider_id,
      enabled: input.enabled,
    },
    invoke: () =>
      commands.sortModeProviderSetEnabled(
        input.mode_id,
        input.cli_key,
        input.provider_id,
        input.enabled
      ) as Promise<GeneratedCommandResult<SortModeProviderRow>>,
  });
}
