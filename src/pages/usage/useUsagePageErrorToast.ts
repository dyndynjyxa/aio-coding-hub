import { useEffect, useRef } from "react";
import { toast } from "sonner";
import type { UsageTableTab } from "./types";

export function useUsagePageErrorToast(errorText: string | null, tableTab: UsageTableTab) {
  const lastRef = useRef<string | null>(null);

  useEffect(() => {
    if (!errorText) {
      lastRef.current = null;
      return;
    }
    const key = `${tableTab}:${errorText}`;
    if (lastRef.current === key) return;

    lastRef.current = key;
    const label =
      tableTab === "cacheTrend" ? "缓存走势" : tableTab === "metricsTrend" ? "指标走势" : "用量";
    toast(`加载${label}失败：请重试（详情见页面错误信息）`);
  }, [errorText, tableTab]);
}
