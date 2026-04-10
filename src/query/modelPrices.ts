import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { CliKey } from "../services/providers/providers";
import {
  modelPriceAliasesGet,
  modelPriceAliasesSet,
  modelPricesList,
  modelPricesSyncBasellm,
  type ModelPriceAliases,
  type ModelPricesSyncReport,
} from "../services/usage/modelPrices";
import { modelPricesKeys } from "./keys";

export function useModelPricesListQuery(cliKey: CliKey, options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: modelPricesKeys.list(cliKey),
    queryFn: () => modelPricesList(cliKey),
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
      if (!codex || !claude || !gemini) return null;
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
    mutationFn: (aliases: ModelPriceAliases) => modelPriceAliasesSet(aliases),
    onSuccess: (updated) => {
      if (!updated) return;
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
    onSuccess: (report) => {
      if (!report) return;
      queryClient.invalidateQueries({ queryKey: modelPricesKeys.all });
    },
  });
}

export function isModelPricesSyncNotModified(report: ModelPricesSyncReport | null) {
  return report?.status === "not_modified";
}
