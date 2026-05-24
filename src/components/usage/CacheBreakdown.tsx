// Usage: 表格行内缓存细分展示组件。

import { formatInteger, formatPercent } from "../../utils/formatters";
import { computeCacheHitRate } from "../../utils/cacheRateMetrics";

export function CacheBreakdown({
  inputTokens,
  cacheCreationInputTokens,
  cacheReadInputTokens,
}: {
  inputTokens: number;
  cacheCreationInputTokens: number;
  cacheReadInputTokens: number;
}) {
  const hitRate = computeCacheHitRate(inputTokens, cacheCreationInputTokens, cacheReadInputTokens);

  return (
    <div className="space-y-0.5 text-[10px] leading-4">
      <div className="text-muted-foreground">
        创建{" "}
        <span className="text-secondary-foreground">{formatInteger(cacheCreationInputTokens)}</span>
      </div>
      <div className="text-muted-foreground">
        读取{" "}
        <span className="text-secondary-foreground">{formatInteger(cacheReadInputTokens)}</span>
      </div>
      <div className="text-muted-foreground">
        命中率 <span className="text-secondary-foreground">{formatPercent(hitRate, 2)}</span>
      </div>
    </div>
  );
}
