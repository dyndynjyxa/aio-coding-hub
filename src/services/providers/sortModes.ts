import { invokeService } from "../invokeServiceCommand";
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
  return invokeService<SortModeSummary[]>("读取排序模板失败", "sort_modes_list");
}

export async function sortModeCreate(input: { name: string }) {
  return invokeService<SortModeSummary>("创建排序模板失败", "sort_mode_create", {
    name: input.name,
  });
}

export async function sortModeRename(input: { mode_id: number; name: string }) {
  return invokeService<SortModeSummary>("重命名排序模板失败", "sort_mode_rename", {
    modeId: input.mode_id,
    name: input.name,
  });
}

export async function sortModeDelete(input: { mode_id: number }) {
  return invokeService<boolean>("删除排序模板失败", "sort_mode_delete", {
    modeId: input.mode_id,
  });
}

export async function sortModeActiveList() {
  return invokeService<SortModeActiveRow[]>("读取激活排序模板失败", "sort_mode_active_list");
}

export async function sortModeActiveSet(input: { cli_key: CliKey; mode_id: number | null }) {
  return invokeService<SortModeActiveRow>("设置激活排序模板失败", "sort_mode_active_set", {
    cliKey: input.cli_key,
    modeId: input.mode_id,
  });
}

export async function sortModeProvidersList(input: { mode_id: number; cli_key: CliKey }) {
  return invokeService<SortModeProviderRow[]>(
    "读取排序模板供应商失败",
    "sort_mode_providers_list",
    {
      modeId: input.mode_id,
      cliKey: input.cli_key,
    }
  );
}

export async function sortModeProvidersSetOrder(input: {
  mode_id: number;
  cli_key: CliKey;
  ordered_provider_ids: number[];
}) {
  return invokeService<SortModeProviderRow[]>(
    "更新排序模板供应商顺序失败",
    "sort_mode_providers_set_order",
    {
      modeId: input.mode_id,
      cliKey: input.cli_key,
      orderedProviderIds: input.ordered_provider_ids,
    }
  );
}

export async function sortModeProviderSetEnabled(input: {
  mode_id: number;
  cli_key: CliKey;
  provider_id: number;
  enabled: boolean;
}) {
  return invokeService<SortModeProviderRow>(
    "更新排序模板供应商启用状态失败",
    "sort_mode_provider_set_enabled",
    {
      modeId: input.mode_id,
      cliKey: input.cli_key,
      providerId: input.provider_id,
      enabled: input.enabled,
    }
  );
}
