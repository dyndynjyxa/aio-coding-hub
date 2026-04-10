import { useCallback, useMemo, useState } from "react";
import { toast } from "sonner";
import type { CliFilterKey } from "../../constants/clis";
import { useCustomDateRange } from "../../hooks/useCustomDateRange";
import type { UsagePeriod } from "../../services/usage/usage";

export function useUsagePageFiltersState() {
  const [period, setPeriod] = useState<UsagePeriod>("daily");
  const [cliKey, setCliKey] = useState<CliFilterKey>("all");

  const onInvalid = useCallback((message: string) => toast(message), []);
  const customRangeOptions = useMemo(() => ({ onInvalid }), [onInvalid]);
  const customRange = useCustomDateRange(period, customRangeOptions);

  return { period, setPeriod, cliKey, setCliKey, ...customRange };
}
