// Usage: 表格行内 Token 细分展示组件。

import { formatInteger } from "../../utils/formatters";

export function TokenBreakdown({
  totalTokens,
  inputTokens,
  outputTokens,
  totalTokensWithCache,
}: {
  totalTokens: number;
  inputTokens: number;
  outputTokens: number;
  totalTokensWithCache?: number;
}) {
  return (
    <div className="space-y-0.5">
      <div>{formatInteger(totalTokens)}</div>
      <div className="text-[10px] leading-4 text-muted-foreground">
        输入 <span className="text-secondary-foreground">{formatInteger(inputTokens)}</span>
      </div>
      <div className="text-[10px] leading-4 text-muted-foreground">
        输出 <span className="text-secondary-foreground">{formatInteger(outputTokens)}</span>
      </div>
      {totalTokensWithCache != null && Number.isFinite(totalTokensWithCache) ? (
        <div className="text-[10px] leading-4 text-muted-foreground">
          含缓存{" "}
          <span className="text-secondary-foreground">{formatInteger(totalTokensWithCache)}</span>
        </div>
      ) : null}
    </div>
  );
}
