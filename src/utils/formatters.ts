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

export function sanitizeTtfbMs(
  ttfbMs: number | null | undefined,
  durationMs: number | null | undefined
) {
  if (ttfbMs == null || !Number.isFinite(ttfbMs)) return null;
  if (durationMs == null || !Number.isFinite(durationMs)) return null;

  const t = Math.max(0, ttfbMs);
  const d = Math.max(0, durationMs);
  if (t >= d) return null;
  return t;
}

export function formatInteger(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return "—";
  const v = Math.max(0, Math.round(value));
  try {
    return new Intl.NumberFormat(undefined, { maximumFractionDigits: 0 }).format(v);
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

export function computeOutputTokensPerSecond(
  outputTokens: number | null | undefined,
  durationMs: number | null | undefined,
  ttfbMs: number | null | undefined
) {
  if (outputTokens == null || !Number.isFinite(outputTokens)) return null;
  if (durationMs == null || !Number.isFinite(durationMs) || durationMs <= 0) return null;
  if (ttfbMs == null || !Number.isFinite(ttfbMs)) return null;
  const generationMs = durationMs - ttfbMs;
  if (!Number.isFinite(generationMs) || generationMs <= 0) return null;
  return outputTokens / (generationMs / 1000);
}

export function formatTokensPerSecond(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return "—";
  const v = Math.max(0, value);
  try {
    return `${new Intl.NumberFormat(undefined, {
      maximumFractionDigits: 1,
      minimumFractionDigits: 1,
    }).format(v)} Token/秒`;
  } catch {
    return `${v.toFixed(1)} Token/秒`;
  }
}

export function formatUsd(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return "—";
  const v = Math.max(0, value);
  try {
    return `$${new Intl.NumberFormat(undefined, {
      maximumFractionDigits: 6,
      minimumFractionDigits: 6,
    }).format(v)}`;
  } catch {
    return `$${v.toFixed(6)}`;
  }
}

export function formatUsdShort(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return "—";
  const v = Math.max(0, value);
  try {
    return `$${new Intl.NumberFormat(undefined, {
      maximumFractionDigits: 2,
      minimumFractionDigits: 2,
    }).format(v)}`;
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
