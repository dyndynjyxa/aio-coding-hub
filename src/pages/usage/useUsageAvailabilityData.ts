import { useMemo } from "react";
import type { CliFilterKey } from "../../constants/clis";
import type { CliKey } from "../../services/providers/providers";
import type { UsagePeriod } from "../../services/usage/usage";
import type { CustomDateRangeApplied } from "../../hooks/useCustomDateRange";
import { useRequestLogsListAllQuery } from "../../query/requestLogs";
import { useGatewayCircuitByProviderId } from "../../query/gateway";
import {
  buildAvailabilityTimeline,
  type AvailabilityTimelineData,
} from "../../components/usage/UsageAvailabilityPanel";

const CLIS: CliKey[] = ["claude", "codex", "gemini"];
const REQUEST_LOGS_LIMIT = 2000;

function periodRangeMs(
  period: UsagePeriod,
  customApplied: CustomDateRangeApplied | null
): { startMs: number; endMs: number } {
  const now = Date.now();
  switch (period) {
    case "daily":
      return { startMs: now - 24 * 60 * 60 * 1000, endMs: now };
    case "weekly":
      return { startMs: now - 7 * 24 * 60 * 60 * 1000, endMs: now };
    case "monthly":
      return { startMs: now - 30 * 24 * 60 * 60 * 1000, endMs: now };
    case "allTime":
      return { startMs: now - 90 * 24 * 60 * 60 * 1000, endMs: now };
    case "custom":
      if (customApplied) {
        return {
          startMs: customApplied.startTs * 1000,
          endMs: customApplied.endTs * 1000,
        };
      }
      return { startMs: now - 24 * 60 * 60 * 1000, endMs: now };
  }
}

export function useUsageAvailabilityData({
  enabled,
  cliKey,
  providerId,
  period,
  customApplied,
}: {
  enabled: boolean;
  cliKey: CliFilterKey;
  providerId: number | null;
  period: UsagePeriod;
  customApplied: CustomDateRangeApplied | null;
}) {
  const logsQuery = useRequestLogsListAllQuery(REQUEST_LOGS_LIMIT, {
    enabled,
    refetchIntervalMs: enabled ? 15000 : false,
  });

  const claudeCircuit = useGatewayCircuitByProviderId("claude");
  const codexCircuit = useGatewayCircuitByProviderId("codex");
  const geminiCircuit = useGatewayCircuitByProviderId("gemini");

  const mergedCircuitMap = useMemo(() => {
    return {
      ...claudeCircuit.circuitByProviderId,
      ...codexCircuit.circuitByProviderId,
      ...geminiCircuit.circuitByProviderId,
    };
  }, [
    claudeCircuit.circuitByProviderId,
    codexCircuit.circuitByProviderId,
    geminiCircuit.circuitByProviderId,
  ]);

  const data: AvailabilityTimelineData | null = useMemo(() => {
    const allLogs = logsQuery.data;
    if (!allLogs) return null;

    const { startMs, endMs } = periodRangeMs(period, customApplied);

    const filtered = allLogs.filter((log) => {
      const ts = log.created_at_ms;
      if (ts < startMs || ts > endMs) return false;
      if (cliKey !== "all" && log.cli_key !== cliKey) return false;
      if (providerId != null && log.final_provider_id !== providerId) return false;
      return true;
    });

    return buildAvailabilityTimeline(filtered, mergedCircuitMap, startMs, endMs);
  }, [logsQuery.data, period, customApplied, cliKey, providerId, mergedCircuitMap]);

  return {
    data,
    loading: enabled && logsQuery.isLoading,
    refreshing: enabled && logsQuery.isFetching && !logsQuery.isLoading,
    refetch: () => {
      void logsQuery.refetch();
      for (const cli of CLIS) {
        if (cli === "claude") void claudeCircuit.refetch();
        if (cli === "codex") void codexCircuit.refetch();
        if (cli === "gemini") void geminiCircuit.refetch();
      }
    },
  };
}
