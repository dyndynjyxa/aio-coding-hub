const LEGACY_CREATE_SHARE_TARGET = 0.9;

export const CACHE_ANOMALY_MONITOR_WINDOW_MINUTES = 60;
export const CACHE_ANOMALY_MONITOR_BASELINE_MINUTES = 45;
export const CACHE_ANOMALY_MONITOR_RECENT_MINUTES = 15;
export const CACHE_ANOMALY_MONITOR_COLD_START_MINUTES = 10;

export const CACHE_ANOMALY_MONITOR_THRESHOLDS = {
  baselineDenomTokensMin: 10_000,
  recentDenomTokensMin: 3_000,
  baselineSuccessRequestsMin: 30,
  recentSuccessRequestsMin: 10,
  coldRecentDenomTokensMin: 2_000,
  coldRecentSuccessRequestsMin: 5,
  baselineHitRateMin: 0.05,
  dropRatioMin: 0.25,
  dropAbsMin: 0.05,
  createShareMin: LEGACY_CREATE_SHARE_TARGET / (1 + LEGACY_CREATE_SHARE_TARGET),
  createReadImbalanceMin: 3,
} as const;

export const CACHE_ANOMALY_MONITOR_NON_CACHING_MODEL_KEYWORDS = ["haiku"] as const;

function formatPercent(value: number): string {
  const pct = value * 100;
  const rounded = Math.round(pct * 10) / 10;
  if (Number.isInteger(rounded)) return `${rounded}%`;
  return `${rounded.toFixed(1)}%`;
}

function windowLabel(minutes: number): string {
  if (minutes % 60 === 0) return `${minutes / 60} 小时`;
  return `${minutes} 分钟`;
}

const thresholds = CACHE_ANOMALY_MONITOR_THRESHOLDS;

export const CACHE_ANOMALY_MONITOR_GUIDE_COPY = {
  overview: `近 ${windowLabel(CACHE_ANOMALY_MONITOR_WINDOW_MINUTES)}滑窗（按 Provider + Model）监测缓存读取/创建异常。仅 Claude / Codex；命中后写入控制台并发送系统通知。`,
  trigger: `触发条件：命中率断崖式下降（最近 ${CACHE_ANOMALY_MONITOR_RECENT_MINUTES}m vs 前 ${CACHE_ANOMALY_MONITOR_BASELINE_MINUTES}m）；或创建异常（创建但读取为 0 / 创建占比过高 / 创建显著高于读取）。`,
  metric:
    "口径：命中率=读取 /（有效输入 + 创建 + 读取）。有效输入：Codex 做 input-cache_read 纠偏；Claude 原样。",
  coldStart: `冷启动：开启后前 ${CACHE_ANOMALY_MONITOR_COLD_START_MINUTES} 分钟也会评估创建异常（不依赖 ${CACHE_ANOMALY_MONITOR_BASELINE_MINUTES}m 基线）。`,
  nonCachingModel: "Haiku：模型名包含 haiku 时默认不采集（该类模型不创建缓存）。",
  thresholds: `门槛（默认）：冷启动 token≥${thresholds.coldRecentDenomTokensMin} 且成功请求≥${thresholds.coldRecentSuccessRequestsMin}；稳定期：基线 token≥${thresholds.baselineDenomTokensMin} 且成功请求≥${thresholds.baselineSuccessRequestsMin}；最近 token≥${thresholds.recentDenomTokensMin} 且成功请求≥${thresholds.recentSuccessRequestsMin}；基线命中率≥${formatPercent(thresholds.baselineHitRateMin)}；创建占比≥${formatPercent(thresholds.createShareMin)}（等价旧口径 ${formatPercent(LEGACY_CREATE_SHARE_TARGET)}）或 创建/读取≥${thresholds.createReadImbalanceMin}。* token 不是请求数`,
} as const;
