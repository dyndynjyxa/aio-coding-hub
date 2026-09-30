import { describe, expect, it } from "vitest";
import {
  hasCodexSystemRequestSpecialSetting,
  resolveClaudeModelMappingFromSpecialSettings,
  resolveModelRedirectFromSpecialSettings,
  resolveCodexResponsesTransportRecords,
} from "../requestLogSpecialSettings";

describe("services/gateway/requestLogSpecialSettings", () => {
  it("reads ordered transport records without treating fallback as provider failover", () => {
    expect(
      resolveCodexResponsesTransportRecords(
        JSON.stringify([
          { type: "other", client_transport: "responses_ws" },
          {
            type: "codex_responses_transport",
            client_transport: "responses_ws",
            upstream_transport: "http",
            transport_action: "http_fallback",
            providerId: 7,
            failure_class: "transport",
            reason_code: "ws_handshake_failed",
            output_committed: false,
            recovery_from_trace_id: "trace-before-recovery",
          },
          {
            type: "codex_responses_transport",
            client_transport: "http",
            upstream_transport: "http",
            transport_action: "provider_switch",
            providerId: 9,
          },
        ])
      )
    ).toEqual([
      {
        terminal: null,
        handshakeStatus: null,
        eventStatus: null,
        clientTransport: "responses_ws",
        upstreamTransport: "http",
        transportAction: "http_fallback",
        failureClass: "transport",
        outputCommitted: false,
        recoveryFromTraceId: "trace-before-recovery",
        providerId: 7,
        reasonCode: "ws_handshake_failed",
      },
      {
        terminal: null,
        handshakeStatus: null,
        eventStatus: null,
        clientTransport: "http",
        upstreamTransport: "http",
        transportAction: "provider_switch",
        failureClass: null,
        outputCommitted: null,
        recoveryFromTraceId: null,
        providerId: 9,
        reasonCode: null,
      },
    ]);
  });

  it("keeps old logs empty and missing or invalid transport fields unknown", () => {
    for (const value of [null, undefined, "bad-json", JSON.stringify([{ type: "other" }])]) {
      expect(resolveCodexResponsesTransportRecords(value)).toEqual([]);
    }
    expect(
      resolveCodexResponsesTransportRecords(
        JSON.stringify({
          type: "codex_responses_transport",
          client_transport: "app_server",
          upstream_transport: {},
          providerId: "7",
          transport_action: false,
          failure_class: 2,
          output_committed: "false",
          recovery_from_trace_id: {},
          reason_code: [],
        })
      )
    ).toEqual([
      {
        terminal: null,
        handshakeStatus: null,
        eventStatus: null,
        clientTransport: null,
        upstreamTransport: null,
        transportAction: null,
        failureClass: null,
        outputCommitted: null,
        recoveryFromTraceId: null,
        providerId: null,
        reasonCode: null,
      },
    ]);
  });

  it.each(["completed", "incomplete", "failed", "unknown", null])(
    "keeps only known response terminals (%s)",
    (terminal) => {
      const [record] = resolveCodexResponsesTransportRecords(
        JSON.stringify({
          type: "codex_responses_transport",
          terminal,
        })
      );
      expect(record.terminal).toBe(terminal === "unknown" ? null : terminal);
    }
  );

  it("distinguishes a successful handshake from an error event status", () => {
    const [record] = resolveCodexResponsesTransportRecords(
      JSON.stringify([
        {
          type: "codex_responses_transport",
          status_source: "responses_event",
          handshake_status: 101,
          event_status: 429,
        },
      ])
    );
    expect(record.handshakeStatus).toBe(101);
    expect(record.eventStatus).toBe(429);
  });

  it("resolves Claude model mapping with final provider preference", () => {
    const settings = JSON.stringify([
      { type: "noop" },
      {
        type: "claude_model_mapping",
        requestedModel: " claude-sonnet ",
        effectiveModel: " gpt-4.1 ",
        mappingKind: " sonnet ",
        providerId: 1,
        providerName: " Provider A ",
        applied: true,
      },
      {
        type: "claude_model_mapping",
        requestedModel: " claude-sonnet ",
        effectiveModel: " gpt-5.4 ",
        mappingKind: " sonnet ",
        providerId: 2,
        providerName: " Provider B ",
        applied: true,
      },
    ]);

    expect(resolveClaudeModelMappingFromSpecialSettings(settings, 2)).toEqual({
      requestedModel: "claude-sonnet",
      effectiveModel: "gpt-5.4",
      mappingKind: "sonnet",
      providerId: 2,
      providerName: "Provider B",
      applied: true,
    });
    expect(resolveClaudeModelMappingFromSpecialSettings(settings, 99)?.providerId).toBe(2);
    expect(resolveModelRedirectFromSpecialSettings(settings, 2)).toEqual({
      stage: "legacy",
      providerId: 2,
      providerName: "Provider B",
      sourceModel: "claude-sonnet",
      targetModel: "gpt-5.4",
    });
  });

  it("resolves generic model redirect with final provider preference", () => {
    const settings = JSON.stringify([
      {
        type: "model_redirect",
        stage: "provider",
        providerId: 1,
        providerName: "Provider A",
        sourceModel: "gpt-original",
        targetModel: "model-a",
      },
      {
        type: "model_redirect",
        stage: "provider",
        providerId: 2,
        providerName: "Provider B",
        sourceModel: "gpt-original",
        targetModel: "model-b",
      },
    ]);

    expect(resolveModelRedirectFromSpecialSettings(settings, 2)).toEqual({
      stage: "provider",
      providerId: 2,
      providerName: "Provider B",
      sourceModel: "gpt-original",
      targetModel: "model-b",
    });
  });

  it("ignores invalid, unapplied, and identity mappings", () => {
    expect(resolveClaudeModelMappingFromSpecialSettings(null)).toBeNull();
    expect(resolveClaudeModelMappingFromSpecialSettings("bad-json")).toBeNull();
    expect(
      resolveClaudeModelMappingFromSpecialSettings(
        JSON.stringify([
          {
            type: "claude_model_mapping",
            requestedModel: "same",
            effectiveModel: "same",
            mappingKind: "sonnet",
            providerId: 1,
            providerName: "Provider A",
            applied: true,
          },
          {
            type: "claude_model_mapping",
            requestedModel: "claude-sonnet",
            effectiveModel: "gpt-5.4",
            mappingKind: "sonnet",
            providerId: 2,
            providerName: "Provider B",
            applied: false,
          },
        ])
      )
    ).toBeNull();
  });

  it("identifies only the structured Codex system request marker", () => {
    expect(
      hasCodexSystemRequestSpecialSetting(
        JSON.stringify([{ type: "noop" }, { type: "codex_system_request", threadSource: "system" }])
      )
    ).toBe(true);
    expect(
      hasCodexSystemRequestSpecialSetting(
        JSON.stringify({ type: "codex_system_request", threadSource: "system" })
      )
    ).toBe(true);
  });

  it("rejects incomplete or mismatched Codex system request markers", () => {
    for (const settings of [
      [{ type: "codex_system_request" }],
      [{ type: "codex_system_request", threadSource: "user" }],
      [{ type: "other", threadSource: "system" }],
      [{ type: "codex_system_request", threadSource: true }],
    ]) {
      expect(hasCodexSystemRequestSpecialSetting(JSON.stringify(settings))).toBe(false);
    }
  });

  it("fails closed for missing or malformed special settings", () => {
    expect(hasCodexSystemRequestSpecialSetting(null)).toBe(false);
    expect(hasCodexSystemRequestSpecialSetting("bad-json")).toBe(false);
    expect(hasCodexSystemRequestSpecialSetting(JSON.stringify([null, false, "marker"]))).toBe(
      false
    );
  });
});
