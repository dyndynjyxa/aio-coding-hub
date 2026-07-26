import { describe, expect, it } from "vitest";
import attemptFixture from "../__fixtures__/gatewayEvents/attempt.json";
import circuitFixture from "../__fixtures__/gatewayEvents/circuit.json";
import logFixture from "../__fixtures__/gatewayEvents/log.json";
import requestFixture from "../__fixtures__/gatewayEvents/request.json";
import requestSignalFixture from "../__fixtures__/gatewayEvents/request_signal.json";
import requestStartFixture from "../__fixtures__/gatewayEvents/request_start.json";
import {
  isGatewayCircuitEvent,
  isGatewayLogEvent,
  normalizeGatewayAttemptEvent,
  normalizeGatewayRequestEvent,
  normalizeGatewayRequestSignalEvent,
  normalizeGatewayRequestStartEvent,
} from "../gatewayEvents";

// These fixtures are the wire contract with the Rust emitter — the same files
// are asserted (serde_json value equality: exact key set + values) by
// src-tauri/src/gateway/events.rs tests (*_payload_matches_shared_fixture).
// A normalizer rejecting a fixture means the frontend guards drifted from
// what the backend actually emits.
describe("gateway event payload contract (shared fixtures)", () => {
  it("accepts the gateway:request fixture", () => {
    const normalized = normalizeGatewayRequestEvent(requestFixture);
    expect(normalized).not.toBeNull();
    expect(normalized?.trace_id).toBe("trace-fixture-001");
    expect(normalized?.status).toBe(200);
    expect(normalized?.attempts).toHaveLength(1);
    expect(normalized?.attempts[0]?.provider_id).toBe(7);
    // Nested mapping is camelCase inside an otherwise snake_case payload.
    expect(normalized?.claude_model_mapping?.effectiveModel).toBe("gpt-5.4");
    expect(normalized?.cache_read_input_tokens).toBe(800);
    // Backend-computed field must survive normalization (it is optional in the
    // type, so dropping the copy would not fail typecheck).
    expect(normalized?.effective_input_tokens).toBe(1200);
  });

  it("accepts null forms of the optional gateway:request fields", () => {
    // The emitter serializes Option::None as explicit null (no
    // skip_serializing_if — guarded by crossLayerContracts.test.ts); the
    // normalizer must stay permissive for those wire shapes.
    const normalized = normalizeGatewayRequestEvent({
      ...requestFixture,
      session_id: null,
      query: null,
      status: null,
      ttfb_ms: null,
      effective_input_tokens: null,
      claude_model_mapping: null,
    });
    expect(normalized).not.toBeNull();
    expect(normalized?.effective_input_tokens).toBeNull();
    expect(normalized?.claude_model_mapping).toBeNull();
  });

  it("accepts the gateway:request_start fixture", () => {
    const normalized = normalizeGatewayRequestStartEvent(requestStartFixture);
    expect(normalized).not.toBeNull();
    expect(normalized?.method).toBe("POST");
    expect(normalized?.ts).toBe(1750000000);
  });

  it("accepts the gateway:request_signal fixture", () => {
    const normalized = normalizeGatewayRequestSignalEvent(requestSignalFixture);
    expect(normalized).not.toBeNull();
    expect(normalized?.phase).toBe("complete");
  });

  it("accepts the gateway:attempt fixture", () => {
    const normalized = normalizeGatewayAttemptEvent(attemptFixture);
    expect(normalized).not.toBeNull();
    expect(normalized?.attempt_index).toBe(1);
    expect(normalized?.outcome).toBe("success");
    expect(normalized?.claude_model_mapping?.requestedModel).toBe("claude-sonnet-4-5");
  });

  it("accepts the gateway:circuit fixture", () => {
    expect(isGatewayCircuitEvent(circuitFixture)).toBe(true);
  });

  it("accepts gateway:circuit payloads without trigger attribution fields (legacy backend)", () => {
    const { trigger_error_code: _code, first_byte_timeout_secs: _secs, ...legacy } = circuitFixture;
    expect(isGatewayCircuitEvent(legacy)).toBe(true);
  });

  it("accepts the gateway:log fixture", () => {
    expect(isGatewayLogEvent(logFixture)).toBe(true);
  });
});
