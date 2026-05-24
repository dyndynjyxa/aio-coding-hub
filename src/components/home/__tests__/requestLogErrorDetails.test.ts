import { describe, expect, it } from "vitest";
import { createRequestLogDetail } from "../../../services/gateway/requestLogFixtures";
import { resolveRequestLogErrorObservation } from "../requestLogErrorDetails";

describe("components/home/requestLogErrorDetails", () => {
  it("returns null when neither details nor summary contain error signal", () => {
    expect(resolveRequestLogErrorObservation(null)).toBeNull();
    expect(resolveRequestLogErrorObservation(undefined)).toBeNull();
    expect(resolveRequestLogErrorObservation(createRequestLogDetail())).toBeNull();
  });

  it("parses structured error details and reason-derived fields", () => {
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
          error_category: "upstream",
          error_code: "GW_PROVIDER_CIRCUIT_OPEN",
          outcome: "failure",
          provider_id: 42,
          provider_index: 1,
          provider_name: "Provider B",
          reason: "rule=provider_circuit, upstream_body={\"error\":\"bad\"}",
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
        errorCategory: "upstream",
        gatewayErrorCode: "GW_UPSTREAM_5XX",
        matchedRule: "provider_circuit",
        outcome: "failure",
        providerId: 42,
        providerIndex: 1,
        providerName: "Provider B",
        reason: "rule=provider_circuit, upstream_body={\"error\":\"bad\"}",
        reasonCode: "circuit_open",
        retryIndex: 2,
        selectionMethod: "sort_mode",
        source: "error_details_json",
        upstreamBodyPreview: "{\"error\":\"bad\"}",
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
});
