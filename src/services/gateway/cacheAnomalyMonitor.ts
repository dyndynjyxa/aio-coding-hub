import { useSyncExternalStore } from "react";
import { logToConsole } from "../consoleLog";
import { noticeSend, type NoticeSendParams } from "../notification/notice";
import type {
  GatewayAttempt,
  GatewayRequestEvent,
  GatewayRequestStartEvent,
} from "./gatewayEvents";
import { computeCacheHitRateDenomTokens } from "../../utils/cacheRateMetrics";
import {
  CACHE_ANOMALY_MONITOR_BASELINE_MINUTES,
  CACHE_ANOMALY_MONITOR_COLD_START_MINUTES,
  CACHE_ANOMALY_MONITOR_NON_CACHING_MODEL_KEYWORDS,
  CACHE_ANOMALY_MONITOR_RECENT_MINUTES,
  CACHE_ANOMALY_MONITOR_THRESHOLDS,
  CACHE_ANOMALY_MONITOR_WINDOW_MINUTES,
} from "./cacheAnomalyMonitorConfig";
import { emitListenerSnapshot } from "../../utils/listeners";

const MINUTE_MS = 60_000;
const WINDOW_MINUTES = CACHE_ANOMALY_MONITOR_WINDOW_MINUTES;
const BASELINE_MINUTES = CACHE_ANOMALY_MONITOR_BASELINE_MINUTES;
const RECENT_MINUTES = CACHE_ANOMALY_MONITOR_RECENT_MINUTES;
const COLD_START_MINUTES = CACHE_ANOMALY_MONITOR_COLD_START_MINUTES;

const EVAL_INTERVAL_MS = 60_000;
const ALERT_DEDUP_MS = 15 * MINUTE_MS;
const COLD_START_WINDOW_MS = COLD_START_MINUTES * MINUTE_MS;

const SAMPLE_RETENTION_MINUTES = 75;
const MAX_SAMPLE_BUCKETS_PER_GROUP = SAMPLE_RETENTION_MINUTES + 1;
const TRACE_MODEL_TTL_MS = 10 * MINUTE_MS;
const MAX_TRACE_MODEL_ENTRIES = 2_000;
const MAX_GROUP_ENTRIES = 500;

const THRESHOLDS = CACHE_ANOMALY_MONITOR_THRESHOLDS;

// Some models may naturally not create caches (e.g. Haiku). For these models we skip monitoring entirely.
const NON_CACHING_MODEL_KEYWORDS = CACHE_ANOMALY_MONITOR_NON_CACHING_MODEL_KEYWORDS;

function isNonCachingModel(cliKey: SupportedCliKey, model: string): boolean {
  if (cliKey !== "claude") return false;
  const m = model.toLowerCase();
  return NON_CACHING_MODEL_KEYWORDS.some((keyword) => m.includes(keyword));
}

type SupportedCliKey = "claude" | "codex";

type Listener = () => void;

type EnabledSnapshot = {
  enabled: boolean;
};

let enabled = false;
let enabledSnapshot: EnabledSnapshot = { enabled };
const enabledListeners = new Set<Listener>();

function emitEnabled() {
  emitListenerSnapshot(
    enabledListeners,
    (listener) => listener(),
    (error) => logToConsole("warn", "缓存异常监控状态订阅处理失败", { error: String(error) })
  );
}

function setEnabledInternal(next: boolean) {
  enabled = next;
  enabledSnapshot = { enabled: next };
  emitEnabled();
}

export function getCacheAnomalyMonitorEnabled(): boolean {
  return enabled;
}

export function setCacheAnomalyMonitorEnabled(next: boolean) {
  const normalized = next === true;
  if (enabled === normalized) return;

  resetState();
  state.enabledAtMs = normalized ? Date.now() : 0;
  setEnabledInternal(normalized);
}

export function subscribeCacheAnomalyMonitorEnabled(listener: Listener) {
  enabledListeners.add(listener);
  return () => enabledListeners.delete(listener);
}

export function useCacheAnomalyMonitorEnabled(): boolean {
  return useSyncExternalStore(
    subscribeCacheAnomalyMonitorEnabled,
    () => enabledSnapshot.enabled,
    () => enabledSnapshot.enabled
  );
}

type MinuteBucket = {
  minute: number;
  denomTokens: number;
  cacheReadTokens: number;
  cacheCreateTokens: number;
  successRequests: number;
};

type WindowSums = {
  denomTokens: number;
  cacheReadTokens: number;
  cacheCreateTokens: number;
  successRequests: number;
};

type Sample = {
  minute: number;
  denomTokens: number;
  cacheReadTokens: number;
  cacheCreateTokens: number;
  successRequest: number;
};

type GroupKeyParts = {
  cliKey: SupportedCliKey;
  providerId: number;
  providerName: string;
  model: string;
};

type GroupState = {
  key: string;
  parts: GroupKeyParts;
  ring: MinuteRing;
  samples: Sample[];
  lastSeenMinute: number;
  lastAlertAtMs: number | null;
};

class MinuteRing {
  private buckets: MinuteBucket[];

  constructor() {
    this.buckets = Array.from({ length: WINDOW_MINUTES }, () => ({
      minute: Number.MIN_SAFE_INTEGER,
      denomTokens: 0,
      cacheReadTokens: 0,
      cacheCreateTokens: 0,
      successRequests: 0,
    }));
  }

  add(minute: number, sample: Omit<Sample, "tsMs">) {
    const idx = mod(minute, WINDOW_MINUTES);
    const bucket = this.buckets[idx];
    if (bucket.minute !== minute) {
      bucket.minute = minute;
      bucket.denomTokens = 0;
      bucket.cacheReadTokens = 0;
      bucket.cacheCreateTokens = 0;
      bucket.successRequests = 0;
    }

    bucket.denomTokens += sample.denomTokens;
    bucket.cacheReadTokens += sample.cacheReadTokens;
    bucket.cacheCreateTokens += sample.cacheCreateTokens;
    bucket.successRequests += sample.successRequest;
  }

  sumRange(minStart: number, minEnd: number): WindowSums {
    const out: WindowSums = {
      denomTokens: 0,
      cacheReadTokens: 0,
      cacheCreateTokens: 0,
      successRequests: 0,
    };

    for (const bucket of this.buckets) {
      if (bucket.minute < minStart || bucket.minute > minEnd) continue;
      out.denomTokens += bucket.denomTokens;
      out.cacheReadTokens += bucket.cacheReadTokens;
      out.cacheCreateTokens += bucket.cacheCreateTokens;
      out.successRequests += bucket.successRequests;
    }
    return out;
  }
}

type TraceModelEntry =
  | { ignore: true; seenAtMs: number }
  | { ignore: false; model: string; seenAtMs: number };

const state = {
  enabledAtMs: enabled ? Date.now() : 0,
  traceModels: new Map<string, TraceModelEntry>(),
  groups: new Map<string, GroupState>(),
  lastEvalMs: 0,
  lastSelfCheckFailureAtMs: 0,
};

function resetState() {
  state.traceModels.clear();
  state.groups.clear();
  state.lastEvalMs = 0;
  state.enabledAtMs = 0;
}

export function __testGetCacheAnomalyMonitorStateSizes() {
  const groups = Array.from(state.groups.values());
  const sampleBuckets = groups.reduce((sum, group) => sum + group.samples.length, 0);
  const maxSampleBucketsPerGroup = Math.max(0, ...groups.map((group) => group.samples.length));

  return {
    traceModels: state.traceModels.size,
    groups: state.groups.size,
    sampleBuckets,
    maxSampleBucketsPerGroup,
  };
}

function mod(value: number, base: number): number {
  const r = value % base;
  return r < 0 ? r + base : r;
}

function normalizeTokenCount(value: number | null | undefined): number {
  if (value == null) return 0;
  const n = Math.floor(Number(value));
  if (!Number.isFinite(n)) return 0;
  return n < 0 ? 0 : n;
}

function normalizeModelName(value: unknown): string {
  const raw = typeof value === "string" ? value : "";
  const trimmed = raw.trim();
  if (!trimmed) return "Unknown";
  return trimmed.length > 200 ? trimmed.slice(0, 200) : trimmed;
}

function isSupportedCliKey(value: unknown): value is SupportedCliKey {
  return value === "claude" || value === "codex";
}

function isSuccessRequest(payload: GatewayRequestEvent): boolean {
  const status = payload.status;
  if (status == null) return false;
  if (status < 200 || status >= 300) return false;
  return !payload.error_code;
}

function formatPct(value: number): string {
  if (!Number.isFinite(value)) return "—";
  return `${Math.round(value * 10000) / 100}%`;
}

function formatRatio(value: number): string {
  if (!Number.isFinite(value)) return "—";
  return `${Math.round(value * 100) / 100}x`;
}

function computeObserveMetrics(observe: WindowSums): {
  createShare: number;
  createReadRatio: number;
} {
  const createShare =
    observe.denomTokens > 0 ? observe.cacheCreateTokens / observe.denomTokens : NaN;
  const createReadRatio =
    observe.cacheReadTokens > 0 ? observe.cacheCreateTokens / observe.cacheReadTokens : NaN;
  return { createShare, createReadRatio };
}

async function safeNoticeSend(params: NoticeSendParams): Promise<boolean> {
  try {
    return await noticeSend(params);
  } catch (err) {
    logToConsole("error", "发送系统通知失败", {
      title: params.title ?? null,
      error: String(err),
    });
    return false;
  }
}

function pickFinalProvider(attempts: GatewayAttempt[] | null | undefined): GatewayAttempt | null {
  const list = attempts ?? [];
  if (list.length === 0) return null;

  for (let i = list.length - 1; i >= 0; i -= 1) {
    if (list[i].outcome === "success") return list[i];
  }
  return list[list.length - 1] ?? null;
}

function extractSample(payload: GatewayRequestEvent, nowMs: number): Sample {
  const cacheReadTokens = normalizeTokenCount(payload.cache_read_input_tokens ?? null);

  const createRaw = normalizeTokenCount(payload.cache_creation_input_tokens ?? null);
  const create5m = normalizeTokenCount(payload.cache_creation_5m_input_tokens ?? null);
  const create1h = normalizeTokenCount(payload.cache_creation_1h_input_tokens ?? null);
  const cacheCreateTokens = create5m + create1h > 0 ? create5m + create1h : createRaw;

  // Backend-computed effective input (single source of truth shared with usage stats).
  const effectiveInput = normalizeTokenCount(payload.effective_input_tokens ?? null);
  const denomTokens = computeCacheHitRateDenomTokens(
    effectiveInput,
    cacheCreateTokens,
    cacheReadTokens
  );

  const successRequest: 0 | 1 = isSuccessRequest(payload) ? 1 : 0;

  return {
    minute: Math.floor(nowMs / MINUTE_MS),
    denomTokens: Math.max(denomTokens, 0),
    cacheReadTokens: Math.max(cacheReadTokens, 0),
    cacheCreateTokens: Math.max(cacheCreateTokens, 0),
    successRequest,
  };
}

function pruneTraceModels(nowMs: number) {
  const cutoff = nowMs - TRACE_MODEL_TTL_MS;
  for (const [traceId, entry] of state.traceModels) {
    if (entry.seenAtMs < cutoff) state.traceModels.delete(traceId);
  }

  const overflow = state.traceModels.size - MAX_TRACE_MODEL_ENTRIES;
  if (overflow <= 0) return;

  let removed = 0;
  for (const traceId of state.traceModels.keys()) {
    state.traceModels.delete(traceId);
    removed += 1;
    if (removed >= overflow) break;
  }
}

function pruneGroups(minuteNow: number) {
  const staleBeforeMinute = minuteNow - WINDOW_MINUTES;
  for (const [key, group] of state.groups) {
    if (group.lastSeenMinute < staleBeforeMinute) {
      state.groups.delete(key);
    }
  }

  while (state.groups.size > MAX_GROUP_ENTRIES) {
    evictOldestGroup();
  }
}

function evictOldestGroup() {
  let oldestKey: string | null = null;
  let oldestMinute = Number.POSITIVE_INFINITY;

  for (const [key, group] of state.groups) {
    if (group.lastSeenMinute < oldestMinute) {
      oldestKey = key;
      oldestMinute = group.lastSeenMinute;
    }
  }

  if (oldestKey != null) {
    state.groups.delete(oldestKey);
  }
}

function pruneSamples(samples: Sample[], minMinuteInclusive: number): Sample[] {
  const kept = samples.filter((s) => s.minute >= minMinuteInclusive);
  if (kept.length <= MAX_SAMPLE_BUCKETS_PER_GROUP) return kept;

  return kept.sort((a, b) => a.minute - b.minute).slice(kept.length - MAX_SAMPLE_BUCKETS_PER_GROUP);
}

function addSampleBucket(samples: Sample[], sample: Sample, minMinuteInclusive: number): Sample[] {
  const bucket = samples.find((s) => s.minute === sample.minute);
  if (bucket) {
    bucket.denomTokens += sample.denomTokens;
    bucket.cacheReadTokens += sample.cacheReadTokens;
    bucket.cacheCreateTokens += sample.cacheCreateTokens;
    bucket.successRequest += sample.successRequest;
    return pruneSamples(samples, minMinuteInclusive);
  }

  samples.push({ ...sample });
  return pruneSamples(samples, minMinuteInclusive);
}

function slowSumSamples(samples: Sample[], minStart: number, minEnd: number): WindowSums {
  const out: WindowSums = {
    denomTokens: 0,
    cacheReadTokens: 0,
    cacheCreateTokens: 0,
    successRequests: 0,
  };
  for (const s of samples) {
    if (s.minute < minStart || s.minute > minEnd) continue;
    out.denomTokens += s.denomTokens;
    out.cacheReadTokens += s.cacheReadTokens;
    out.cacheCreateTokens += s.cacheCreateTokens;
    out.successRequests += s.successRequest;
  }
  return out;
}

function sumsEqual(a: WindowSums, b: WindowSums): boolean {
  return (
    a.denomTokens === b.denomTokens &&
    a.cacheReadTokens === b.cacheReadTokens &&
    a.cacheCreateTokens === b.cacheCreateTokens &&
    a.successRequests === b.successRequests
  );
}

type GroupEval = {
  group: GroupState;
  baseline: WindowSums;
  recent: WindowSums;
  baselineHitRate: number;
  recentHitRate: number;
  observe: WindowSums;
  observeHitRate: number;
  observeMinutes: number;
  coldStart: boolean;
  totalDenomTokens: number;
};

function shouldAlert(evalRow: GroupEval): { ok: true; reason: string } | { ok: false } {
  const { baseline, recent, baselineHitRate, recentHitRate, observe, coldStart } = evalRow;

  const observeDenomMin = coldStart
    ? THRESHOLDS.coldRecentDenomTokensMin
    : THRESHOLDS.recentDenomTokensMin;
  const observeSuccessMin = coldStart
    ? THRESHOLDS.coldRecentSuccessRequestsMin
    : THRESHOLDS.recentSuccessRequestsMin;

  if (observe.denomTokens >= observeDenomMin && observe.successRequests >= observeSuccessMin) {
    const creationButNoRead = observe.cacheReadTokens === 0 && observe.cacheCreateTokens > 0;
    if (creationButNoRead) return { ok: true, reason: "缓存创建但读取为 0" };

    const { createShare, createReadRatio } = computeObserveMetrics(observe);
    if (Number.isFinite(createShare) && createShare >= THRESHOLDS.createShareMin) {
      return { ok: true, reason: "缓存创建占比异常高" };
    }

    if (Number.isFinite(createReadRatio) && createReadRatio >= THRESHOLDS.createReadImbalanceMin) {
      return { ok: true, reason: "缓存创建显著高于读取" };
    }
  }

  // Relative drop check: only meaningful with stable baseline.
  if (baseline.denomTokens < THRESHOLDS.baselineDenomTokensMin) return { ok: false };
  if (recent.denomTokens < THRESHOLDS.recentDenomTokensMin) return { ok: false };
  if (baseline.successRequests < THRESHOLDS.baselineSuccessRequestsMin) return { ok: false };
  if (recent.successRequests < THRESHOLDS.recentSuccessRequestsMin) return { ok: false };
  if (!Number.isFinite(baselineHitRate) || baselineHitRate < THRESHOLDS.baselineHitRateMin) {
    return { ok: false };
  }
  if (!Number.isFinite(recentHitRate)) return { ok: false };

  const absDrop = baselineHitRate - recentHitRate;
  const ratio = baselineHitRate > 0 ? recentHitRate / baselineHitRate : NaN;

  const bigDrop = ratio <= THRESHOLDS.dropRatioMin && absDrop >= THRESHOLDS.dropAbsMin;
  if (bigDrop) return { ok: true, reason: "缓存命中率断崖式下降" };

  return { ok: false };
}

async function emitAlert(evalRow: GroupEval, reason: string) {
  const {
    group,
    baseline,
    recent,
    baselineHitRate,
    recentHitRate,
    observe,
    observeHitRate,
    observeMinutes,
    coldStart,
  } = evalRow;
  const { cliKey, providerId, providerName, model } = group.parts;

  const title = `缓存异常（${reason}）`;
  const { createShare: observeCreateShare, createReadRatio: observeCreateReadRatio } =
    computeObserveMetrics(observe);
  const windowLabel = coldStart ? `冷启动(${observeMinutes}m)` : `最近(${observeMinutes}m)`;

  const body = [
    `CLI：${cliKey}`,
    `Provider：${providerName} (#${providerId})`,
    `Model：${model}`,
    `${windowLabel}：命中率 ${formatPct(observeHitRate)} · 读取token ${observe.cacheReadTokens} · 创建token ${observe.cacheCreateTokens} · 分母token ${observe.denomTokens} · 成功请求 ${observe.successRequests}`,
    `创建占比 ${formatPct(observeCreateShare)} · 创建/读取 ${formatRatio(observeCreateReadRatio)}`,
    `基线(45m)：命中率 ${formatPct(baselineHitRate)} · 分母token ${baseline.denomTokens} · 成功请求 ${baseline.successRequests}`,
  ].join("\n");

  logToConsole("warn", title, {
    reason,
    cold_start: coldStart,
    observe_minutes: observeMinutes,
    cli_key: cliKey,
    provider_id: providerId,
    provider_name: providerName,
    requested_model: model,
    baseline,
    recent,
    observe,
    baseline_hit_rate: baselineHitRate,
    recent_hit_rate: recentHitRate,
    observe_hit_rate: observeHitRate,
    observe_create_share: observeCreateShare,
    observe_create_read_ratio: observeCreateReadRatio,
  });

  await safeNoticeSend({ level: "warning", title, body });
}

function disableDueToSelfCheckFailure(nowMs: number, details: unknown) {
  if (nowMs - state.lastSelfCheckFailureAtMs < 10_000) return;
  state.lastSelfCheckFailureAtMs = nowMs;

  logToConsole("error", "缓存异常监测：滑窗统计自检失败，已自动关闭", details);
  void safeNoticeSend({
    level: "error",
    title: "缓存异常监测已关闭",
    body: "滑窗统计自检失败（可能是统计代码问题），已自动关闭监测。请查看控制台详情。",
  });

  setCacheAnomalyMonitorEnabled(false);
}

function maybeEvaluate(nowMs: number) {
  if (!enabled) return;
  if (nowMs - state.lastEvalMs < EVAL_INTERVAL_MS) return;
  state.lastEvalMs = nowMs;

  const minuteNow = Math.floor(nowMs / MINUTE_MS);
  const baselineStart = minuteNow - (BASELINE_MINUTES + RECENT_MINUTES - 1);
  const baselineEnd = minuteNow - RECENT_MINUTES;
  const recentStart = minuteNow - (RECENT_MINUTES - 1);
  const recentEnd = minuteNow;

  const coldStartActive = state.enabledAtMs > 0 && nowMs - state.enabledAtMs < COLD_START_WINDOW_MS;
  const coldRecentMinutes = coldStartActive
    ? Math.min(COLD_START_MINUTES, Math.floor((nowMs - state.enabledAtMs) / MINUTE_MS) + 1)
    : 0;
  const observeMinutes = coldStartActive ? coldRecentMinutes : RECENT_MINUTES;
  const observeStart = minuteNow - (observeMinutes - 1);
  const observeEnd = minuteNow;

  const evalRows: GroupEval[] = [];
  pruneGroups(minuteNow);

  for (const group of state.groups.values()) {
    const baseline = group.ring.sumRange(baselineStart, baselineEnd);
    const recent = group.ring.sumRange(recentStart, recentEnd);
    const observe = coldStartActive ? group.ring.sumRange(observeStart, observeEnd) : recent;

    const baselineHitRate =
      baseline.denomTokens > 0 ? baseline.cacheReadTokens / baseline.denomTokens : NaN;
    const recentHitRate =
      recent.denomTokens > 0 ? recent.cacheReadTokens / recent.denomTokens : NaN;
    const observeHitRate =
      observe.denomTokens > 0 ? observe.cacheReadTokens / observe.denomTokens : NaN;

    evalRows.push({
      group,
      baseline,
      recent,
      baselineHitRate,
      recentHitRate,
      observe,
      observeHitRate,
      observeMinutes,
      coldStart: coldStartActive,
      totalDenomTokens: baseline.denomTokens + recent.denomTokens,
    });
  }

  // Self-check: ensure ring buffer sums match slow recompute for top N groups (by traffic).
  const selfCheckRows = evalRows
    .slice()
    .sort((a, b) => b.totalDenomTokens - a.totalDenomTokens)
    .slice(0, 20);

  for (const row of selfCheckRows) {
    const minKeep = minuteNow - SAMPLE_RETENTION_MINUTES;
    const samples = pruneSamples(row.group.samples, minKeep);
    row.group.samples = samples;

    const slowBaseline = slowSumSamples(samples, baselineStart, baselineEnd);
    const slowRecent = slowSumSamples(samples, recentStart, recentEnd);

    const slowObserve = coldStartActive
      ? slowSumSamples(samples, observeStart, observeEnd)
      : slowRecent;

    if (
      !sumsEqual(row.baseline, slowBaseline) ||
      !sumsEqual(row.recent, slowRecent) ||
      !sumsEqual(row.observe, slowObserve)
    ) {
      disableDueToSelfCheckFailure(nowMs, {
        key: row.group.key,
        parts: row.group.parts,
        ring: { baseline: row.baseline, recent: row.recent, observe: row.observe },
        slow: { baseline: slowBaseline, recent: slowRecent, observe: slowObserve },
        cold_start: coldStartActive,
        observe_minutes: observeMinutes,
      });
      return;
    }
  }

  for (const row of evalRows) {
    const verdict = shouldAlert(row);
    if (!verdict.ok) continue;

    const lastAlertAt = row.group.lastAlertAtMs ?? 0;
    if (nowMs - lastAlertAt < ALERT_DEDUP_MS) continue;

    row.group.lastAlertAtMs = nowMs;
    void emitAlert(row, verdict.reason);
  }
}

export function ingestCacheAnomalyRequestStart(payload: GatewayRequestStartEvent) {
  if (!enabled) return;
  if (!payload?.trace_id) return;
  if (!isSupportedCliKey(payload.cli_key)) return;

  const nowMs = Date.now();
  pruneTraceModels(nowMs);

  const model = normalizeModelName(payload.requested_model);
  const ignore = isNonCachingModel(payload.cli_key, model);

  state.traceModels.set(
    payload.trace_id,
    ignore ? { ignore: true, seenAtMs: nowMs } : { ignore: false, model, seenAtMs: nowMs }
  );
}

export function ingestCacheAnomalyRequest(payload: GatewayRequestEvent) {
  if (!enabled) return;
  if (!payload?.trace_id) return;
  if (!isSupportedCliKey(payload.cli_key)) return;

  const nowMs = Date.now();
  pruneTraceModels(nowMs);

  const traceEntry = state.traceModels.get(payload.trace_id);
  if (traceEntry) state.traceModels.delete(payload.trace_id);

  if (!isSuccessRequest(payload)) return;

  const attempt = pickFinalProvider(payload.attempts);
  if (!attempt) return;
  const providerId = Math.floor(Number(attempt.provider_id));
  if (!Number.isFinite(providerId) || providerId < 0) return;

  const providerName = (attempt.provider_name || "Unknown").trim() || "Unknown";

  if (traceEntry?.ignore) return;
  const model = traceEntry && !traceEntry.ignore ? traceEntry.model : "Unknown";

  const sampleBase = extractSample(payload, nowMs);
  if (sampleBase.successRequest === 0) return;
  if (sampleBase.denomTokens <= 0) return;

  const minuteNow = Math.floor(nowMs / MINUTE_MS);
  const minKeep = minuteNow - SAMPLE_RETENTION_MINUTES;

  const parts: GroupKeyParts = {
    cliKey: payload.cli_key,
    providerId,
    providerName,
    model,
  };
  const key = `${parts.cliKey}:${parts.providerId}:${parts.model}`;

  let group = state.groups.get(key);
  if (!group) {
    group = {
      key,
      parts,
      ring: new MinuteRing(),
      samples: [],
      lastSeenMinute: minuteNow,
      lastAlertAtMs: null,
    };
    state.groups.set(key, group);
  } else {
    group.parts.providerName = providerName;
  }

  group.lastSeenMinute = minuteNow;
  group.ring.add(sampleBase.minute, sampleBase);
  group.samples = addSampleBucket(group.samples, sampleBase, minKeep);
  pruneGroups(minuteNow);

  maybeEvaluate(nowMs);
}
