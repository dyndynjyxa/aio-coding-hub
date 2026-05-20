// Usage:
// - Query adapter for `src/services/providerLimitUsage.ts` used by `src/components/home/HomeProviderLimitPanel.tsx`.

import { keepPreviousData, useQuery } from "@tanstack/react-query";
import type { CliKey } from "../services/providers/providers";
import {
  providerLimitUsageV1,
  validateProviderLimitUsageCliKey,
} from "../services/providers/providerLimitUsage";
import { providerLimitUsageKeys } from "./keys";

export function useProviderLimitUsageV1Query(
  cliKey: CliKey | null,
  options?: { enabled?: boolean; refetchIntervalMs?: number | false }
) {
  const normalizedCliKey = validateProviderLimitUsageCliKey(cliKey);

  return useQuery({
    queryKey: providerLimitUsageKeys.list(normalizedCliKey),
    queryFn: () => providerLimitUsageV1(normalizedCliKey),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
    refetchInterval: options?.refetchIntervalMs ?? false,
  });
}
