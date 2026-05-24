import { useEffect } from "react";
import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { CliKey } from "../services/providers/providers";
import {
  getLastModelPricesSync,
  modelPriceAliasesGet,
  modelPriceAliasesSet,
  modelPricesList,
  modelPricesSyncBasellm,
  normalizeModelPriceAliases,
  subscribeModelPricesUpdated,
  validateModelPricesCliKey,
  type ModelPriceAliases,
  type ModelPricesSyncReport,
} from "../services/usage/modelPrices";
import { modelPricesKeys } from "./keys";

export function useModelPricesListQuery(cliKey: CliKey, options?: { enabled?: boolean }) {
  const normalizedCliKey = validateModelPricesCliKey(cliKey);

  return useQuery({
    queryKey: modelPricesKeys.list(normalizedCliKey),
    queryFn: () => modelPricesList(normalizedCliKey),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function useModelPricesTotalCountQuery(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: [...modelPricesKeys.all, "count"] as const,
    queryFn: async () => {
      const [codex, claude, gemini] = await Promise.all([
        modelPricesList("codex"),
        modelPricesList("claude"),
        modelPricesList("gemini"),
      ]);
      return codex.length + claude.length + gemini.length;
    },
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function useModelPriceAliasesQuery(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: modelPricesKeys.aliases(),
    queryFn: () => modelPriceAliasesGet(),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function useModelPriceAliasesSetMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (aliases: ModelPriceAliases) =>
      modelPriceAliasesSet(normalizeModelPriceAliases(aliases)),
    onSuccess: (updated) => {
      queryClient.setQueryData<ModelPriceAliases | null>(modelPricesKeys.aliases(), updated);
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: modelPricesKeys.aliases() });
    },
  });
}

export function useModelPricesSyncBasellmMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { force: boolean }) => modelPricesSyncBasellm(input.force),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: modelPricesKeys.all });
    },
  });
}

export function useModelPricesUpdatedSubscription(
  onUpdated: (snapshot: { report: ModelPricesSyncReport | null; syncedAt: number | null }) => void
) {
  const queryClient = useQueryClient();

  useEffect(() => {
    return subscribeModelPricesUpdated(() => {
      void queryClient.invalidateQueries({ queryKey: modelPricesKeys.all });
      const latest = getLastModelPricesSync();
      onUpdated({
        report: latest.report,
        syncedAt: latest.syncedAt,
      });
    });
  }, [onUpdated, queryClient]);
}

export function isModelPricesSyncNotModified(report: ModelPricesSyncReport | null) {
  return report?.status === "not_modified";
}
