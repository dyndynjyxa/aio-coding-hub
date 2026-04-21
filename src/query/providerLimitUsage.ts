// Usage:
// - Query adapter for `src/services/providerLimitUsage.ts` used by `src/components/home/HomeProviderLimitPanel.tsx`.

import { keepPreviousData, useQuery } from "@tanstack/react-query";
import type { CliKey } from "../services/providers/providers";
import { providerLimitUsageV1 } from "../services/providers/providerLimitUsage";
import { providerLimitUsageKeys } from "./keys";

export function useProviderLimitUsageV1Query(
  cliKey: CliKey | null,
  options?: { enabled?: boolean; refetchIntervalMs?: number | false }
) {
  return useQuery({
    queryKey: providerLimitUsageKeys.list(cliKey),
    queryFn: () => providerLimitUsageV1(cliKey),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
    refetchInterval: options?.refetchIntervalMs ?? false,
  });
}
