export function formatDurationMs(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return "—";
  const ms = Math.max(0, Math.round(value));
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60_000) return `${(ms / 1000).toFixed(2)}s`;
  const minutes = Math.floor(ms / 60_000);
  const seconds = ((ms % 60_000) / 1000).toFixed(1);
  return `${minutes}m${seconds}s`;
}

export function formatDurationMsShort(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return "—";
  const ms = Math.max(0, Math.round(value));
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`;
  const minutes = Math.floor(ms / 60_000);
  if (ms < 3_600_000) return `${minutes}m`;
  const hours = Math.floor(ms / 3_600_000);
  const remainingMinutes = Math.floor((ms % 3_600_000) / 60_000);
  return `${hours}h${remainingMinutes}m`;
}

const INTEGER_FORMATTER = new Intl.NumberFormat(undefined, { maximumFractionDigits: 0 });
const TOKENS_PER_SECOND_FORMATTER = new Intl.NumberFormat(undefined, {
  maximumFractionDigits: 1,
  minimumFractionDigits: 1,
});
const USD_FORMATTER = new Intl.NumberFormat(undefined, {
  maximumFractionDigits: 6,
  minimumFractionDigits: 6,
});
const USD_SHORT_FORMATTER = new Intl.NumberFormat(undefined, {
  maximumFractionDigits: 2,
  minimumFractionDigits: 2,
});

export function formatCompactDurationMs(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return "—";
  const ms = Math.max(0, value);
  if (ms === 0) return "0s";
  if (ms < 1000) return "<1s";

  const totalSeconds = Math.round(ms / 1000);
  if (totalSeconds <= 0) return "<1s";

  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  const parts: string[] = [];
  if (hours > 0) parts.push(`${hours}h`);
  if (minutes > 0) parts.push(`${minutes}m`);
  if (seconds > 0 || parts.length === 0) parts.push(`${seconds}s`);
  return parts.join("");
}

export function sanitizeTtfbMs(
  ttfbMs: number | null | undefined,
  durationMs: number | null | undefined
) {
  if (ttfbMs == null || !Number.isFinite(ttfbMs)) return null;
  if (durationMs == null || !Number.isFinite(durationMs)) return null;

  const t = Math.max(0, ttfbMs);
  const d = Math.max(0, durationMs);
  if (t > d) return null;
  return t;
}

export function formatInteger(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return "—";
  const v = Math.max(0, Math.round(value));
  try {
    return INTEGER_FORMATTER.format(v);
  } catch {
    return String(v);
  }
}

export function formatPercent(value: number | null | undefined, digits = 1) {
  if (value == null || !Number.isFinite(value)) return "—";
  const pct = value * 100;
  const d = Number.isFinite(digits) ? Math.min(6, Math.max(0, Math.round(digits))) : 0;
  const factor = 10 ** d;
  const rounded = Math.round(pct * factor) / factor;
  return `${rounded.toFixed(d)}%`;
}

// Threshold above which a computed rate is considered unreliable when the
// generation window (duration − TTFB) is very small relative to total duration.
// Some upstream proxies buffer SSE responses, causing TTFB ≈ duration and an
// inflated rate.  When detected we fall back to total-duration throughput.
// See also: claude-code-hub `shouldHideOutputRate` (uses 5000; we use a lower
// ceiling because falling back is less disruptive than hiding entirely).
const OUTPUT_RATE_SANITY_CEIL = 5000;

export function computeOutputTokensPerSecond(
  outputTokens: number | null | undefined,
  durationMs: number | null | undefined,
  ttfbMs: number | null | undefined
) {
  if (outputTokens == null || !Number.isFinite(outputTokens)) return null;
  if (durationMs == null || !Number.isFinite(durationMs) || durationMs <= 0) return null;
  if (ttfbMs == null || !Number.isFinite(ttfbMs)) return null;
  const generationMs = durationMs - ttfbMs;
  if (!Number.isFinite(generationMs) || generationMs <= 0) {
    // Fallback: non-stream or precision truncation where ttfb == duration.
    if (outputTokens > 0) {
      return outputTokens / (durationMs / 1000);
    }
    return null;
  }
  const rate = outputTokens / (generationMs / 1000);

  // Sanity check: if the generation window is < 10% of total duration and the
  // computed rate exceeds a reasonable ceiling, the TTFB is likely inflated
  // (e.g. upstream proxy buffering SSE).  Fall back to total-duration throughput.
  if (generationMs / durationMs < 0.1 && rate > OUTPUT_RATE_SANITY_CEIL) {
    return outputTokens / (durationMs / 1000);
  }

  return rate;
}

export function formatTokensPerSecond(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return "—";
  const v = Math.max(0, value);
  try {
    return `${TOKENS_PER_SECOND_FORMATTER.format(v)} Token/秒`;
  } catch {
    return `${v.toFixed(1)} Token/秒`;
  }
}

export function formatUsd(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return "—";
  const v = Math.max(0, value);
  try {
    return `$${USD_FORMATTER.format(v)}`;
  } catch {
    return `$${v.toFixed(6)}`;
  }
}

// Keep raw output for debug-style displays; user-facing cost UI should prefer `formatUsd`.
export function formatUsdRaw(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return "—";
  return `$${String(value)}`;
}

export function formatUsdShort(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return "—";
  const v = Math.max(0, value);
  try {
    return `$${USD_SHORT_FORMATTER.format(v)}`;
  } catch {
    return `$${v.toFixed(2)}`;
  }
}

export function formatUnixSeconds(ts: number | null | undefined) {
  if (ts == null || !Number.isFinite(ts)) return "—";
  try {
    return new Date(ts * 1000).toLocaleString();
  } catch {
    return String(ts);
  }
}

export function formatCountdownSeconds(totalSeconds: number | null | undefined) {
  if (totalSeconds == null || !Number.isFinite(totalSeconds)) return "—";
  const total = Math.max(0, Math.floor(totalSeconds));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;
  const pad2 = (v: number) => String(v).padStart(2, "0");
  return hours > 0
    ? `${hours}:${pad2(minutes)}:${pad2(seconds)}`
    : `${pad2(minutes)}:${pad2(seconds)}`;
}

export function formatRelativeTimeFromMs(
  timestampMs: number | null | undefined,
  nowMs: number = Date.now()
) {
  if (timestampMs == null || !Number.isFinite(timestampMs)) return "—";
  if (!Number.isFinite(nowMs)) return "—";

  const deltaMs = Math.max(0, nowMs - timestampMs);
  if (deltaMs < 60_000) return "<1分钟";

  const minutes = Math.floor(deltaMs / 60_000);
  if (minutes < 60) return `${minutes}分钟`;

  const hours = Math.floor(deltaMs / 3_600_000);
  if (hours < 24) return `${hours}小时`;

  const days = Math.floor(deltaMs / 86_400_000);
  return `${days}天`;
}

export function formatRelativeTimeFromUnixSeconds(
  ts: number | null | undefined,
  nowMs: number = Date.now()
) {
  if (ts == null || !Number.isFinite(ts)) return "—";
  return formatRelativeTimeFromMs(ts * 1000, nowMs);
}

export function formatBytes(bytes: number | null | undefined) {
  if (bytes == null || !Number.isFinite(bytes) || bytes < 0) return "—";
  const b = Math.floor(bytes);
  if (b < 1024) return `${b} B`;
  const kb = b / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB`;
  const mb = kb / 1024;
  if (mb < 1024) return `${mb.toFixed(1)} MB`;
  const gb = mb / 1024;
  return `${gb.toFixed(2)} GB`;
}

export function formatTokensPerSecondShort(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return "—";
  const v = Math.max(0, value);
  if (v >= 1000) {
    return `${(v / 1000).toFixed(1)}k t/s`;
  }
  return `${v.toFixed(1)} t/s`;
}

export function formatUsdCompact(value: number | null | undefined): string {
  if (value == null || !Number.isFinite(value)) return "—";
  const v = Math.max(0, value);
  if (v === 0) return "$0";
  if (v < 0.01) return `$${v.toFixed(4)}`;
  return `$${v.toFixed(2)}`;
}

export function formatIsoDateTime(value: string | null | undefined) {
  if (!value) return "—";
  try {
    const d = new Date(value);
    if (!Number.isFinite(d.getTime())) return value;
    const yyyy = d.getFullYear();
    const mm = String(d.getMonth() + 1).padStart(2, "0");
    const dd = String(d.getDate()).padStart(2, "0");
    const hh = String(d.getHours()).padStart(2, "0");
    const mi = String(d.getMinutes()).padStart(2, "0");
    const ss = String(d.getSeconds()).padStart(2, "0");
    return `${yyyy}-${mm}-${dd} ${hh}:${mi}:${ss}`;
  } catch {
    return value;
  }
}

/**
 * Circuit-breaker recovery hint: absolute local time plus remaining minutes.
 * Returns null when the recovery point is unknown (graceful degradation for
 * logs recorded before attribution existed or lost across backend restart).
 */
export function formatCircuitRecovery(
  recoverAtUnix: number | null | undefined,
  nowMs: number = Date.now()
) {
  if (recoverAtUnix == null || !Number.isFinite(recoverAtUnix)) return null;
  const at = formatUnixSeconds(recoverAtUnix);
  const remainingSec = recoverAtUnix - Math.floor(nowMs / 1000);
  if (remainingSec <= 0) return `已过预计恢复时间（${at}）`;
  const minutes = Math.max(1, Math.ceil(remainingSec / 60));
  return `预计 ${at} 恢复（约 ${minutes} 分钟后）`;
}
