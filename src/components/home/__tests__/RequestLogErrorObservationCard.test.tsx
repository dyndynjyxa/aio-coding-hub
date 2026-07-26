import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { GatewayErrorCodes, GatewayErrorDescriptions } from "../../../constants/gatewayErrorCodes";
import { RequestLogErrorObservationCard } from "../RequestLogErrorObservationCard";
import type { RequestLogErrorObservation } from "../requestLogErrorDetails";

const baseObservation: RequestLogErrorObservation = {
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
};

describe("components/home/RequestLogErrorObservationCard", () => {
  it("renders nothing without an observation", () => {
    const { container } = render(<RequestLogErrorObservationCard observation={null} />);
    expect(container).toBeEmptyDOMElement();
  });

  it("renders description, suggestion, detail fields, and truncated upstream preview", () => {
    render(
      <RequestLogErrorObservationCard
        observation={{
          ...baseObservation,
          circuitFailureCount: 3,
          circuitFailureThreshold: 5,
          circuitStateAfter: "OPEN",
          circuitStateBefore: "CLOSED",
          decision: "skip",
          displayErrorCode: "GW_PROVIDER_CIRCUIT_OPEN",
          errorCategory: "upstream",
          gatewayErrorCode: "GW_PROVIDER_CIRCUIT_OPEN",
          gwDescription: GatewayErrorDescriptions.GW_PROVIDER_CIRCUIT_OPEN,
          matchedRule: "provider_circuit",
          providerId: 9,
          providerName: "Provider B",
          reason: "熔断器打开",
          reasonCode: "circuit_open",
          selectionMethod: "sort_mode",
          upstreamBodyPreview: "x".repeat(501),
          upstreamStatus: 503,
        }}
      />
    );

    expect(screen.getByText("供应商熔断 (GW_PROVIDER_CIRCUIT_OPEN)")).toBeInTheDocument();
    expect(screen.getByText("Provider 已熔断")).toBeInTheDocument();
    expect(screen.getByText(/该 Provider 因连续失败已被熔断/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "详细信息" }));

    expect(screen.getByText("错误分类:")).toBeInTheDocument();
    expect(screen.getByText("上游状态码:")).toBeInTheDocument();
    expect(screen.getByText("决策:")).toBeInTheDocument();
    expect(screen.getByText("选择方式:")).toBeInTheDocument();
    expect(screen.getByText("原因码:")).toBeInTheDocument();
    expect(screen.getByText("匹配规则:")).toBeInTheDocument();
    expect(screen.getByText("原因:")).toBeInTheDocument();
    expect(screen.getByText("熔断器:")).toBeInTheDocument();
    expect(screen.getByText("供应商:")).toBeInTheDocument();
    expect(screen.getByText(/CLOSED → OPEN \(3\/5\)/)).toBeInTheDocument();
    expect(screen.getByText("Provider B (id=9)")).toBeInTheDocument();
    expect(screen.getByText(/x{500}…/)).toBeInTheDocument();
  });

  it("renders reason as primary text and raw details when no gateway description exists", () => {
    render(
      <RequestLogErrorObservationCard
        observation={{
          ...baseObservation,
          circuitStateAfter: "OPEN",
          displayErrorCode: "CUSTOM_ERROR",
          providerId: 12,
          rawDetailsText: "raw error details",
          reason: "custom reason",
        }}
      />
    );

    expect(screen.getByText("CUSTOM_ERROR")).toBeInTheDocument();
    expect(screen.getByText("custom reason")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "详细信息" }));

    expect(screen.getByText("→ OPEN")).toBeInTheDocument();
    expect(screen.getByText("id=12")).toBeInTheDocument();
    expect(screen.getByText("原始错误信息")).toBeInTheDocument();
    expect(screen.getByText("raw error details")).toBeInTheDocument();
  });

  it("renders the failure attempt summary with timeout secs and dominant-code suggestion (AC1)", () => {
    render(
      <RequestLogErrorObservationCard
        observation={{
          ...baseObservation,
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
          displayErrorCode: GatewayErrorCodes.REQUEST_ABORTED,
          gatewayErrorCode: GatewayErrorCodes.REQUEST_ABORTED,
          gwDescription: GatewayErrorDescriptions.GW_REQUEST_ABORTED,
        }}
      />
    );

    expect(screen.getByText("失败尝试")).toBeInTheDocument();
    expect(screen.getByText(/上游超时/)).toHaveTextContent("上游超时 ×3（Provider A，30 秒）");
    // Dominant failure code differs from the displayed terminal code, so its
    // suggestion (containing the first-byte timeout settings path) is appended.
    expect(screen.getByText(/首字节超时/)).toHaveTextContent(
      GatewayErrorDescriptions.GW_UPSTREAM_TIMEOUT.suggestion
    );
  });

  it("renders circuit trigger label and recovery hint for all-circuit-open requests", () => {
    const farFutureUnix = 4_102_444_800; // 2100-01-01, keeps "约 N 分钟后" stable
    render(
      <RequestLogErrorObservationCard
        observation={{
          ...baseObservation,
          attemptFailureSummary: [
            {
              errorCode: GatewayErrorCodes.PROVIDER_CIRCUIT_OPEN,
              count: 2,
              providerNames: ["Provider A", "Provider B"],
              timeoutSecs: null,
              circuitTriggerErrorCode: GatewayErrorCodes.UPSTREAM_TIMEOUT,
              circuitRecoverAtUnix: farFutureUnix,
            },
          ],
          displayErrorCode: GatewayErrorCodes.ALL_PROVIDERS_UNAVAILABLE,
          gatewayErrorCode: GatewayErrorCodes.ALL_PROVIDERS_UNAVAILABLE,
          gwDescription: GatewayErrorDescriptions.GW_ALL_PROVIDERS_UNAVAILABLE,
        }}
      />
    );

    const groupLine = screen.getByText(/供应商熔断 ×2/);
    expect(groupLine).toHaveTextContent("供应商熔断 ×2（Provider A、Provider B）");
    expect(groupLine).toHaveTextContent("触发：上游超时");
    expect(groupLine).toHaveTextContent(/约 \d+ 分钟后/);
  });

  it("omits trigger and recovery text when attribution is missing (restart degradation)", () => {
    render(
      <RequestLogErrorObservationCard
        observation={{
          ...baseObservation,
          attemptFailureSummary: [
            {
              errorCode: GatewayErrorCodes.PROVIDER_CIRCUIT_OPEN,
              count: 1,
              providerNames: ["Provider A"],
              timeoutSecs: null,
              circuitTriggerErrorCode: null,
              circuitRecoverAtUnix: null,
            },
          ],
          displayErrorCode: GatewayErrorCodes.ALL_PROVIDERS_UNAVAILABLE,
          gatewayErrorCode: GatewayErrorCodes.ALL_PROVIDERS_UNAVAILABLE,
          gwDescription: GatewayErrorDescriptions.GW_ALL_PROVIDERS_UNAVAILABLE,
        }}
      />
    );

    const groupLine = screen.getByText(/供应商熔断 ×1/);
    expect(groupLine).toHaveTextContent("供应商熔断 ×1（Provider A）");
    expect(groupLine).not.toHaveTextContent("触发：");
    expect(groupLine).not.toHaveTextContent("恢复");
  });

  it("renders every failure group and the dominant 4xx suggestion for mixed storms (AC2)", () => {
    render(
      <RequestLogErrorObservationCard
        observation={{
          ...baseObservation,
          attemptFailureSummary: [
            {
              errorCode: GatewayErrorCodes.UPSTREAM_4XX,
              count: 2,
              providerNames: ["Provider A", "Provider B"],
              timeoutSecs: null,
              circuitTriggerErrorCode: null,
              circuitRecoverAtUnix: null,
            },
            {
              errorCode: GatewayErrorCodes.UPSTREAM_TIMEOUT,
              count: 1,
              providerNames: ["Provider A"],
              timeoutSecs: null,
              circuitTriggerErrorCode: null,
              circuitRecoverAtUnix: null,
            },
          ],
          displayErrorCode: GatewayErrorCodes.UPSTREAM_ALL_FAILED,
          gatewayErrorCode: GatewayErrorCodes.UPSTREAM_ALL_FAILED,
          gwDescription: GatewayErrorDescriptions.GW_UPSTREAM_ALL_FAILED,
        }}
      />
    );

    expect(screen.getByText(/上游4XX/)).toHaveTextContent("上游4XX ×2（Provider A、Provider B）");
    expect(screen.getByText(/上游超时/)).toHaveTextContent("上游超时 ×1（Provider A）");
    expect(
      screen.getByText(GatewayErrorDescriptions.GW_UPSTREAM_4XX.suggestion)
    ).toBeInTheDocument();
  });

  it("omits the summary section without failure attempts and skips a duplicate suggestion (AC3)", () => {
    const { unmount } = render(
      <RequestLogErrorObservationCard
        observation={{
          ...baseObservation,
          attemptFailureSummary: null,
          displayErrorCode: GatewayErrorCodes.REQUEST_ABORTED,
          gwDescription: GatewayErrorDescriptions.GW_REQUEST_ABORTED,
        }}
      />
    );

    expect(screen.queryByText("失败尝试")).not.toBeInTheDocument();
    unmount();

    // Dominant code equals the displayed code: the suggestion must not repeat.
    render(
      <RequestLogErrorObservationCard
        observation={{
          ...baseObservation,
          attemptFailureSummary: [
            {
              errorCode: GatewayErrorCodes.UPSTREAM_TIMEOUT,
              count: 2,
              providerNames: ["Provider A"],
              timeoutSecs: null,
              circuitTriggerErrorCode: null,
              circuitRecoverAtUnix: null,
            },
          ],
          displayErrorCode: GatewayErrorCodes.UPSTREAM_TIMEOUT,
          gatewayErrorCode: GatewayErrorCodes.UPSTREAM_TIMEOUT,
          gwDescription: GatewayErrorDescriptions.GW_UPSTREAM_TIMEOUT,
        }}
      />
    );

    expect(screen.getAllByText(/首字节超时/)).toHaveLength(1);
    expect(screen.getByText(/×2/)).toHaveTextContent("上游超时 ×2（Provider A）");
  });

  it("renders a fallback title when only an upstream status is available", () => {
    render(
      <RequestLogErrorObservationCard
        observation={{
          ...baseObservation,
          upstreamStatus: 502,
        }}
      />
    );

    expect(screen.getByText("HTTP 502 响应异常")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "详细信息" }));

    expect(screen.getByText("上游状态码:")).toBeInTheDocument();
    expect(screen.getByText("502")).toBeInTheDocument();
  });
});
