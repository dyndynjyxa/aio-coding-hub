import { describe, expect, it } from "vitest";
import type { RequestLogSummary } from "../requestLogs";
import type { TraceSession } from "../traceStore";
import { buildRequestActivityProjection } from "../requestActivityProjection";

function log(overrides: Partial<RequestLogSummary> = {}): RequestLogSummary {
  return {
    id: 1,
    trace_id: "trace-1",
    cli_key: "claude",
    session_id: null,
    method: "POST",
    path: "/v1/messages",
    query: null,
    status: null,
    error_code: null,
    duration_ms: 0,
    ttfb_ms: null,
    attempts_json: "[]",
    input_tokens: null,
    output_tokens: null,
    total_tokens: null,
    cache_read_input_tokens: null,
    cache_creation_input_tokens: null,
    cache_creation_5m_input_tokens: null,
    cache_creation_1h_input_tokens: null,
    usage_json: null,
    requested_model: "claude-3-opus",
    cost_usd: null,
    cost_multiplier: 1,
    special_settings_json: null,
    provider_chain_json: null,
    error_details_json: null,
    final_provider_id: null,
    created_at_ms: 1_700_000_000_000,
    created_at: 1_700_000_000,
    ...overrides,
  } as RequestLogSummary;
}

function trace(overrides: Partial<TraceSession> = {}): TraceSession {
  return {
    trace_id: "trace-1",
    cli_key: "claude",
    session_id: null,
    method: "POST",
    path: "/v1/messages",
    query: null,
    requested_model: "claude-3-opus",
    claude_model_mapping: null,
    first_seen_ms: 1_700_000_000_000,
    last_seen_ms: 1_700_000_000_000,
    attempts: [],
    ...overrides,
  };
}

describe("services/gateway/requestActivityProjection", () => {
  const persistedMappingSettings = JSON.stringify([
    {
      type: "claude_model_mapping",
      requestedModel: "claude-sonnet",
      effectiveModel: "gpt-5.4",
      mappingKind: "sonnet",
      providerId: 2,
      providerName: "Provider B",
      applied: true,
    },
  ]);

  it("keeps old pending logs visible as fallback rows without live traces", () => {
    const projection = buildRequestActivityProjection({
      requestLogs: [
        log({
          trace_id: "old-pending",
          created_at_ms: 1_700_000_000_000 - 10 * 60 * 1000,
        }),
      ],
      traces: [],
      nowMs: 1_700_000_000_000,
      realtimeCardLimit: 5,
      realtimeCandidateLimit: 20,
    });

    expect(projection.realtimeCards).toHaveLength(0);
    expect(projection.requestRows.map((row) => row.log.trace_id)).toEqual(["old-pending"]);
    expect(projection.requestRows[0]?.liveTrace).toBeNull();
    expect(projection.hasPending).toBe(true);
  });

  it("renders a pending log with a visible trace as one realtime card and no duplicate row", () => {
    const projection = buildRequestActivityProjection({
      requestLogs: [log({ trace_id: "live-pending" })],
      traces: [trace({ trace_id: "live-pending" })],
      nowMs: 1_700_000_000_000 + 10 * 60 * 1000,
      realtimeCardLimit: 5,
      realtimeCandidateLimit: 20,
    });

    expect(projection.realtimeCards.map((card) => card.trace.trace_id)).toEqual(["live-pending"]);
    expect(projection.requestRows).toHaveLength(0);
    expect(projection.visibleRealtimeTraceIds.has("live-pending")).toBe(true);
    expect(projection.hasLiveRealtimeCards).toBe(true);
  });

  it("hides terminal rows only while their completed realtime card is in the exit window", () => {
    const completedTrace = trace({
      trace_id: "completed",
      last_seen_ms: 1_700_000_000_000,
      summary: {
        trace_id: "completed",
        cli_key: "claude",
        method: "POST",
        path: "/v1/messages",
        query: null,
        status: 200,
        error_category: null,
        error_code: null,
        duration_ms: 500,
        ttfb_ms: null,
        attempts: [],
      } as TraceSession["summary"],
    });
    const completedLog = log({ trace_id: "completed", status: 200, duration_ms: 500 });

    const duringExit = buildRequestActivityProjection({
      requestLogs: [completedLog],
      traces: [completedTrace],
      nowMs: 1_700_000_000_500,
      realtimeCardLimit: 5,
      realtimeCandidateLimit: 20,
    });
    expect(duringExit.realtimeCards.map((card) => card.trace.trace_id)).toEqual(["completed"]);
    expect(duringExit.requestRows).toHaveLength(0);

    const afterExit = buildRequestActivityProjection({
      requestLogs: [completedLog],
      traces: [completedTrace],
      nowMs: 1_700_000_002_000,
      realtimeCardLimit: 5,
      realtimeCandidateLimit: 20,
    });
    expect(afterExit.realtimeCards).toHaveLength(0);
    expect(afterExit.requestRows.map((row) => row.log.trace_id)).toEqual(["completed"]);
  });

  it("backfills realtime card model mapping from persisted request log settings", () => {
    const projection = buildRequestActivityProjection({
      requestLogs: [
        log({
          trace_id: "mapped-pending",
          requested_model: "claude-sonnet",
          special_settings_json: persistedMappingSettings,
          final_provider_id: 2,
        }),
      ],
      traces: [
        trace({
          trace_id: "mapped-pending",
          requested_model: "claude-sonnet",
          claude_model_mapping: null,
        }),
      ],
      nowMs: 1_700_000_000_000,
      realtimeCardLimit: 5,
      realtimeCandidateLimit: 20,
    });

    expect(projection.realtimeCards[0]?.trace.claude_model_mapping).toMatchObject({
      requestedModel: "claude-sonnet",
      effectiveModel: "gpt-5.4",
      providerId: 2,
    });
  });

  it("keeps live trace model mapping ahead of persisted request log settings", () => {
    const projection = buildRequestActivityProjection({
      requestLogs: [
        log({
          trace_id: "mapped-live",
          special_settings_json: persistedMappingSettings,
          final_provider_id: 2,
        }),
      ],
      traces: [
        trace({
          trace_id: "mapped-live",
          claude_model_mapping: {
            requestedModel: "claude-sonnet",
            effectiveModel: "gpt-4.1",
            mappingKind: "sonnet",
            providerId: 1,
            providerName: "Provider A",
            applied: true,
          },
        }),
      ],
      nowMs: 1_700_000_000_000,
      realtimeCardLimit: 5,
      realtimeCandidateLimit: 20,
    });

    expect(projection.realtimeCards[0]?.trace.claude_model_mapping).toMatchObject({
      effectiveModel: "gpt-4.1",
      providerId: 1,
    });
  });
});
