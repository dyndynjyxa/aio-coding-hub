import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  createSettingsSetInput,
  settingsGet,
  settingsSet,
  type AppSettingsPatch,
  type AppSettings,
  type SettingsMutationResult,
  type SettingsSetInput,
} from "../services/settings/settings";
import { settingsCircuitBreakerNoticeSet } from "../services/settings/settingsCircuitBreakerNotice";
import { settingsCodexSessionIdCompletionSet } from "../services/settings/settingsCodexSessionIdCompletion";
import {
  settingsGatewayRectifierSet,
  type GatewayRectifierSettingsPatch,
} from "../services/settings/settingsGatewayRectifier";
import { gatewayKeys, settingsKeys } from "./keys";

export const SETTINGS_READONLY_MESSAGE =
  "设置文件读取失败，已进入只读保护。请先修复或恢复 settings.json 后刷新页面。";

export type SettingsReadProtection = {
  settingsReadErrorMessage: string | null;
  settingsWriteBlocked: boolean;
};

export function getSettingsReadProtection(query: {
  data?: AppSettings | null;
  isError?: boolean;
}): SettingsReadProtection {
  if (query.isError) {
    return {
      settingsReadErrorMessage: SETTINGS_READONLY_MESSAGE,
      settingsWriteBlocked: true,
    };
  }

  if (query.data != null) {
    return {
      settingsReadErrorMessage: null,
      settingsWriteBlocked: false,
    };
  }

  return {
    settingsReadErrorMessage: null,
    settingsWriteBlocked: false,
  };
}

export function useSettingsQuery(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: settingsKeys.get(),
    queryFn: () => settingsGet(),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export { type SettingsSetInput } from "../services/settings/settings";
export { type SettingsMutationResult } from "../services/settings/settings";

export function useSettingsSetMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: SettingsSetInput) => settingsSet(input),
    onSuccess: (result) => syncSettingsMutationCaches(queryClient, result),
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: settingsKeys.get() });
    },
  });
}

function syncSettingsMutationCaches(
  queryClient: ReturnType<typeof useQueryClient>,
  result: SettingsMutationResult | null | undefined
) {
  if (!result) return;
  queryClient.setQueryData<AppSettings | null>(settingsKeys.get(), result.settings);
  queryClient.setQueryData(gatewayKeys.status(), result.runtime.gateway_status);
}

export function useSettingsPatchMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (patch: AppSettingsPatch) => {
      const current = queryClient.getQueryData<AppSettings | null>(settingsKeys.get());
      if (!current) {
        throw new Error("SETTINGS_NOT_READY: settings query cache is empty");
      }
      return settingsSet(createSettingsSetInput(current, patch));
    },
    onSuccess: (result) => syncSettingsMutationCaches(queryClient, result),
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: settingsKeys.get() });
    },
  });
}

export function useSettingsGatewayRectifierSetMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: GatewayRectifierSettingsPatch) => settingsGatewayRectifierSet(input),
    onSuccess: (updated) => {
      if (!updated) return;
      queryClient.setQueryData<AppSettings | null>(settingsKeys.get(), updated);
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: settingsKeys.get() });
    },
  });
}

export function useSettingsCircuitBreakerNoticeSetMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (enable: boolean) => settingsCircuitBreakerNoticeSet(enable),
    onSuccess: (updated) => {
      if (!updated) return;
      queryClient.setQueryData<AppSettings | null>(settingsKeys.get(), updated);
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: settingsKeys.get() });
    },
  });
}

export function useSettingsCodexSessionIdCompletionSetMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (enable: boolean) => settingsCodexSessionIdCompletionSet(enable),
    onSuccess: (updated) => {
      if (!updated) return;
      queryClient.setQueryData<AppSettings | null>(settingsKeys.get(), updated);
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: settingsKeys.get() });
    },
  });
}
