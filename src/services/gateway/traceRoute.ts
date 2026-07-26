// Mirrors src-tauri/src/infra/request_logs/queries.rs `route_from_attempts` +
// `has_failover`（row_to_summary: has_failover = route.len() > 1）; keep in sync.
//
// 规则：连续同 provider 的段折叠为一个 hop（同 provider 重试不算切换），
// 真正切换过 provider（hop 数 > 1）才算 failover。判定只看 provider 序列，
// 不看 status——与 Rust 一致（provider_id>0 的 skipped attempt 也计入 hop，
// 见其测试 route_includes_skipped_attempts）。真实输入来自 RealtimeTraceCards
// 构造的 segments，其状态域只有 success/started/failed（skipped outcome 已被
// 映射为 "failed"），不依赖此处对 skipped 的处理。
//
// 注意：单 provider 失败（含重试后成功）不算 failover——这是与旧前端内联
// 判定（`segments.some(failed)`）的行为差异点，目的是与落库后的徽章一致。

export type TraceRouteSegment = {
  provider: string;
  status: string;
};

export function hasFailoverFromSegments(segments: ReadonlyArray<TraceRouteSegment>): boolean {
  let hopCount = 0;
  let lastProvider: string | null = null;
  for (const seg of segments) {
    if (seg.provider === lastProvider) continue;
    lastProvider = seg.provider;
    hopCount += 1;
    if (hopCount > 1) return true;
  }
  return false;
}
