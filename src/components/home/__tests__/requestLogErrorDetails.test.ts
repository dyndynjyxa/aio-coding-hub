import { describe, expect, it } from "vitest";
import { GatewayErrorCodes } from "../../../constants/gatewayErrorCodes";
import type { AttemptJsonEntry } from "../../../services/gateway/attemptsJson";
import { createRequestLogDetail } from "../../../services/gateway/requestLogFixtures";
import {
  buildAttemptFailureSummary,
  resolveRequestLogErrorObservation,
} from "../requestLogErrorDetails";

function createAttempt(overrides: Partial<AttemptJsonEntry> = {}): AttemptJsonEntry {
  return {
    provider_id: 1,
    provider_name: "Provider A",
    base_url: "https://example.com",
    outcome: "success",
    status: null,
    ...overrides,
  };
}

function timeoutAttempt(overrides: Partial<AttemptJsonEntry> = {}): AttemptJsonEntry {
  return createAttempt({
    outcome: "request_timeout: category=SYSTEM_ERROR code=GW_UPSTREAM_TIMEOUT decision=switch",
    error_code: GatewayErrorCodes.UPSTREAM_TIMEOUT,
    timeout_secs: 30,
    ...overrides,
  });
}

describe("components/home/requestLogErrorDetails", () => {
  it("returns null when neither details nor summary contain error signal", () => {
    expect(resolveRequestLogErrorObservation(null)).toBeNull();
    expect(resolveRequestLogErrorObservation(undefined)).toBeNull();
    expect(resolveRequestLogErrorObservation(createRequestLogDetail())).toBeNull();
  });

  it("uses the audit marker for full-input retry and does not invent an upstream 400", () => {
    const log = createRequestLogDetail({
      cli_key: "codex",
      status: 400,
      error_code: GatewayErrorCodes.REQUEST_REJECTED,
      error_details_json: JSON.stringify({
        error_category: "context",
        reason_code: "previous_response_not_found",
        reason: "Response context is unavailable; resend the full input",
      }),
      special_settings_json: JSON.stringify([
        {
          type: "codex_responses_transport",
          scope: "attempt",
          transport_action: "full_input_retry",
          reason_code: "previous_response_not_found",
          output_committed: false,
        },
      ]),
    });

    expect(resolveRequestLogErrorObservation(log)).toMatchObject({
      isFullInputRetry: true,
      displayErrorCode: GatewayErrorCodes.REQUEST_REJECTED,
      upstreamStatus: null,
      reasonCode: "previous_response_not_found",
      gwDescription: {
        desc: "网关已要求客户端重发完整输入",
        suggestion: "支持自动恢复的客户端会重发完整输入；恢复结果请查看后续请求。",
      },
    });
    for (const specialSettings of [
      null,
      [{ type: "codex_responses_transport", reason_code: "previous_response_not_found" }],
      [{ type: "codex_responses_transport", transport_action: "full_input_retry" }],
    ]) {
      expect(
        resolveRequestLogErrorObservation({
          ...log,
          special_settings_json: specialSettings == null ? null : JSON.stringify(specialSettings),
        })
      ).toMatchObject({
        isFullInputRetry: false,
        gwDescription: { desc: "请求或上下文恢复校验未通过" },
      });
    }
  });

  it.each(["upstream", "local"])("parses explicit error details (%s)", (errorCategory) => {
    const observation = resolveRequestLogErrorObservation(
      createRequestLogDetail({
        status: 502,
        error_code: "GW_UPSTREAM_5XX",
        error_details_json: JSON.stringify({
          attempt_duration_ms: 321,
          circuit_failure_count: 2,
          circuit_failure_threshold: 5,
          circuit_state_after: "OPEN",
          circuit_state_before: "CLOSED",
          decision: "skip",
          error_category: errorCategory,
          error_code: "GW_PROVIDER_CIRCUIT_OPEN",
          outcome: "failure",
          provider_id: 42,
          provider_index: 1,
          provider_name: "Provider B",
          reason: 'rule=provider_circuit, upstream_body={"error":"bad"}',
          reason_code: "circuit_open",
          retry_index: 2,
          selection_method: "sort_mode",
          upstream_status: 503,
        }),
      })
    );

    expect(observation).toEqual(
      expect.objectContaining({
        attemptDurationMs: 321,
        circuitFailureCount: 2,
        circuitFailureThreshold: 5,
        circuitStateAfter: "OPEN",
        circuitStateBefore: "CLOSED",
        decision: "skip",
        displayErrorCode: "GW_PROVIDER_CIRCUIT_OPEN",
        errorCategory,
        gatewayErrorCode: "GW_UPSTREAM_5XX",
        matchedRule: "provider_circuit",
        outcome: "failure",
        providerId: 42,
        providerIndex: 1,
        providerName: "Provider B",
        reason: 'rule=provider_circuit, upstream_body={"error":"bad"}',
        reasonCode: "circuit_open",
        retryIndex: 2,
        selectionMethod: "sort_mode",
        source: "error_details_json",
        upstreamBodyPreview: '{"error":"bad"}',
        upstreamStatus: 503,
      })
    );
  });

  it("keeps raw details for invalid or empty JSON and falls back to summary", () => {
    expect(
      resolveRequestLogErrorObservation(
        createRequestLogDetail({
          error_details_json: "not json",
        })
      )
    ).toEqual(
      expect.objectContaining({
        rawDetailsText: "not json",
        source: "error_details_json",
      })
    );

    expect(
      resolveRequestLogErrorObservation(
        createRequestLogDetail({
          error_details_json: "{}",
        })
      )
    ).toEqual(
      expect.objectContaining({
        rawDetailsText: "{}",
        source: "error_details_json",
      })
    );

    expect(
      resolveRequestLogErrorObservation(
        createRequestLogDetail({
          status: 429,
          error_code: "GW_PROVIDER_RATE_LIMITED",
          error_details_json: null,
        })
      )
    ).toEqual(
      expect.objectContaining({
        displayErrorCode: "GW_PROVIDER_RATE_LIMITED",
        gatewayErrorCode: "GW_PROVIDER_RATE_LIMITED",
        source: "summary",
        upstreamStatus: 429,
      })
    );
  });

  describe("buildAttemptFailureSummary", () => {
    it("groups timeout attempts with count, deduped providers, and max timeout secs (AC1)", () => {
      const summary = buildAttemptFailureSummary([
        timeoutAttempt({ provider_id: 1, provider_name: "Provider A" }),
        timeoutAttempt({ provider_id: 1, provider_name: "Provider A", timeout_secs: 300 }),
        timeoutAttempt({ provider_id: 2, provider_name: "Provider B" }),
      ]);

      expect(summary).toEqual([
        {
          errorCode: GatewayErrorCodes.UPSTREAM_TIMEOUT,
          count: 3,
          providerNames: ["Provider A", "Provider B"],
          timeoutSecs: 300,
          circuitTriggerErrorCode: null,
          circuitRecoverAtUnix: null,
        },
      ]);
    });

    it("groups mixed failure codes sorted by count descending (AC2)", () => {
      const summary = buildAttemptFailureSummary([
        timeoutAttempt(),
        createAttempt({ error_code: GatewayErrorCodes.UPSTREAM_4XX, status: 401 }),
        createAttempt({ error_code: GatewayErrorCodes.UPSTREAM_4XX, status: 401 }),
      ]);

      expect(summary).toEqual([
        {
          errorCode: GatewayErrorCodes.UPSTREAM_4XX,
          count: 2,
          providerNames: ["Provider A"],
          timeoutSecs: null,
          circuitTriggerErrorCode: null,
          circuitRecoverAtUnix: null,
        },
        {
          errorCode: GatewayErrorCodes.UPSTREAM_TIMEOUT,
          count: 1,
          providerNames: ["Provider A"],
          timeoutSecs: 30,
          circuitTriggerErrorCode: null,
          circuitRecoverAtUnix: null,
        },
      ]);
    });

    it("returns null for null, empty, and all-success attempts (AC3)", () => {
      expect(buildAttemptFailureSummary(null)).toBeNull();
      expect(buildAttemptFailureSummary([])).toBeNull();
      expect(
        buildAttemptFailureSummary([createAttempt(), createAttempt({ provider_id: 2 })])
      ).toBeNull();
    });

    it("omits timeout secs for legacy timeout attempts without the structured field (AC3)", () => {
      const summary = buildAttemptFailureSummary([timeoutAttempt({ timeout_secs: undefined })]);

      expect(summary).toEqual([
        {
          errorCode: GatewayErrorCodes.UPSTREAM_TIMEOUT,
          count: 1,
          providerNames: ["Provider A"],
          timeoutSecs: null,
          circuitTriggerErrorCode: null,
          circuitRecoverAtUnix: null,
        },
      ]);
    });

    it("never assigns timeout secs to non-timeout groups", () => {
      const summary = buildAttemptFailureSummary([
        createAttempt({ error_code: GatewayErrorCodes.STREAM_ERROR, timeout_secs: 30 }),
      ]);

      expect(summary).toEqual([
        {
          errorCode: GatewayErrorCodes.STREAM_ERROR,
          count: 1,
          providerNames: ["Provider A"],
          timeoutSecs: null,
          circuitTriggerErrorCode: null,
          circuitRecoverAtUnix: null,
        },
      ]);
    });

    it("carries circuit trigger code and latest recovery point for gate-skip attempts", () => {
      const summary = buildAttemptFailureSummary([
        createAttempt({
          outcome: "skipped",
          error_code: GatewayErrorCodes.PROVIDER_CIRCUIT_OPEN,
          circuit_trigger_error_code: GatewayErrorCodes.UPSTREAM_TIMEOUT,
          circuit_recover_at_unix: 1_750_001_800,
        }),
        createAttempt({
          provider_id: 2,
          provider_name: "Provider B",
          outcome: "skipped",
          error_code: GatewayErrorCodes.PROVIDER_CIRCUIT_OPEN,
          circuit_trigger_error_code: GatewayErrorCodes.UPSTREAM_TIMEOUT,
          circuit_recover_at_unix: 1_750_002_000,
        }),
      ]);

      expect(summary).toEqual([
        {
          errorCode: GatewayErrorCodes.PROVIDER_CIRCUIT_OPEN,
          count: 2,
          providerNames: ["Provider A", "Provider B"],
          timeoutSecs: null,
          circuitTriggerErrorCode: GatewayErrorCodes.UPSTREAM_TIMEOUT,
          circuitRecoverAtUnix: 1_750_002_000,
        },
      ]);
    });

    it.each([GatewayErrorCodes.UPSTREAM_4XX, undefined])(
      "does not assign one provider's trigger to a mixed or unknown group (%s)",
      (otherCause) => {
        const summary = buildAttemptFailureSummary([
          createAttempt({
            error_code: GatewayErrorCodes.PROVIDER_CIRCUIT_OPEN,
            circuit_trigger_error_code: GatewayErrorCodes.UPSTREAM_TIMEOUT,
          }),
          createAttempt({
            provider_id: 2,
            error_code: GatewayErrorCodes.PROVIDER_CIRCUIT_OPEN,
            circuit_trigger_error_code: otherCause,
          }),
          createAttempt({
            provider_id: 3,
            error_code: GatewayErrorCodes.PROVIDER_CIRCUIT_OPEN,
            circuit_trigger_error_code: GatewayErrorCodes.UPSTREAM_TIMEOUT,
          }),
        ]);
        expect(summary?.[0]?.circuitTriggerErrorCode).toBeNull();
      }
    );

    it("degrades to null attribution when the new fields are absent (legacy logs)", () => {
      const summary = buildAttemptFailureSummary([
        createAttempt({
          outcome: "skipped",
          error_code: GatewayErrorCodes.PROVIDER_CIRCUIT_OPEN,
        }),
      ]);

      expect(summary).toEqual([
        {
          errorCode: GatewayErrorCodes.PROVIDER_CIRCUIT_OPEN,
          count: 1,
          providerNames: ["Provider A"],
          timeoutSecs: null,
          circuitTriggerErrorCode: null,
          circuitRecoverAtUnix: null,
        },
      ]);
    });
  });

  it("resolves attempt failure summary from attempts_json with an aborted terminal code (AC1)", () => {
    const observation = resolveRequestLogErrorObservation(
      createRequestLogDetail({
        status: 499,
        error_code: "GW_REQUEST_ABORTED",
        attempts_json: JSON.stringify([timeoutAttempt(), timeoutAttempt(), timeoutAttempt()]),
      })
    );

    expect(observation).toEqual(
      expect.objectContaining({
        displayErrorCode: "GW_REQUEST_ABORTED",
        attemptFailureSummary: [
          {
            errorCode: GatewayErrorCodes.UPSTREAM_TIMEOUT,
            count: 3,
            providerNames: ["Provider A"],
            timeoutSecs: 30,
            circuitTriggerErrorCode: null,
            circuitRecoverAtUnix: null,
          },
        ],
      })
    );
  });

  it("keeps a null attempt failure summary for invalid attempts_json without throwing (AC3)", () => {
    const observation = resolveRequestLogErrorObservation(
      createRequestLogDetail({
        status: 502,
        error_code: "GW_UPSTREAM_5XX",
        attempts_json: "not json",
      })
    );

    expect(observation).toEqual(
      expect.objectContaining({
        displayErrorCode: "GW_UPSTREAM_5XX",
        attemptFailureSummary: null,
      })
    );
  });
});
