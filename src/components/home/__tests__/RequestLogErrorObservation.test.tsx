import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { GatewayErrorCodes } from "../../../constants/gatewayErrorCodes";
import { createRequestLogDetail } from "../../../services/gateway/requestLogFixtures";
import { RequestLogErrorObservationCard } from "../RequestLogErrorObservationCard";
import {
  resolveRequestLogErrorObservation,
  type RequestLogErrorObservation,
} from "../requestLogErrorDetails";

function createObservation(
  overrides: Partial<RequestLogErrorObservation> = {}
): RequestLogErrorObservation {
  return {
    attemptDurationMs: null,
    attemptFailureSummary: null,
    circuitFailureCount: null,
    circuitFailureThreshold: null,
    circuitStateAfter: null,
    circuitStateBefore: null,
    decision: null,
    displayErrorCode: null,
    errorCategory: null,
    gatewayErrorCode: null,
    gwDescription: null,
    matchedRule: null,
    outcome: null,
    providerId: null,
    providerIndex: null,
    providerName: null,
    rawDetailsText: null,
    reason: null,
    reasonCode: null,
    retryIndex: null,
    selectionMethod: null,
    source: "summary",
    upstreamBodyPreview: null,
    upstreamStatus: null,
    ...overrides,
  };
}

describe("components/home/RequestLogErrorObservation", () => {
  it("returns null for a missing observation", () => {
    const { container } = render(<RequestLogErrorObservationCard observation={null} />);

    expect(container).toBeEmptyDOMElement();
  });

  it("renders rich observation details and truncates upstream body preview", () => {
    const preview = "u".repeat(520);
    render(
      <RequestLogErrorObservationCard
        observation={createObservation({
          circuitFailureCount: 3,
          circuitFailureThreshold: 5,
          circuitStateAfter: "OPEN",
          circuitStateBefore: "CLOSED",
          decision: "skip",
          displayErrorCode: GatewayErrorCodes.UPSTREAM_TIMEOUT,
          errorCategory: "upstream",
          gatewayErrorCode: GatewayErrorCodes.UPSTREAM_TIMEOUT,
          gwDescription: {
            desc: "上游服务响应超时",
            suggestion: "检查 Provider 服务状态。",
          },
          matchedRule: "timeout-rule",
          providerId: 42,
          providerName: "Provider A",
          reason: "rule=timeout-rule, upstream_body=timeout body",
          reasonCode: "TIMEOUT",
          selectionMethod: "weighted",
          source: "error_details_json",
          upstreamBodyPreview: preview,
          upstreamStatus: 504,
        })}
      />
    );

    expect(
      screen.getByText(`上游超时 (${GatewayErrorCodes.UPSTREAM_TIMEOUT})`)
    ).toBeInTheDocument();
    expect(screen.getByText("上游服务响应超时")).toBeInTheDocument();
    expect(screen.getByText("检查 Provider 服务状态。")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "详细信息" }));

    expect(screen.getByText("错误分类:")).toBeInTheDocument();
    expect(screen.getByText("上游状态码:")).toBeInTheDocument();
    expect(screen.getByText("决策:")).toBeInTheDocument();
    expect(screen.getByText("选择方式:")).toBeInTheDocument();
    expect(screen.getByText("原因码:")).toBeInTheDocument();
    expect(screen.getByText("匹配规则:")).toBeInTheDocument();
    expect(screen.getByText("原因:")).toBeInTheDocument();
    expect(screen.getByText("熔断器:")).toBeInTheDocument();
    expect(screen.getByText("CLOSED → OPEN (3/5)")).toBeInTheDocument();
    expect(screen.getByText("供应商:")).toBeInTheDocument();
    expect(screen.getByText("Provider A (id=42)")).toBeInTheDocument();
    expect(screen.getByText("上游响应预览")).toBeInTheDocument();
    expect(screen.getByText(`${"u".repeat(500)}…`)).toBeInTheDocument();
    expect(screen.queryByText("原始错误信息")).not.toBeInTheDocument();
  });

  it("renders raw details and primary reason when no gateway description exists", () => {
    const rawDetails = "r".repeat(520);
    render(
      <RequestLogErrorObservationCard
        observation={createObservation({
          circuitFailureCount: 2,
          circuitFailureThreshold: 4,
          circuitStateBefore: "HALF_OPEN",
          displayErrorCode: "CUSTOM_CODE",
          providerId: 99,
          rawDetailsText: rawDetails,
          reason: "plain upstream failure",
        })}
      />
    );

    expect(screen.getByText("CUSTOM_CODE")).toBeInTheDocument();
    expect(screen.getByText("plain upstream failure")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "详细信息" }));

    expect(screen.getByText("HALF_OPEN (2/4)")).toBeInTheDocument();
    expect(screen.getByText("id=99")).toBeInTheDocument();
    expect(screen.getByText("原始错误信息")).toBeInTheDocument();
    expect(screen.getByText(`${"r".repeat(500)}…`)).toBeInTheDocument();
  });

  it("omits the disclosure when there are no secondary details", () => {
    render(
      <RequestLogErrorObservationCard
        observation={createObservation({
          reason: "plain upstream failure",
        })}
      />
    );

    expect(screen.getByText("plain upstream failure")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "详细信息" })).not.toBeInTheDocument();
  });

  it("resolves null, clean success, and summary-only failures", () => {
    expect(resolveRequestLogErrorObservation(null)).toBeNull();
    expect(resolveRequestLogErrorObservation(undefined)).toBeNull();
    expect(resolveRequestLogErrorObservation(createRequestLogDetail({ status: 200 }))).toBeNull();

    expect(
      resolveRequestLogErrorObservation(
        createRequestLogDetail({
          error_code: GatewayErrorCodes.PROVIDER_CIRCUIT_OPEN,
          error_details_json: null,
          status: 503,
        })
      )
    ).toMatchObject({
      displayErrorCode: GatewayErrorCodes.PROVIDER_CIRCUIT_OPEN,
      gatewayErrorCode: GatewayErrorCodes.PROVIDER_CIRCUIT_OPEN,
      source: "summary",
      upstreamStatus: 503,
    });

    expect(
      resolveRequestLogErrorObservation(
        createRequestLogDetail({
          error_code: null,
          error_details_json: null,
          status: 502,
        })
      )
    ).toMatchObject({
      displayErrorCode: null,
      gatewayErrorCode: null,
      source: "summary",
      upstreamStatus: 502,
    });
  });

  it("parses structured error details and reason fallbacks", () => {
    const observation = resolveRequestLogErrorObservation(
      createRequestLogDetail({
        error_code: GatewayErrorCodes.UPSTREAM_ALL_FAILED,
        error_details_json: JSON.stringify({
          attempt_duration_ms: 321,
          circuit_failure_count: 4,
          circuit_failure_threshold: 5,
          circuit_state_after: "OPEN",
          circuit_state_before: "CLOSED",
          decision: "failover",
          error_category: "provider",
          error_code: GatewayErrorCodes.UPSTREAM_5XX,
          matched_rule: "",
          outcome: "failed",
          provider_id: 8,
          provider_index: 2,
          provider_name: " Provider B ",
          reason: 'rule=from-reason, upstream_body={"error":"bad"}',
          reason_code: "UPSTREAM",
          request_error_code: GatewayErrorCodes.UPSTREAM_TIMEOUT,
          retry_index: 1,
          selection_method: "round_robin",
          upstream_body_preview: "",
          upstream_status: 500,
        }),
        status: 500,
      })
    );

    expect(observation).toMatchObject({
      attemptDurationMs: 321,
      circuitFailureCount: 4,
      circuitFailureThreshold: 5,
      circuitStateAfter: "OPEN",
      circuitStateBefore: "CLOSED",
      decision: "failover",
      displayErrorCode: GatewayErrorCodes.UPSTREAM_5XX,
      errorCategory: "provider",
      gatewayErrorCode: GatewayErrorCodes.UPSTREAM_TIMEOUT,
      matchedRule: "from-reason",
      outcome: "failed",
      providerId: 8,
      providerIndex: 2,
      providerName: "Provider B",
      reason: 'rule=from-reason, upstream_body={"error":"bad"}',
      reasonCode: "UPSTREAM",
      retryIndex: 1,
      selectionMethod: "round_robin",
      source: "error_details_json",
      upstreamBodyPreview: '{"error":"bad"}',
      upstreamStatus: 500,
    });
  });

  it("keeps raw details text for malformed, primitive, array, and empty object payloads", () => {
    const malformed = resolveRequestLogErrorObservation(
      createRequestLogDetail({ error_details_json: "not-json", status: 200 })
    );
    expect(malformed).toMatchObject({
      rawDetailsText: "not-json",
      source: "error_details_json",
    });

    const primitive = resolveRequestLogErrorObservation(
      createRequestLogDetail({ error_details_json: '"plain"', status: 200 })
    );
    expect(primitive).toMatchObject({
      rawDetailsText: '"plain"',
      source: "error_details_json",
    });

    const arrayPayload = resolveRequestLogErrorObservation(
      createRequestLogDetail({ error_details_json: "[]", status: 200 })
    );
    expect(arrayPayload).toMatchObject({
      rawDetailsText: "[]",
      source: "error_details_json",
    });

    const emptyObject = resolveRequestLogErrorObservation(
      createRequestLogDetail({ error_details_json: "{}", status: 200 })
    );
    expect(emptyObject).toMatchObject({
      rawDetailsText: "{}",
      source: "error_details_json",
    });
  });

  it("renders resolved observations through the card", () => {
    const observation = resolveRequestLogErrorObservation(
      createRequestLogDetail({
        error_details_json: JSON.stringify({
          error_code: GatewayErrorCodes.REQUEST_ABORTED,
          reason: "client closed the stream",
          upstream_status: 499,
        }),
        status: 499,
      })
    );

    render(<RequestLogErrorObservationCard observation={observation} />);

    expect(screen.getByText(/请求中断/)).toBeInTheDocument();
    expect(screen.getByText("请求被中断")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "详细信息" }));
    const detailSection = screen.getByText("上游状态码:").parentElement as HTMLElement;
    expect(within(detailSection).getByText("499")).toBeInTheDocument();
  });
});
