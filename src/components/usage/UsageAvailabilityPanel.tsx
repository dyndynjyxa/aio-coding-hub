import { useMemo } from "react";
import type { CliKey } from "../../services/providers/providers";
import type { RequestLogSummary } from "../../services/gateway/requestLogs";
import type { GatewayProviderCircuitStatus } from "../../services/gateway/gateway";
import { EmptyState } from "../../ui/EmptyState";
import { Spinner } from "../../ui/Spinner";
import { cn } from "../../utils/cn";
import { formatInteger, formatPercent, formatDurationMs } from "../../utils/formatters";
import { RefreshCw } from "lucide-react";
import { StatCard, StatCardSkeleton } from "./StatCard";

function isSuccess(status: number | null) {
  return status != null && status >= 200 && status < 400;
}

type TimeBucket = {
  startMs: number;
  endMs: number;
  totalRequests: number;
  successCount: number;
  availabilityRate: number;
};

type ProviderTimeline = {
  providerId: number;
  providerName: string;
  cliKey: CliKey;
  totalRequests: number;
  successCount: number;
  availabilityRate: number;
  avgDurationMs: number;
  circuitState: string | null;
  buckets: TimeBucket[];
  density: "dense" | "sparse" | "none";
};

export type AvailabilityTimelineData = {
  providers: ProviderTimeline[];
  bucketStartMs: number;
  bucketEndMs: number;
  bucketCount: number;
  bucketSizeMs: number;
  bucketSizeLabel: string;
};

function computeBucketSizeMs(rangeMs: number): { sizeMs: number; label: string } {
  const TARGET_BUCKETS = 60;
  const raw = rangeMs / TARGET_BUCKETS;

  const candidates = [
    { sizeMs: 5 * 60_000, label: "5 分钟" },
    { sizeMs: 10 * 60_000, label: "10 分钟" },
    { sizeMs: 15 * 60_000, label: "15 分钟" },
    { sizeMs: 24 * 60_000, label: "24 分钟" },
    { sizeMs: 30 * 60_000, label: "30 分钟" },
    { sizeMs: 60 * 60_000, label: "1 小时" },
    { sizeMs: 2 * 60 * 60_000, label: "2 小时" },
    { sizeMs: 4 * 60 * 60_000, label: "4 小时" },
    { sizeMs: 6 * 60 * 60_000, label: "6 小时" },
    { sizeMs: 12 * 60 * 60_000, label: "12 小时" },
    { sizeMs: 24 * 60 * 60_000, label: "1 天" },
  ];

  for (const c of candidates) {
    if (c.sizeMs >= raw) return c;
  }
  return candidates[candidates.length - 1]!;
}

function classifyDensity(totalRequests: number, bucketCount: number): "dense" | "sparse" | "none" {
  if (totalRequests === 0) return "none";
  const avgPerBucket = totalRequests / bucketCount;
  return avgPerBucket >= 2 ? "dense" : "sparse";
}

export function buildAvailabilityTimeline(
  logs: RequestLogSummary[],
  circuitMap: Record<number, GatewayProviderCircuitStatus> | null,
  rangeStartMs: number,
  rangeEndMs: number
): AvailabilityTimelineData {
  const rangeMs = rangeEndMs - rangeStartMs;
  const { sizeMs, label } = computeBucketSizeMs(rangeMs);
  const bucketCount = Math.max(1, Math.ceil(rangeMs / sizeMs));

  const providerMap = new Map<
    number,
    {
      providerId: number;
      providerName: string;
      cliKey: CliKey;
      totalRequests: number;
      successCount: number;
      totalDurationMs: number;
      bucketData: Map<number, { total: number; success: number }>;
    }
  >();

  for (const log of logs) {
    const pid = log.final_provider_id;
    let entry = providerMap.get(pid);
    if (!entry) {
      entry = {
        providerId: pid,
        providerName: log.final_provider_name,
        cliKey: log.cli_key,
        totalRequests: 0,
        successCount: 0,
        totalDurationMs: 0,
        bucketData: new Map(),
      };
      providerMap.set(pid, entry);
    }

    entry.totalRequests++;
    const success = isSuccess(log.status);
    if (success) entry.successCount++;
    entry.totalDurationMs += log.duration_ms;

    const bucketIdx = Math.min(
      bucketCount - 1,
      Math.max(0, Math.floor((log.created_at_ms - rangeStartMs) / sizeMs))
    );
    let bucket = entry.bucketData.get(bucketIdx);
    if (!bucket) {
      bucket = { total: 0, success: 0 };
      entry.bucketData.set(bucketIdx, bucket);
    }
    bucket.total++;
    if (success) bucket.success++;
  }

  const providers: ProviderTimeline[] = [];
  for (const entry of providerMap.values()) {
    const buckets: TimeBucket[] = [];
    for (let i = 0; i < bucketCount; i++) {
      const startMs = rangeStartMs + i * sizeMs;
      const endMs = startMs + sizeMs;
      const data = entry.bucketData.get(i);
      if (data) {
        buckets.push({
          startMs,
          endMs,
          totalRequests: data.total,
          successCount: data.success,
          availabilityRate: data.total > 0 ? data.success / data.total : 0,
        });
      } else {
        buckets.push({
          startMs,
          endMs,
          totalRequests: 0,
          successCount: 0,
          availabilityRate: 0,
        });
      }
    }

    const rate = entry.totalRequests > 0 ? entry.successCount / entry.totalRequests : 0;
    const avgMs = entry.totalRequests > 0 ? entry.totalDurationMs / entry.totalRequests : 0;

    providers.push({
      providerId: entry.providerId,
      providerName: entry.providerName,
      cliKey: entry.cliKey,
      totalRequests: entry.totalRequests,
      successCount: entry.successCount,
      availabilityRate: rate,
      avgDurationMs: avgMs,
      circuitState: circuitMap?.[entry.providerId]?.state ?? null,
      buckets,
      density: classifyDensity(entry.totalRequests, bucketCount),
    });
  }

  providers.sort((a, b) => b.totalRequests - a.totalRequests);

  return {
    providers,
    bucketStartMs: rangeStartMs,
    bucketEndMs: rangeEndMs,
    bucketCount,
    bucketSizeMs: sizeMs,
    bucketSizeLabel: label,
  };
}

function rateColor(rate: number) {
  if (rate >= 0.95) return "text-emerald-500";
  if (rate >= 0.8) return "text-amber-500";
  return "text-rose-500";
}

function statusDotColor(rate: number) {
  if (rate >= 0.95) return "bg-emerald-500";
  if (rate >= 0.8) return "bg-amber-500";
  return "bg-rose-500";
}

function bucketDotColor(bucket: TimeBucket) {
  if (bucket.totalRequests === 0) return null;
  if (bucket.availabilityRate >= 0.95) return "bg-emerald-400";
  if (bucket.availabilityRate >= 0.8) return "bg-amber-400";
  return "bg-rose-400";
}

function densityLabel(density: "dense" | "sparse" | "none") {
  if (density === "dense") return "密集";
  if (density === "sparse") return "稀疏";
  return "";
}

function formatTimeLabel(ms: number) {
  const d = new Date(ms);
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

function formatDateTimeLabel(ms: number) {
  const d = new Date(ms);
  return `${d.getMonth() + 1}/${d.getDate()} ${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

function TimeAxisLabels({
  startMs,
  endMs,
  tickCount,
  rangeMs,
}: {
  startMs: number;
  endMs: number;
  tickCount: number;
  rangeMs: number;
}) {
  const showDate = rangeMs > 48 * 60 * 60_000;
  const formatter = showDate ? formatDateTimeLabel : formatTimeLabel;
  const ticks: { label: string; pct: number }[] = [];
  const step = (endMs - startMs) / tickCount;
  for (let i = 0; i <= tickCount; i++) {
    const ms = startMs + step * i;
    ticks.push({ label: formatter(ms), pct: (i / tickCount) * 100 });
  }

  return (
    <div className="relative h-5 text-[10px] text-muted-foreground">
      {ticks.map((tick, i) => (
        <span
          key={i}
          className="absolute -translate-x-1/2 whitespace-nowrap"
          style={{ left: `${tick.pct}%` }}
        >
          {tick.label}
        </span>
      ))}
    </div>
  );
}

function BucketDot({ bucket, maxRequests }: { bucket: TimeBucket; maxRequests: number }) {
  if (bucket.totalRequests === 0) return <div className="h-full" />;

  const color = bucketDotColor(bucket)!;
  const minSize = 4;
  const maxSize = 14;
  const ratio = maxRequests > 0 ? bucket.totalRequests / maxRequests : 0;
  const size = minSize + ratio * (maxSize - minSize);

  return (
    <div
      className="flex h-full items-center justify-center"
      title={
        `${formatTimeLabel(bucket.startMs)} - ${formatTimeLabel(bucket.endMs)}\n` +
        `请求: ${bucket.totalRequests}\n` +
        `成功: ${bucket.successCount}\n` +
        `可用率: ${formatPercent(bucket.availabilityRate, 1)}`
      }
    >
      <div className={cn("rounded-full", color)} style={{ width: size, height: size }} />
    </div>
  );
}

function ProviderTimelineRow({
  provider,
  maxBucketRequests,
}: {
  provider: ProviderTimeline;
  maxBucketRequests: number;
}) {
  return (
    <div className="flex items-center gap-0 border-b border-border dark:border-border last:border-b-0">
      {/* Left: Provider info */}
      <div className="shrink-0 w-44 py-3 pr-3">
        <div className="flex items-center gap-2">
          <div
            className={cn(
              "h-2.5 w-2.5 shrink-0 rounded-full",
              provider.totalRequests === 0
                ? "bg-muted dark:bg-secondary"
                : statusDotColor(provider.availabilityRate)
            )}
          />
          <span className="text-xs font-semibold text-foreground truncate">
            {provider.providerName}
          </span>
        </div>
        {provider.totalRequests > 0 && (
          <div className="mt-0.5 ml-[18px] text-[10px] text-muted-foreground">
            {densityLabel(provider.density)}
          </div>
        )}
      </div>

      {/* Middle: Timeline dots */}
      <div className="flex-1 min-w-0 py-2">
        <div
          className="grid h-8 rounded bg-secondary/50"
          style={{ gridTemplateColumns: `repeat(${provider.buckets.length}, 1fr)` }}
        >
          {provider.buckets.map((bucket, i) => (
            <BucketDot key={i} bucket={bucket} maxRequests={maxBucketRequests} />
          ))}
        </div>
      </div>

      {/* Right: Rate + count */}
      <div className="shrink-0 w-28 text-right pl-3 py-3">
        {provider.totalRequests > 0 ? (
          <>
            <div
              className={cn("text-sm font-bold tabular-nums", rateColor(provider.availabilityRate))}
            >
              {formatPercent(provider.availabilityRate, 1)}
            </div>
            <div className="text-[10px] text-muted-foreground">
              {formatInteger(provider.totalRequests)} 个请求
            </div>
          </>
        ) : (
          <>
            <div className="text-xs text-muted-foreground">暂无数据</div>
            <div className="text-[10px] text-muted-foreground">无请求</div>
          </>
        )}
      </div>
    </div>
  );
}

function AvailabilitySummaryCards({
  providers,
  loading,
}: {
  providers: ProviderTimeline[];
  loading: boolean;
}) {
  const { overallRate, avgLatency, errorRate } = useMemo(() => {
    let totalReqs = 0;
    let totalSuccess = 0;
    let totalDuration = 0;
    for (const p of providers) {
      totalReqs += p.totalRequests;
      totalSuccess += p.successCount;
      totalDuration += p.avgDurationMs * p.totalRequests;
    }
    return {
      overallRate: totalReqs > 0 ? totalSuccess / totalReqs : 0,
      avgLatency: totalReqs > 0 ? totalDuration / totalReqs : 0,
      errorRate: totalReqs > 0 ? (totalReqs - totalSuccess) / totalReqs : 0,
    };
  }, [providers]);

  if (loading) {
    return (
      <div className="grid grid-cols-3 gap-3">
        <StatCardSkeleton />
        <StatCardSkeleton />
        <StatCardSkeleton />
      </div>
    );
  }

  return (
    <div className="grid grid-cols-3 gap-3">
      <StatCard
        title="总体可用率"
        value={formatPercent(overallRate, 2)}
        accent="green"
        hint={`${providers.length} 个供应商`}
      />
      <StatCard title="平均延迟" value={formatDurationMs(avgLatency)} accent="blue" />
      <StatCard
        title="错误率"
        value={formatPercent(errorRate, 2)}
        accent={errorRate >= 0.05 ? "orange" : "slate"}
      />
    </div>
  );
}

export type UsageAvailabilityPanelProps = {
  data: AvailabilityTimelineData | null;
  loading: boolean;
  onRefresh: () => void;
  refreshing: boolean;
};

export function UsageAvailabilityPanel({
  data,
  loading,
  onRefresh,
  refreshing,
}: UsageAvailabilityPanelProps) {
  if (loading || !data) {
    return (
      <div className="flex flex-col gap-4 px-6 pb-6">
        <AvailabilitySummaryCards providers={[]} loading />
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Spinner size="sm" />
          加载可用率数据中...
        </div>
      </div>
    );
  }

  if (data.providers.length === 0) {
    return (
      <div className="flex flex-col gap-4 px-6 pb-6">
        <AvailabilitySummaryCards providers={[]} loading={false} />
        <EmptyState title="暂无请求记录" description="当有请求经过网关后，可用率数据将自动展示。" />
      </div>
    );
  }

  const maxBucketRequests = Math.max(
    1,
    ...data.providers.flatMap((p) => p.buckets.map((b) => b.totalRequests))
  );

  const tickCount = Math.min(8, data.bucketCount);
  const rangeMs = data.bucketEndMs - data.bucketStartMs;

  return (
    <div className="flex flex-col gap-4 px-6 pb-6">
      <AvailabilitySummaryCards providers={data.providers} loading={false} />

      {/* Timeline section */}
      <div className="rounded-lg border border-border bg-white dark:bg-card p-4">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-sm font-semibold text-foreground">供应商可用性时间线</h3>
          <div className="flex items-center gap-3">
            <span className="text-[10px] text-muted-foreground">
              时间分段: {data.bucketSizeLabel}
            </span>
            <button
              type="button"
              onClick={onRefresh}
              disabled={refreshing}
              className="rounded-md p-1 text-muted-foreground transition-colors hover:bg-secondary hover:text-indigo-600 disabled:cursor-not-allowed disabled:opacity-50 dark:hover:bg-secondary dark:hover:text-indigo-400"
              title="刷新可用率数据"
            >
              <RefreshCw className={cn("h-3.5 w-3.5", refreshing && "animate-spin")} />
            </button>
          </div>
        </div>

        {/* Time axis */}
        <div className="flex items-center gap-0 mb-1">
          <div className="shrink-0 w-44" />
          <div className="flex-1 min-w-0">
            <TimeAxisLabels
              startMs={data.bucketStartMs}
              endMs={data.bucketEndMs}
              tickCount={tickCount}
              rangeMs={rangeMs}
            />
          </div>
          <div className="shrink-0 w-28" />
        </div>

        {/* Provider rows */}
        <div className="flex-1 min-h-0 overflow-auto scrollbar-overlay">
          {data.providers.map((provider) => (
            <ProviderTimelineRow
              key={`${provider.cliKey}:${provider.providerId}`}
              provider={provider}
              maxBucketRequests={maxBucketRequests}
            />
          ))}
        </div>
      </div>
    </div>
  );
}
