// Usage: 表格行内 Token 细分展示组件。

import { formatTokensMillions } from "../../utils/chartHelpers";
import { formatPercent, formatInteger } from "../../utils/formatters";

type TokenBreakdownDisplayMode = "full" | "compactRatio";

function formatTokenValue(value: number, compact: boolean) {
  return compact ? formatTokensMillions(value) : formatInteger(value);
}

function computeCacheRatio(totalTokens: number, totalTokensWithCache?: number) {
  if (
    totalTokensWithCache == null ||
    !Number.isFinite(totalTokensWithCache) ||
    totalTokensWithCache <= 0
  ) {
    return null;
  }
  const cacheTokens = Math.max(0, totalTokensWithCache - Math.max(0, totalTokens));
  return cacheTokens / totalTokensWithCache;
}

export function TokenBreakdown({
  totalTokens,
  inputTokens,
  outputTokens,
  totalTokensWithCache,
  displayMode = "full",
  useCompactUnits = false,
}: {
  totalTokens: number;
  inputTokens: number;
  outputTokens: number;
  totalTokensWithCache?: number;
  displayMode?: TokenBreakdownDisplayMode;
  useCompactUnits?: boolean;
}) {
  const cacheRatio = computeCacheRatio(totalTokens, totalTokensWithCache);

  if (displayMode === "compactRatio") {
    return (
      <div className="space-y-0.5">
        <div>{formatTokenValue(totalTokens, useCompactUnits)}</div>
        <div className="text-[10px] leading-4 text-slate-500 dark:text-slate-400">
          含缓存{" "}
          <span className="text-slate-700 dark:text-slate-300">
            {cacheRatio == null ? "—" : formatPercent(cacheRatio)}
          </span>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-0.5">
      <div>{formatTokenValue(totalTokens, useCompactUnits)}</div>
      <div className="text-[10px] leading-4 text-slate-500 dark:text-slate-400">
        输入{" "}
        <span className="text-slate-700 dark:text-slate-300">
          {formatTokenValue(inputTokens, useCompactUnits)}
        </span>
      </div>
      <div className="text-[10px] leading-4 text-slate-500 dark:text-slate-400">
        输出{" "}
        <span className="text-slate-700 dark:text-slate-300">
          {formatTokenValue(outputTokens, useCompactUnits)}
        </span>
      </div>
      {totalTokensWithCache != null && Number.isFinite(totalTokensWithCache) ? (
        <div className="text-[10px] leading-4 text-slate-500 dark:text-slate-400">
          含缓存{" "}
          <span className="text-slate-700 dark:text-slate-300">
            {formatTokenValue(totalTokensWithCache, useCompactUnits)}
          </span>
        </div>
      ) : null}
    </div>
  );
}
