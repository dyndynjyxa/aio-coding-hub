import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { UsageAvailabilityPanel } from "../UsageAvailabilityPanel";
import { buildAvailabilityTimeline } from "../usageAvailabilityTimeline";
import type { RequestLogSummary } from "../../../services/gateway/requestLogs";
import type { GatewayProviderCircuitStatus } from "../../../services/gateway/gateway";
import type { CliKey } from "../../../services/providers/providers";

function makeLog(
  overrides: Partial<RequestLogSummary> & { final_provider_id: number }
): RequestLogSummary {
  return {
    id: 1,
    trace_id: "t1",
    cli_key: "claude" as CliKey,
    session_id: null,
    method: "POST",
    path: "/v1/messages",
    excluded_from_stats: false,
    special_settings_json: null,
    requested_model: null,
    status: 200,
    error_code: null,
    is_interrupted: false,
    duration_ms: 1000,
    ttfb_ms: null,
    attempt_count: 1,
    has_failover: false,
    start_provider_id: 1,
    start_provider_name: "Provider A",
    final_provider_name: "Provider A",
    final_provider_source_id: null,
    final_provider_source_name: null,
    route: [],
    session_reuse: false,
    input_tokens: null,
    output_tokens: null,
    total_tokens: null,
    cache_read_input_tokens: null,
    cache_creation_input_tokens: null,
    cache_creation_5m_input_tokens: null,
    cache_creation_1h_input_tokens: null,
    effective_input_tokens: null,
    cost_usd: null,
    provider_chain_json: null,
    error_details_json: null,
    cost_multiplier: 1,
    created_at_ms: 0,
    last_activity_ms: null,
    activity_details_json: null,
    created_at: 0,
    ...overrides,
  };
}

const HOUR_MS = 60 * 60 * 1000;
const DAY_MS = 24 * HOUR_MS;

describe("buildAvailabilityTimeline", () => {
  it("returns empty providers for empty logs", () => {
    const result = buildAvailabilityTimeline([], null, 0, DAY_MS);
    expect(result.providers).toEqual([]);
    expect(result.bucketCount).toBeGreaterThan(0);
    expect(result.bucketSizeMs).toBeGreaterThan(0);
  });

  it("groups logs by provider and calculates availability rate", () => {
    const start = 0;
    const end = DAY_MS;
    const logs = [
      makeLog({
        id: 1,
        final_provider_id: 1,
        final_provider_name: "P1",
        status: 200,
        created_at_ms: HOUR_MS,
        duration_ms: 100,
      }),
      makeLog({
        id: 2,
        final_provider_id: 1,
        final_provider_name: "P1",
        status: 500,
        created_at_ms: 2 * HOUR_MS,
        duration_ms: 200,
      }),
      makeLog({
        id: 3,
        final_provider_id: 2,
        final_provider_name: "P2",
        status: 200,
        created_at_ms: 3 * HOUR_MS,
        duration_ms: 300,
      }),
    ];

    const result = buildAvailabilityTimeline(logs, null, start, end);
    expect(result.providers).toHaveLength(2);

    const p1 = result.providers.find((p) => p.providerId === 1)!;
    expect(p1.totalRequests).toBe(2);
    expect(p1.successCount).toBe(1);
    expect(p1.availabilityRate).toBe(0.5);
    expect(p1.avgDurationMs).toBe(150);

    const p2 = result.providers.find((p) => p.providerId === 2)!;
    expect(p2.totalRequests).toBe(1);
    expect(p2.successCount).toBe(1);
    expect(p2.availabilityRate).toBe(1);
  });

  it("sorts providers by totalRequests descending", () => {
    const logs = [
      makeLog({
        id: 1,
        final_provider_id: 1,
        final_provider_name: "Few",
        status: 200,
        created_at_ms: HOUR_MS,
        duration_ms: 100,
      }),
      makeLog({
        id: 2,
        final_provider_id: 2,
        final_provider_name: "Many",
        status: 200,
        created_at_ms: HOUR_MS,
        duration_ms: 100,
      }),
      makeLog({
        id: 3,
        final_provider_id: 2,
        final_provider_name: "Many",
        status: 200,
        created_at_ms: 2 * HOUR_MS,
        duration_ms: 100,
      }),
      makeLog({
        id: 4,
        final_provider_id: 2,
        final_provider_name: "Many",
        status: 200,
        created_at_ms: 3 * HOUR_MS,
        duration_ms: 100,
      }),
    ];

    const result = buildAvailabilityTimeline(logs, null, 0, DAY_MS);
    expect(result.providers[0]!.providerName).toBe("Many");
    expect(result.providers[1]!.providerName).toBe("Few");
  });

  it("places logs into correct time buckets", () => {
    const start = 0;
    const end = HOUR_MS;
    const logs = [
      makeLog({
        id: 1,
        final_provider_id: 1,
        final_provider_name: "P1",
        status: 200,
        created_at_ms: 2 * 60_000,
        duration_ms: 100,
      }),
      makeLog({
        id: 2,
        final_provider_id: 1,
        final_provider_name: "P1",
        status: 200,
        created_at_ms: 3 * 60_000,
        duration_ms: 100,
      }),
      makeLog({
        id: 3,
        final_provider_id: 1,
        final_provider_name: "P1",
        status: 500,
        created_at_ms: 50 * 60_000,
        duration_ms: 100,
      }),
    ];

    const result = buildAvailabilityTimeline(logs, null, start, end);
    const p1 = result.providers[0]!;

    const nonEmptyBuckets = p1.buckets.filter((b) => b.totalRequests > 0);
    expect(nonEmptyBuckets.length).toBeGreaterThanOrEqual(1);

    const totalReqs = p1.buckets.reduce((sum, b) => sum + b.totalRequests, 0);
    expect(totalReqs).toBe(3);
  });

  it("attaches circuit breaker state from circuitMap", () => {
    const circuitMap: Record<number, GatewayProviderCircuitStatus> = {
      1: {
        provider_id: 1,
        state: "OPEN",
        failure_count: 5,
        failure_threshold: 3,
        open_until: 999,
        cooldown_until: null,
      },
    };

    const logs = [
      makeLog({
        id: 1,
        final_provider_id: 1,
        final_provider_name: "P1",
        status: 200,
        created_at_ms: HOUR_MS,
        duration_ms: 100,
      }),
      makeLog({
        id: 2,
        final_provider_id: 2,
        final_provider_name: "P2",
        status: 200,
        created_at_ms: HOUR_MS,
        duration_ms: 100,
      }),
    ];

    const result = buildAvailabilityTimeline(logs, circuitMap, 0, DAY_MS);
    const p1 = result.providers.find((p) => p.providerId === 1)!;
    const p2 = result.providers.find((p) => p.providerId === 2)!;

    expect(p1.circuitState).toBe("OPEN");
    expect(p2.circuitState).toBeNull();
  });

  it("classifies density correctly", () => {
    const start = 0;
    const end = DAY_MS;

    const manyLogs = Array.from({ length: 200 }, (_, i) =>
      makeLog({
        id: i,
        final_provider_id: 1,
        final_provider_name: "Dense",
        status: 200,
        created_at_ms: start + (i / 200) * (end - start),
        duration_ms: 100,
      })
    );

    const fewLogs = [
      makeLog({
        id: 999,
        final_provider_id: 2,
        final_provider_name: "Sparse",
        status: 200,
        created_at_ms: HOUR_MS,
        duration_ms: 100,
      }),
    ];

    const result = buildAvailabilityTimeline([...manyLogs, ...fewLogs], null, start, end);
    const dense = result.providers.find((p) => p.providerName === "Dense")!;
    const sparse = result.providers.find((p) => p.providerName === "Sparse")!;

    expect(dense.density).toBe("dense");
    expect(sparse.density).toBe("sparse");
  });

  it("treats 2xx and 3xx as success, 4xx and 5xx as failures", () => {
    const logs = [
      makeLog({
        id: 1,
        final_provider_id: 1,
        final_provider_name: "P",
        status: 200,
        created_at_ms: HOUR_MS,
        duration_ms: 100,
      }),
      makeLog({
        id: 2,
        final_provider_id: 1,
        final_provider_name: "P",
        status: 301,
        created_at_ms: 2 * HOUR_MS,
        duration_ms: 100,
      }),
      makeLog({
        id: 3,
        final_provider_id: 1,
        final_provider_name: "P",
        status: 400,
        created_at_ms: 3 * HOUR_MS,
        duration_ms: 100,
      }),
      makeLog({
        id: 4,
        final_provider_id: 1,
        final_provider_name: "P",
        status: 500,
        created_at_ms: 4 * HOUR_MS,
        duration_ms: 100,
      }),
      makeLog({
        id: 5,
        final_provider_id: 1,
        final_provider_name: "P",
        status: null,
        created_at_ms: 5 * HOUR_MS,
        duration_ms: 100,
      }),
    ];

    const result = buildAvailabilityTimeline(logs, null, 0, DAY_MS);
    const p = result.providers[0]!;
    expect(p.successCount).toBe(2);
    expect(p.totalRequests).toBe(5);
    expect(p.availabilityRate).toBeCloseTo(0.4);
  });

  it("chooses appropriate bucket size for different time ranges", () => {
    const r1h = buildAvailabilityTimeline([], null, 0, HOUR_MS);
    expect(r1h.bucketSizeMs).toBeLessThanOrEqual(5 * 60_000);

    const r24h = buildAvailabilityTimeline([], null, 0, DAY_MS);
    expect(r24h.bucketSizeMs).toBeGreaterThanOrEqual(5 * 60_000);
    expect(r24h.bucketSizeMs).toBeLessThanOrEqual(60 * 60_000);

    const r7d = buildAvailabilityTimeline([], null, 0, 7 * DAY_MS);
    expect(r7d.bucketSizeMs).toBeGreaterThanOrEqual(60 * 60_000);

    const r90d = buildAvailabilityTimeline([], null, 0, 90 * DAY_MS);
    expect(r90d.bucketSizeMs).toBe(DAY_MS);
  });

  it("generates correct number of buckets per provider", () => {
    const logs = [
      makeLog({
        id: 1,
        final_provider_id: 1,
        final_provider_name: "P1",
        status: 200,
        created_at_ms: HOUR_MS,
        duration_ms: 100,
      }),
    ];

    const result = buildAvailabilityTimeline(logs, null, 0, DAY_MS);
    const p1 = result.providers[0]!;
    expect(p1.buckets).toHaveLength(result.bucketCount);
    expect(result.bucketCount).toBe(Math.ceil(DAY_MS / result.bucketSizeMs));
  });
});

describe("UsageAvailabilityPanel", () => {
  it("renders loading and empty states", () => {
    const { rerender } = render(
      <UsageAvailabilityPanel data={null} loading onRefresh={vi.fn()} refreshing={false} />
    );

    expect(screen.getByText("加载可用率数据中...")).toBeInTheDocument();
    expect(screen.getByLabelText("Loading")).toBeInTheDocument();

    rerender(
      <UsageAvailabilityPanel
        data={buildAvailabilityTimeline([], null, 0, DAY_MS)}
        loading={false}
        onRefresh={vi.fn()}
        refreshing={false}
      />
    );

    expect(screen.getByText("暂无请求记录")).toBeInTheDocument();
    expect(screen.getByText("当有请求经过网关后，可用率数据将自动展示。")).toBeInTheDocument();
    expect(screen.getByText("0 个供应商")).toBeInTheDocument();
  });

  it("renders provider rows, bucket dots, date ticks, and refresh state", () => {
    const onRefresh = vi.fn();
    const data = buildAvailabilityTimeline(
      [
        makeLog({
          id: 1,
          final_provider_id: 1,
          final_provider_name: "Healthy",
          status: 200,
          created_at_ms: HOUR_MS,
          duration_ms: 100,
        }),
        makeLog({
          id: 2,
          final_provider_id: 2,
          final_provider_name: "Partial",
          status: 200,
          created_at_ms: DAY_MS,
          duration_ms: 200,
        }),
        makeLog({
          id: 3,
          final_provider_id: 2,
          final_provider_name: "Partial",
          status: 500,
          created_at_ms: DAY_MS + HOUR_MS,
          duration_ms: 400,
        }),
        makeLog({
          id: 4,
          final_provider_id: 3,
          final_provider_name: "Down",
          status: 500,
          created_at_ms: 2 * DAY_MS,
          duration_ms: 600,
        }),
      ],
      null,
      0,
      4 * DAY_MS
    );

    render(
      <UsageAvailabilityPanel data={data} loading={false} onRefresh={onRefresh} refreshing={true} />
    );

    expect(screen.getByText("供应商可用性时间线")).toBeInTheDocument();
    expect(screen.getByText(`时间分段: ${data.bucketSizeLabel}`)).toBeInTheDocument();
    expect(screen.getByText("Healthy")).toBeInTheDocument();
    expect(screen.getByText("Partial")).toBeInTheDocument();
    expect(screen.getByText("Down")).toBeInTheDocument();
    expect(screen.getAllByText("稀疏")).toHaveLength(3);

    expect(screen.getAllByText(/\d+\/\d+ \d{2}:\d{2}/u).length).toBeGreaterThan(0);
    const bucketTitles = Array.from(document.querySelectorAll<HTMLElement>("[title]")).map(
      (element) => element.title
    );
    expect(
      bucketTitles.some(
        (title) =>
          title.includes("请求: 1") && title.includes("成功: 1") && title.includes("100.0%")
      )
    ).toBe(true);
    expect(
      bucketTitles.some(
        (title) => title.includes("请求: 1") && title.includes("成功: 0") && title.includes("0.0%")
      )
    ).toBe(true);

    const refreshButton = screen.getByRole("button", { name: "刷新可用率数据" });
    expect(refreshButton).toBeDisabled();
    fireEvent.click(refreshButton);
    expect(onRefresh).not.toHaveBeenCalled();
  });

  it("invokes refresh and labels dense traffic", () => {
    const onRefresh = vi.fn();
    const denseLogs = Array.from({ length: 130 }, (_, index) =>
      makeLog({
        id: index + 1,
        final_provider_id: 1,
        final_provider_name: "Dense",
        status: 200,
        created_at_ms: index * 60_000,
        duration_ms: 100,
      })
    );

    render(
      <UsageAvailabilityPanel
        data={buildAvailabilityTimeline(denseLogs, null, 0, HOUR_MS)}
        loading={false}
        onRefresh={onRefresh}
        refreshing={false}
      />
    );

    expect(screen.getByText("密集")).toBeInTheDocument();
    fireEvent.click(screen.getByTitle("刷新可用率数据"));
    expect(onRefresh).toHaveBeenCalledTimes(1);
  });
});
