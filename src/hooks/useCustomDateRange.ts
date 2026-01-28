import { useCallback, useMemo, useState } from "react";
import { unixSecondsAtLocalStartOfDay, unixSecondsAtLocalStartOfNextDay } from "../utils/localDate";

export type CustomDateRangeApplied = {
  startDate: string;
  endDate: string;
  startTs: number;
  endTs: number;
};

export type CustomDateRangeBounds = {
  startTs: number | null;
  endTs: number | null;
};

const DEFAULT_INVALID_DATE_MESSAGE = "请选择有效的开始/结束日期";
const DEFAULT_INVALID_RANGE_MESSAGE = "日期范围无效：结束日期必须不早于开始日期";

export function useCustomDateRange(
  period: string,
  options?: { onInvalid?: (message: string) => void }
) {
  const [customStartDate, setCustomStartDate] = useState<string>("");
  const [customEndDate, setCustomEndDate] = useState<string>("");
  const [customApplied, setCustomApplied] = useState<CustomDateRangeApplied | null>(null);

  const bounds = useMemo<CustomDateRangeBounds>(() => {
    if (period !== "custom") return { startTs: null, endTs: null };
    if (!customApplied) return { startTs: null, endTs: null };
    return { startTs: customApplied.startTs, endTs: customApplied.endTs };
  }, [customApplied, period]);

  const showCustomForm = period === "custom";

  const applyCustomRange = useCallback(() => {
    const startTs = unixSecondsAtLocalStartOfDay(customStartDate);
    const endTs = unixSecondsAtLocalStartOfNextDay(customEndDate);
    if (startTs == null || endTs == null) {
      options?.onInvalid?.(DEFAULT_INVALID_DATE_MESSAGE);
      return false;
    }
    if (startTs >= endTs) {
      options?.onInvalid?.(DEFAULT_INVALID_RANGE_MESSAGE);
      return false;
    }
    setCustomApplied({
      startDate: customStartDate,
      endDate: customEndDate,
      startTs,
      endTs,
    });
    return true;
  }, [customEndDate, customStartDate, options]);

  const clearCustomRange = useCallback(() => {
    setCustomApplied(null);
    setCustomStartDate("");
    setCustomEndDate("");
  }, []);

  return {
    customStartDate,
    setCustomStartDate,
    customEndDate,
    setCustomEndDate,
    customApplied,
    bounds,
    showCustomForm,
    applyCustomRange,
    clearCustomRange,
  };
}
